use super::{
    editor::{Editor, TOOL_NAME},
    files,
    tests::{model, setup, shot},
};
use crate::database::Store;
use serde_json::{Value, json};
fn workspace() -> (Store, Editor) {
    let store = setup();
    let editor = {
        let db = store.db.lock().unwrap();
        files::save(&db, &store, "p", model("bag", "Bag"), 0).unwrap();
        let file = files::save(&db, &store, "p", shot("s1", "bag"), 0).unwrap();
        Editor::new(&file, &files::list(&db, "p").unwrap())
    };
    (store, editor)
}
fn edit(store: &Store, editor: &mut Editor, args: Value) -> anyhow::Result<String> {
    editor.execute(store, "p", TOOL_NAME, args)
}
#[test]
fn each_edit_is_persisted_and_later_errors_or_dropped_sessions_do_not_undo_it() {
    let (store, mut editor) = workspace();
    let before = editor.file.clone();
    edit(
        &store,
        &mut editor,
        json!({"command":"insert","path":"/scene.js","insert_line":0,"new_str":"// 调整动作"}),
    )
    .unwrap();
    let saved = files::read(&store.db.lock().unwrap(), "p", "s1")
        .unwrap()
        .unwrap();
    assert_eq!(saved.revision, before.revision + 1);
    assert_eq!(
        std::fs::read_to_string(files::path(&store, "p", &saved).unwrap()).unwrap(),
        saved.code
    );
    assert!(saved.code.starts_with("// 调整动作\n"));
    assert!(
        edit(
            &store,
            &mut editor,
            json!({"command":"create","path":"/scene.js","file_text":"overwrite"})
        )
        .is_err()
    );
    assert!(
        edit(
            &store,
            &mut editor,
            json!({"command":"create","path":"/scene.json","file_text":"not a protocol"})
        )
        .is_err()
    );
    drop(editor);
    assert_eq!(
        files::read(&store.db.lock().unwrap(), "p", "s1")
            .unwrap()
            .unwrap()
            .code,
        saved.code
    );
    let db = store.db.lock().unwrap();
    let count: i64 = db
        .query_row("SELECT count(*) FROM agent_messages", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0);
    drop(db);
    std::fs::remove_dir_all(&store.root).unwrap();
}
#[test]
fn a_new_model_is_one_js_file_with_host_owned_name_and_binding() {
    let (store, mut editor) = workspace();
    let code = "export function createModel({THREE}) { return new THREE.Group(); }";
    edit(
        &store,
        &mut editor,
        json!({"command":"create","path":"/models/午餐包提手.js","file_text":code}),
    )
    .unwrap();
    let all = files::list(&store.db.lock().unwrap(), "p").unwrap();
    let model = all.iter().find(|f| f.name == "午餐包提手").unwrap();
    assert_eq!(model.code, code);
    assert_eq!(editor.file.bindings["午餐包提手"], model.id);
    assert!(editor.file.asset_ids.contains(&model.id));
    let read = edit(
        &store,
        &mut editor,
        json!({"command":"view","path":"/models/午餐包提手.js"}),
    )
    .unwrap();
    assert!(read.contains(code));
    edit(&store, &mut editor, json!({"command":"str_replace","path":"/models/午餐包提手.js","old_str":"Group","new_str":"Mesh"})).unwrap();
    assert!(
        files::read(&store.db.lock().unwrap(), "p", &model.id)
            .unwrap()
            .unwrap()
            .code
            .contains("Mesh")
    );
    assert!(
        std::fs::read_dir(files::path(&store, "p", model).unwrap().parent().unwrap())
            .unwrap()
            .all(|entry| entry.unwrap().path().extension().unwrap() == "js")
    );
    std::fs::remove_dir_all(&store.root).unwrap();
}
#[test]
fn stale_edits_require_a_reread_and_preserve_internal_and_external_changes() {
    let (store, mut a) = workspace();
    let mut b = Editor::new(
        &a.file,
        &files::list(&store.db.lock().unwrap(), "p").unwrap(),
    );
    edit(
        &store,
        &mut a,
        json!({"command":"insert","path":"/scene.js","insert_line":0,"new_str":"// first"}),
    )
    .unwrap();
    let args = json!({"command":"insert","path":"/scene.js","insert_line":0,"new_str":"// second"});
    assert!(
        edit(&store, &mut b, args.clone())
            .unwrap_err()
            .to_string()
            .contains("view the file")
    );
    edit(&store, &mut b, json!({"command":"view","path":"/scene.js"})).unwrap();
    edit(&store, &mut b, args.clone()).unwrap();
    let path = files::path(&store, "p", &b.file).unwrap();
    std::fs::write(&path, format!("// external\n{}", b.file.code)).unwrap();
    assert!(edit(&store, &mut b, args.clone()).is_err());
    let read = edit(&store, &mut b, json!({"command":"view","path":"/scene.js"})).unwrap();
    assert!(read.contains("// external"));
    edit(&store, &mut b, args).unwrap();
    assert!(b.file.code.contains("// first") && b.file.code.contains("// external"));
    std::fs::remove_dir_all(&store.root).unwrap();
}
#[test]
fn dsh_null_placeholders_deletion_ranges_and_path_rules_are_preserved() {
    let (store, mut editor) = workspace();
    let input = json!({"command":"view","path":"/scene.js","view_range":null,"file_text":null,"old_str":null,"new_str":null,"insert_line":null});
    assert!(
        crate::assistant::harness::schema::issues(&super::editor_schema::schema(), &input)
            .is_empty()
    );
    assert!(
        edit(&store, &mut editor, input)
            .unwrap()
            .contains("total 1 lines")
    );
    let error = edit(
        &store,
        &mut editor,
        json!({"command":"view","path":"/scene.js","view_range":[1,500]}),
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("has 1 lines") && error.contains("-1"));
    for path in [
        "scene.js",
        "/../scene.js",
        "/models/../../x.js",
        "/etc/passwd",
        "/models/bag.js",
    ] {
        assert!(
            edit(
                &store,
                &mut editor,
                json!({"command":"insert","path":path,"insert_line":0,"new_str":"wrong"})
            )
            .is_err(),
            "{path}"
        );
    }
    assert!(
        edit(
            &store,
            &mut editor,
            json!({"command":"str_replace","path":"/scene.js","old_str":"update(t)","new_str":null})
        )
        .is_err()
    );
    edit(
        &store,
        &mut editor,
        json!({"command":"str_replace","path":"/scene.js","old_str":"update(t){}"}),
    )
    .unwrap();
    assert!(!editor.file.code.contains("update(t)"));
    std::fs::remove_dir_all(&store.root).unwrap();
}
#[test]
fn existing_empty_files_use_insert_and_deleted_files_can_be_created_again() {
    let (store, mut editor) = workspace();
    let code = editor.file.code.clone();
    edit(
        &store,
        &mut editor,
        json!({"command":"str_replace","path":"/scene.js","old_str":code,"new_str":""}),
    )
    .unwrap();
    assert!(
        edit(
            &store,
            &mut editor,
            json!({"command":"create","path":"/scene.js","file_text":"wrong"})
        )
        .is_err()
    );
    edit(
        &store,
        &mut editor,
        json!({"command":"insert","path":"/scene.js","insert_line":0,"new_str":"// new"}),
    )
    .unwrap();
    std::fs::remove_file(files::path(&store, "p", &editor.file).unwrap()).unwrap();
    assert!(
        edit(
            &store,
            &mut editor,
            json!({"command":"view","path":"/scene.js"})
        )
        .is_err()
    );
    edit(
        &store,
        &mut editor,
        json!({"command":"create","path":"/scene.js","file_text":"// recreated"}),
    )
    .unwrap();
    assert_eq!(editor.file.code, "// recreated");
    std::fs::remove_dir_all(&store.root).unwrap();
}
