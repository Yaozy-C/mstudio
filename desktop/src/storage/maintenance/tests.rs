use super::{run, scan};
use crate::{database::Store, projects::write_document};
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    time::{Duration, SystemTime},
};

pub(super) struct Fixture {
    pub(super) base: PathBuf,
    pub(super) root: PathBuf,
    pub(super) data: PathBuf,
    pub(super) store: Store,
}
impl Fixture {
    pub(super) fn new() -> Self {
        let base = std::env::temp_dir().join(format!("mstudio-gc-{}", uuid::Uuid::new_v4()));
        let data = base.join("app");
        let root = base.join("media");
        fs::create_dir_all(root.join("assets")).unwrap();
        let store = Store::open(data.clone()).unwrap();
        store.location.write().unwrap().directory = root.clone();
        store
            .db
            .lock()
            .unwrap()
            .execute(
                "UPDATE settings SET value=?1 WHERE key='file-storage'",
                [json!({"directory":root,"previous":[]}).to_string()],
            )
            .unwrap();
        write_document(
            &store,
            json!({"id":"p","name":"test","assets":[],"nodes":[],"clips":[]}),
            true,
        )
        .unwrap();
        Self {
            base,
            root,
            data,
            store,
        }
    }
    pub(super) fn file(&self, name: &str) -> PathBuf {
        let path = self.root.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, vec![7u8; 8192]).unwrap();
        fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_times(
                fs::FileTimes::new().set_modified(SystemTime::now() - Duration::from_secs(600)),
            )
            .unwrap();
        path
    }
    pub(super) fn asset(&self, id: &str, extras: Value) -> Value {
        let path = self.file(&format!("assets/{id}.png"));
        let mut value =
            json!({"id":id,"name":format!("{id}.png"),"path":path,"kind":"image","generated":true});
        value
            .as_object_mut()
            .unwrap()
            .extend(extras.as_object().unwrap().clone());
        self.store
            .db
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO assets VALUES(?1,?2)",
                rusqlite::params![id, value.to_string()],
            )
            .unwrap();
        value
    }
    pub(super) fn document(&self, assets: Vec<Value>, nodes: Value) {
        let db = self.store.db.lock().unwrap();
        db.execute(
            "UPDATE projects SET document=?1 WHERE id='p'",
            [
                json!({"id":"p","name":"test","assets":assets,"nodes":nodes,"clips":[]})
                    .to_string(),
            ],
        )
        .unwrap();
    }
    pub(super) fn scan(&self) -> scan::Plan {
        scan::build(&self.store.db.lock().unwrap(), &self.root, &self.data).unwrap()
    }
    pub(super) fn clean(&self) -> run::Outcome {
        let fingerprint = self.scan().report.fingerprint;
        run::clean(&self.store, &self.data, &fingerprint).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}
