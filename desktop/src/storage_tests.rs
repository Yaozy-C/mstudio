use crate::{database::Store, jobs};
use serde_json::json;
#[test]
fn settings_projects_and_jobs_survive_reopen() -> anyhow::Result<()> {
    let root = std::env::temp_dir().join(format!("mstudio-db-{}", mstudio::media::id()));
    let store = Store::open(root.clone())?;
    store.set_setting("plugins", "[\"export\"]")?;
    store.db.lock().unwrap().execute(
        "INSERT INTO projects VALUES(?1,?2,?3,1)",
        rusqlite::params![
            "p1",
            "First",
            json!({"id":"p1","name":"First","nodes":[]}).to_string()
        ],
    )?;
    let job = json!({"id":"j1","projectId":"p1","status":"IN_QUEUE","requestId":"remote-1"});
    jobs::save(&store, &job)?;
    drop(store);
    let store = Store::open(root.clone())?;
    assert_eq!(store.setting("plugins")?, "[\"export\"]");
    assert_eq!(store.projects()?[0]["document"]["name"], "First");
    assert_eq!(jobs::get(&store, "j1")?["requestId"], "remote-1");
    let integrity: String =
        store
            .db
            .lock()
            .unwrap()
            .query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
    assert_eq!(integrity, "ok");
    store
        .db
        .lock()
        .unwrap()
        .execute("UPDATE projects SET document='invalid'", [])?;
    assert!(
        store.projects().is_err(),
        "Corrupt documents must not become empty projects"
    );
    drop(store);
    std::fs::remove_dir_all(root)?;
    Ok(())
}
#[test]
fn credentials_only_go_to_real_queue_origin() {
    assert!(jobs::queue_url("https://queue.fal.run/fal-ai/test/requests/123/status").is_ok());
    for url in [
        "http://queue.fal.run/test",
        "https://queue.fal.run.evil.test/test",
        "https://evil.test/test",
        "https://name:pass@queue.fal.run/test",
        "https://queue.fal.run:9000/test",
    ] {
        assert!(jobs::queue_url(url).is_err());
    }
}
