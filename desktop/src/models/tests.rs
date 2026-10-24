use super::*;
use crate::database::Store;
use serde_json::json;

fn fixture() -> (std::path::PathBuf, Store) {
    let root = std::env::temp_dir().join(format!("mstudio-models-{}", mstudio::media::id()));
    let store = Store::open(root.clone()).unwrap();
    store.db.lock().unwrap().execute_batch("INSERT INTO projects VALUES('a','A','{}',0); INSERT INTO projects VALUES('b','B','{}',0);").unwrap();
    {
        let db = store.db.lock().unwrap();
        for (id, key) in [("one", "private-one"), ("two", "private-two")] {
            connections::save(
                &db,
                connections::ServiceConnection {
                    id: format!("service-{id}"),
                    name: id.into(),
                    kind: "openai-compatible".into(),
                    endpoint: "http://127.0.0.1:1234/v1".into(),
                    has_key: false,
                    model_count: 0,
                },
                Some(key.into()),
                false,
            )
            .unwrap();
        }
    }
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
            context_window: Some(262_144),
            limits: Default::default(),
            inputs: Default::default(),
        },
        connection_id: Some(format!("service-{id}")),
        has_key: false,
    }
}

#[test]
fn profiles_defaults_and_project_choice_persist_without_exposing_credentials() {
    let (root, store) = fixture();
    {
        let db = store.db.lock().unwrap();
        let mut unbound = model("one");
        unbound.connection_id = None;
        assert!(
            storage::save(&db, unbound)
                .unwrap_err()
                .to_string()
                .contains("服务连接")
        );
        storage::save(&db, model("one")).unwrap();
        storage::save(&db, model("two")).unwrap();
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
        db.execute(
            "UPDATE model_profiles SET data='invalid' WHERE id='two'",
            [],
        )
        .unwrap();
        assert_eq!(resolve(&db, Some("a"), None).unwrap().0.id, "one");
        assert_eq!(resolve(&db, None, Some("one")).unwrap().0.id, "one");
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
        let profile = model("one");
        storage::save(&db, profile).unwrap();
        storage::select(&db, "a", Some("one")).unwrap();
        let mut service = connections::get(&db, "service-one").unwrap();
        service.endpoint = "https://other.example/v1".into();
        assert!(connections::save(&db, service.clone(), None, false).is_err());
        assert_eq!(resolve(&db, None, Some("one")).unwrap().1, "private-one");
        connections::save(&db, service, None, true).unwrap();
        assert!(resolve(&db, None, Some("one")).is_err());
        assert!(!catalog(&db, None).unwrap().profiles[0].has_key);
        storage::remove(&db, "one").unwrap();
        let catalog = catalog(&db, Some("a")).unwrap();
        assert!(catalog.profiles.is_empty());
        assert!(catalog.default_id.is_none() && catalog.selected_id.is_none());
        assert!(connections::key(&db, "service-one").unwrap().is_empty());
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
        storage::save(&db, model("one")).unwrap();
        let mut service = connections::get(&db, "service-two").unwrap();
        service.endpoint = endpoint;
        connections::save(&db, service, Some("local-key-two".into()), false).unwrap();
        storage::save(&db, model("two")).unwrap();
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