#[test]
fn cleanup_keeps_real_usage_and_removes_ownership_only_orphans_from_disk_and_db() {
    let f = Fixture::new();
    let live = f.asset("live", json!({}));
    let library = f.asset("library", json!({"inLibrary":true}));
    let imported = f.asset("imported", json!({"generated":false}));
    let removed = f.asset("removed", json!({"generated":false,"inLibrary":false}));
    let orphan = f.asset("orphan", json!({}));
    let global = f.asset("global", json!({}));
    f.document(
        vec![
            live.clone(),
            library.clone(),
            imported.clone(),
            removed,
            orphan.clone(),
        ],
        json!([{"id":"shot","references":[{"assetId":"live"}]}]),
    );
    f.store
        .db
        .lock()
        .unwrap()
        .execute("INSERT INTO project_assets VALUES('p','orphan')", [])
        .unwrap();
    f.store
        .db
        .lock()
        .unwrap()
        .execute("INSERT INTO global_assets VALUES('global',0)", [])
        .unwrap();
    let film = f.file("exports/final-film.mp4");
    let named_film = f.file("exports/previs-my-final.mp4");
    let loose = f.file("imports/abandoned/staged.mp4");
    let outcome = f.clean();
    assert_eq!(outcome.deleted_records, 2);
    assert_eq!(outcome.deleted_files, 3);
    assert!(outcome.freed_bytes > 0);
    assert!(!PathBuf::from(orphan["path"].as_str().unwrap()).exists());
    assert!(!loose.exists());
    for asset in [live, library, imported, global] {
        assert!(PathBuf::from(asset["path"].as_str().unwrap()).exists());
    }
    assert!(film.exists());
    assert!(named_film.exists());
    let db = f.store.db.lock().unwrap();
    let raw: String = db
        .query_row("SELECT document FROM projects WHERE id='p'", [], |r| {
            r.get(0)
        })
        .unwrap();
    let doc: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(doc["assets"].as_array().unwrap().len(), 3);
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM project_assets WHERE asset_id='orphan'",
            [],
            |r| r.get::<_, u64>(0)
        )
        .unwrap(),
        0
    );
    assert_eq!(
        db.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
}
#[test]
fn current_references_across_projects_jobs_and_messages_pin_files_even_if_missing() {
    let f = Fixture::new();
    let shared = f.asset("shared", json!({}));
    let output = f.asset("result", json!({}));
    let attachment = f.asset("chat", json!({"attachmentOnly":true}));
    let missing = f.asset("missing", json!({}));
    fs::remove_file(missing["path"].as_str().unwrap()).unwrap();
    f.document(
        vec![missing],
        json!([{"id":"shot","shot":{"frames":[{"assetId":"missing"}]}}]),
    );
    write_document(&f.store,json!({"id":"q","name":"other","assets":[shared],"nodes":[],"clips":[{"assetId":"shared"}]}),true).unwrap();
    let db = f.store.db.lock().unwrap();
    db.execute(
        "INSERT INTO jobs VALUES('j','p',?1)",
        [json!({"status":"COMPLETED","asset":output}).to_string()],
    )
    .unwrap();
    db.execute("INSERT INTO agent_messages(project_id,role,content,model,payload,attribution) VALUES('p','user','','',?1,'{}')",[json!({"attachments":[{"id":attachment["id"],"kind":"asset"}]}).to_string()]).unwrap();
    drop(db);
    assert_eq!(f.clean().deleted_records, 0);
    assert!(f.root.join("assets/shared.png").exists());
    assert!(f.root.join("assets/chat.png").exists());
}
#[test]
fn changed_references_invalidate_the_review_before_any_deletion() {
    let f = Fixture::new();
    let asset = f.asset("newly-used", json!({}));
    let old = f.scan().report.fingerprint;
    f.document(vec![asset], json!([{"id":"node","assetId":"newly-used"}]));
    assert!(
        run::clean(&f.store, &f.data, &old)
            .err()
            .unwrap()
            .to_string()
            .contains("重新扫描")
    );
    assert!(f.root.join("assets/newly-used.png").exists());
}
#[test]
fn active_jobs_block_cleanup_and_recent_files_are_not_candidates() {
    let f = Fixture::new();
    f.asset("old", json!({}));
    fs::write(f.root.join("assets/fresh.png"), "being written").unwrap();
    assert_eq!(f.scan().report.unused_files, 1);
    f.store
        .db
        .lock()
        .unwrap()
        .execute(
            "INSERT INTO jobs VALUES('j','p','{\"status\":\"RUNNING\"}')",
            [],
        )
        .unwrap();
    let plan = f.scan();
    assert!(!plan.report.can_clean);
    assert!(run::clean(&f.store, &f.data, &plan.report.fingerprint).is_err());
    assert!(f.root.join("assets/old.png").exists());
}
#[test]
fn private_exports_and_task_snapshots_are_preserved_but_orphan_private_media_is_collected() {
    let f = Fixture::new();
    let db = f.store.db.lock().unwrap();
    for (id, folder) in [
        ("latest", "11111111-1111-4111-8111-111111111111"),
        ("task-input", "22222222-2222-4222-8222-222222222222"),
        ("orphan", "33333333-3333-4333-8333-333333333333"),
    ] {
        let file = f.file(&format!("exports/previs-{folder}/media/assets/{id}.mp4"));
        let value = json!({"id":id,"name":id,"path":file,"kind":"video"});
        db.execute(
            "INSERT INTO previs_media VALUES('p',?1,?2)",
            rusqlite::params![id, value.to_string()],
        )
        .unwrap();
        if id == "latest" {
            db.execute(
                "INSERT INTO previs_exports VALUES('p','export',?1)",
                [json!({"asset":value}).to_string()],
            )
            .unwrap();
        }
    }
    drop(db);
    f.document(
        vec![],
        json!([{"id":"shot","references":[{"assetId":"task-input"}]}]),
    );
    assert_eq!(f.clean().deleted_records, 1);
    assert!(
        f.root
            .join("exports/previs-11111111-1111-4111-8111-111111111111/media/assets/latest.mp4")
            .exists()
    );
    assert!(
        f.root
            .join("exports/previs-22222222-2222-4222-8222-222222222222/media/assets/task-input.mp4")
            .exists()
    );
    assert!(
        !f.root
            .join("exports/previs-33333333-3333-4333-8333-333333333333/media/assets/orphan.mp4")
            .exists()
    );
}
#[cfg(unix)]
#[test]
fn symlinks_and_unknown_files_are_never_deleted_and_hardlinks_are_not_double_counted() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    let outside = f.base.join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("keep"), "user data").unwrap();
    fs::create_dir(outside.join("empty-user-folder")).unwrap();
    symlink(&outside, f.root.join("imports")).unwrap();
    let kept = f.file("unknown-user-file.txt");
    let alias = f.root.join("assets/unreferenced-alias.png");
    fs::hard_link(&kept, &alias).unwrap();
    let plan = f.scan();
    assert_eq!(plan.report.unused_files, 1);
    assert_eq!(plan.report.reclaimable_bytes, 0);
    f.clean();
    assert!(outside.join("keep").exists());
    assert!(outside.join("empty-user-folder").exists());
    assert!(kept.exists());
    assert!(!alias.exists());
}
