use super::*;
use crate::database::Store;
use serde_json::json;

fn fixture() -> (std::path::PathBuf, Store) {
    let root = std::env::temp_dir().join(format!("mstudio-models-{}", mstudio::media::id()));
    let store = Store::open(root.clone()).unwrap();
    store.db.lock().unwrap().execute_batch("INSERT INTO projects VALUES('a','A','{}',0); INSERT INTO projects VALUES('b','B','{}',0);").unwrap();
    (root, store)
}
fn model(id: &str) -> Model {
    Model {
        id: id.into(),
        name: format!("Connection {id}"),
        profile: Profile {
            endpoint: "http://127.0.0.1:1234/v1".into(),
            model: id.into(),
            adapter: "openai-compatible".into(),
            context_window: None,
            inputs: Default::default(),
        },
        connection_id: None,
        has_key: false,
    }
}

#[test]
fn profiles_defaults_and_project_choice_persist_without_exposing_credentials() {
    let (root, store) = fixture();
    {
        let db = store.db.lock().unwrap();
        storage::save(&db, model("one"), Some("private-one".into()), false).unwrap();
        storage::save(&db, model("two"), Some("private-two".into()), false).unwrap();
        storage::select(&db, "a", Some("two")).unwrap();
        assert_eq!(resolve(&db, Some("a"), None).unwrap().0.id, "two");
        assert_eq!(resolve(&db, Some("b"), None).unwrap().0.id, "one");
        let value = serde_json::to_string(&catalog(&db, Some("a")).unwrap()).unwrap();
        assert!(!value.contains("private-"));
        assert!(value.contains("\"hasKey\":true"));
        let captured = resolve(&db, Some("a"), Some("two")).unwrap();
        storage::select(&db, "a", Some("one")).unwrap();
        assert_eq!(captured.0.id, "two");
        assert_eq!(captured.1, "private-two");
        assert!(resolve(&db, Some("a"), Some("missing")).is_err());
    }
    drop(store);
    let store = Store::open(root.clone()).unwrap();
    assert_eq!(
        resolve(&store.db.lock().unwrap(), Some("a"), None)
            .unwrap()
            .0
            .id,
        "one"
    );
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn editing_destination_requires_new_key_and_deletion_clears_all_preferences() {
    let (root, store) = fixture();
    {
        let db = store.db.lock().unwrap();
        let mut profile = model("one");
        storage::save(&db, profile.clone(), Some("private-key".into()), false).unwrap();
        storage::select(&db, "a", Some("one")).unwrap();
        profile.profile.endpoint = "https://other.example/v1".into();
        assert!(storage::save(&db, profile.clone(), None, false).is_err());
        assert_eq!(resolve(&db, None, Some("one")).unwrap().1, "private-key");
        storage::save(&db, profile, None, true).unwrap();
        assert!(resolve(&db, None, Some("one")).is_err());
        assert!(!catalog(&db, None).unwrap().profiles[0].has_key);
        storage::remove(&db, "one").unwrap();
        let catalog = catalog(&db, Some("a")).unwrap();
        assert!(catalog.profiles.is_empty());
        assert!(catalog.default_id.is_none() && catalog.selected_id.is_none());
        assert_eq!(
            db.query_row("SELECT count(*) FROM model_credentials", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn chosen_connection_routes_the_real_sdk_and_connection_check_to_its_model_and_key() {
    use std::io::{Read, Write};
    let server = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}/v1", server.local_addr().unwrap());
    let task = std::thread::spawn(move || {
        for index in 0..2 {
            let (mut stream, _) = server.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(10)))
                .unwrap();
            let mut bytes = Vec::new();
            loop {
                let mut chunk = [0; 4096];
                let count = stream.read(&mut chunk).unwrap();
                assert!(count > 0);
                bytes.extend_from_slice(&chunk[..count]);
                if let Some(pos) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..pos]).to_lowercase();
                    let size = headers
                        .lines()
                        .find_map(|l| {
                            l.strip_prefix("content-length:")
                                .and_then(|v| v.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if bytes.len() >= pos + 4 + size {
                        break;
                    }
                }
            }
            let request = String::from_utf8_lossy(&bytes).to_lowercase();
            assert!(request.contains("authorization: bearer local-key-two"));
            let body = if index == 0 {
                assert!(request.starts_with("get /v1/models"));
                json!({"data":[{"id":"two"}]})
            } else {
                assert!(request.starts_with("post /v1/chat/completions"));
                assert!(request.contains("\"model\":\"two\""));
                json!({"id":"test","object":"chat.completion","created":0,"model":"two","choices":[{"index":0,"message":{"role":"assistant","content":"Selected model replied"},"finish_reason":"stop"}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}})
            }.to_string();
            write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
        }
    });
    let (root, store) = fixture();
    let selected = {
        let db = store.db.lock().unwrap();
        storage::save(&db, model("one"), None, false).unwrap();
        let mut second = model("two");
        second.profile.endpoint = endpoint;
        storage::save(&db, second, Some("local-key-two".into()), false).unwrap();
        storage::select(&db, "a", Some("two")).unwrap();
        resolve(&db, Some("a"), None).unwrap()
    };
    assert!(
        commands::check_connection(&selected.0, &selected.1)
            .await
            .unwrap()
            .contains("已找到")
    );
    let answer = crate::assistant::complete_for_test(
        &selected.0.profile,
        &selected.1,
        json!([{"role":"system","content":"test"},{"role":"user","content":"hello"}]),
        None,
    )
    .await
    .unwrap();
    assert_eq!(answer, "Selected model replied");
    task.join().unwrap();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn shared_services_migrate_accounts_and_keep_model_choices() {
    let (root, store) = fixture();
    {
        let db = store.db.lock().unwrap();
        let mut chat = model("google-chat");
        chat.profile.endpoint = "https://generativelanguage.googleapis.com".into();
        chat.profile.adapter = "gemini-native".into();
        storage::save(&db, chat.clone(), Some("shared-secret".into()), false).unwrap();
        storage::select(&db, "a", Some(&chat.id)).unwrap();
        chat.id = "other-account".into();
        storage::save(&db, chat, Some("other-secret".into()), false).unwrap();
        let media = json!([{"id":"image","name":"Image","kind":"image","plugin":"gemini-native","endpoint":"https://generativelanguage.googleapis.com","params":{"model":"image-model"},"enabled":true}]);
        db.execute(
            "INSERT INTO settings VALUES('media-models',?1)",
            [media.to_string()],
        )
        .unwrap();
        db.execute(
            "INSERT INTO settings VALUES('media-key:image','shared-secret')",
            [],
        )
        .unwrap();
    }
    drop(store);
    let store = Store::open(root.clone()).unwrap();
    let connection_id;
    {
        let db = store.db.lock().unwrap();
        let services = connections::list(&db).unwrap();
        assert_eq!(services.len(), 2);
        let chat = resolve(&db, None, Some("google-chat")).unwrap().0;
        let image = media::resolved(&db).unwrap().remove(0);
        assert_eq!(chat.connection_id, image.connection_id);
        connection_id = chat.connection_id.unwrap();
        assert_eq!(resolve(&db, Some("a"), None).unwrap().0.id, "google-chat");
        assert!(connections::get(&db, &connection_id).unwrap().has_key);
        assert!(!serde_json::to_string(&services).unwrap().contains("secret"));
        assert!(
            !serde_json::to_string(&catalog(&db, None).unwrap())
                .unwrap()
                .contains("secret")
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM model_credentials", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert!(setting(&db, "media-key:image").unwrap().is_none());
        let mut service = connections::get(&db, &connection_id).unwrap();
        connections::save(&db, service.clone(), Some("rotated-secret".into()), false).unwrap();
        assert_eq!(
            resolve(&db, None, Some("google-chat")).unwrap().1,
            "rotated-secret"
        );
        assert_eq!(
            connections::media_key(&db, &image).unwrap(),
            "rotated-secret"
        );
        assert_eq!(
            resolve(&db, None, Some("other-account")).unwrap().1,
            "other-secret"
        );
        service.endpoint = "https://another.example".into();
        assert!(connections::save(&db, service, None, false).is_err());
    }
    drop(store);
    let store = Store::open(root.clone()).unwrap();
    {
        let db = store.db.lock().unwrap();
        assert_eq!(connections::list(&db).unwrap().len(), 2);
        assert_eq!(
            resolve(&db, None, Some("google-chat"))
                .unwrap()
                .0
                .connection_id
                .as_deref(),
            Some(connection_id.as_str())
        );
    }
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
