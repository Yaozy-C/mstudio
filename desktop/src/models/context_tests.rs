use super::*;
use serde_json::json;

fn gemini() -> Profile {
    Profile {
        endpoint: "https://generativelanguage.googleapis.com".into(),
        model: "gemini-4-argon".into(),
        adapter: "gemini-native".into(),
        ..Default::default()
    }
}

#[test]
fn official_capacity_manual_override_and_route_scoping() {
    let mut profile = gemini();
    assert_eq!(window(&profile), Some(1_048_576));
    assert_eq!(output_cap(&profile), Some(32_768));
    assert_eq!(threshold(&profile, 1_048_576), Some(838_860));
    profile.context_window = Some(200_000);
    assert_eq!(window(&profile), Some(200_000));
    assert_eq!(threshold(&profile, 200_000), Some(101_696));
    profile.limits.context_window_mode = Some(Mode::Auto);
    assert_eq!(window(&profile), Some(1_048_576));
    profile.endpoint = "https://google-proxy.example".into();
    assert_eq!(window(&profile), Some(UNKNOWN_CONTEXT));
    profile.adapter = "codex".into();
    assert_eq!(window(&profile), None);
}

#[test]
fn service_metadata_wins_only_for_its_exact_route_and_model() {
    let mut profile = gemini();
    profile.limits.model_limits = from_service(
        &profile,
        &json!({
            "inputTokenLimit":1_000_000,"outputTokenLimit":32_768
        }),
    );
    assert_eq!(window(&profile), Some(1_000_000));
    assert_eq!(output_cap(&profile), Some(32_768));
    profile.model = "new-model".into();
    assert_eq!(window(&profile), Some(UNKNOWN_CONTEXT));
    assert!(from_service(&profile, &json!({"context_length":0})).is_none());
    assert_eq!(
        from_service(&profile, &json!({"context_length":"500000"}))
            .unwrap()
            .context_window,
        500_000
    );
}

#[test]
fn dsh_threshold_reserves_actual_output_and_summary_headroom() {
    let mut profile = Profile {
        endpoint: "https://api.deepseek.com".into(),
        model: "deepseek-flash".into(),
        adapter: "openai-responses".into(),
        ..Default::default()
    };
    assert_eq!(window(&profile), Some(1_000_000));
    assert_eq!(output_cap(&profile), Some(256_000));
    assert_eq!(threshold(&profile, 1_000_000), Some(678_464));
    profile.limits.max_output_tokens = Some(100_000);
    profile.limits.compaction_headroom = Some(10_000);
    assert_eq!(threshold(&profile, 1_000_000), Some(800_000));
    profile.context_window = Some(8192);
    assert!(super::super::context_migration::validate_policy(&profile).is_err());
    assert_eq!(threshold(&profile, 8192), None);
    profile.context_window = Some(1_000_000);
    profile.limits.compaction_headroom = Some(890_000);
    assert_eq!(threshold(&profile, 1_000_000), None);
    profile.adapter = "codex".into();
    assert_eq!(output_cap(&profile), None);
    assert!(super::super::context_migration::validate_policy(&profile).is_err());
}

#[test]
fn legacy_repair_is_atomic_idempotent_and_preserves_custom_overrides() {
    let db = rusqlite::Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE settings(key TEXT PRIMARY KEY,value TEXT); CREATE TABLE model_profiles(id TEXT PRIMARY KEY,data TEXT);").unwrap();
    for (id, window, host) in [
        ("old", json!(32768), "generativelanguage.googleapis.com"),
        ("unset", json!(null), "generativelanguage.googleapis.com"),
        ("custom", json!(200000), "generativelanguage.googleapis.com"),
        ("proxy", json!(32768), "proxy.example"),
    ] {
        let mut raw = serde_json::to_value(gemini()).unwrap();
        raw["id"] = json!(id);
        raw["name"] = json!(id);
        raw["contextWindow"] = window;
        raw["endpoint"] = json!(format!("https://{host}"));
        raw["unrelated"] = json!({"keep":true});
        db.execute(
            "INSERT INTO model_profiles VALUES(?1,?2)",
            rusqlite::params![id, raw.to_string()],
        )
        .unwrap();
    }
    super::super::context_migration::migrate(&db).unwrap();
    let read = |id| -> Value {
        let raw: String = db
            .query_row("SELECT data FROM model_profiles WHERE id=?1", [id], |row| {
                row.get(0)
            })
            .unwrap();
        serde_json::from_str(&raw).unwrap()
    };
    assert_eq!(read("old")["contextWindow"], 1_048_576);
    assert_eq!(read("unset")["contextWindowMode"], "auto");
    assert_eq!(read("old")["unrelated"]["keep"], true);
    assert_eq!(read("custom")["contextWindow"], 200_000);
    assert_eq!(read("proxy")["contextWindow"], 32_768);
    let before = read("old");
    super::super::context_migration::migrate(&db).unwrap();
    assert_eq!(read("old"), before);
    let backup: String = db
        .query_row(
            "SELECT value FROM settings WHERE key='text-context-window-v1-backup'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Vec<Value>>(&backup).unwrap().len(),
        2
    );
}

#[tokio::test]
async fn uncatalogued_service_discovery_reads_real_metadata_without_generation() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let profile = Profile {
        endpoint: format!("http://{}/v1", listener.local_addr().unwrap()),
        model: "brand-new-model".into(),
        adapter: "openai-compatible".into(),
        ..Default::default()
    };
    let task = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut bytes = [0; 4096];
        let count = stream.read(&mut bytes).unwrap();
        let request = String::from_utf8_lossy(&bytes[..count]);
        assert!(request.starts_with("GET /v1/models "));
        let body = json!({"data":[{"id":"brand-new-model","context_length":524288,"max_output_tokens":32768}]}).to_string();
        write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
    });
    let limits = super::super::context_commands::discover(&profile, "fixture")
        .await
        .unwrap();
    task.join().unwrap();
    assert_eq!(limits.context_window, 524_288);
    assert_eq!(limits.source, "service");
    assert_eq!(limits.max_output_tokens, Some(32_768));
}

#[tokio::test]
async fn gemini_model_get_discovers_a_new_model_and_clamps_its_default_output_cap() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let mut profile = Profile {
        endpoint: format!("http://{}", listener.local_addr().unwrap()),
        model: "brand-new-gemini".into(),
        adapter: "gemini-native".into(),
        ..Default::default()
    };
    let task = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut bytes = [0; 4096];
        let count = stream.read(&mut bytes).unwrap();
        let request = String::from_utf8_lossy(&bytes[..count]).to_lowercase();
        assert!(request.starts_with("get /v1beta/models/brand-new-gemini "));
        assert!(request.contains("x-goog-api-key: fixture"));
        let body = json!({"name":"models/brand-new-gemini","inputTokenLimit":1048576,"outputTokenLimit":32768}).to_string();
        write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
    });
    profile.limits.model_limits =
        super::super::context_commands::discover(&profile, "fixture").await;
    task.join().unwrap();
    assert_eq!(profile.context_window(), Some(1_048_576));
    assert_eq!(output_cap(&profile), Some(32_768));
}
