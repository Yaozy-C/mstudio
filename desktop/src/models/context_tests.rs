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
    assert_eq!(window(&profile), None);
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
    assert_eq!(window(&profile), None);
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
    assert!(super::super::context_policy::validate_policy(&profile).is_err());
    assert_eq!(threshold(&profile, 8192), None);
    profile.context_window = Some(1_000_000);
    profile.limits.compaction_headroom = Some(890_000);
    assert_eq!(threshold(&profile, 1_000_000), None);
    profile.adapter = "codex".into();
    assert_eq!(output_cap(&profile), None);
    assert!(super::super::context_policy::validate_policy(&profile).is_err());
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
        .unwrap()
        .unwrap();
    task.join().unwrap();
    assert_eq!(limits.context_window, 524_288);
    assert_eq!(limits.source, "service");
    assert_eq!(limits.max_output_tokens, Some(32_768));
}

#[tokio::test]
async fn gemini_model_get_discovers_capacity_without_inventing_an_output_default() {
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
    profile.limits.model_limits = super::super::context_commands::discover(&profile, "fixture")
        .await
        .unwrap();
    task.join().unwrap();
    assert_eq!(profile.context_window(), Some(1_048_576));
    assert_eq!(
        profile
            .limits
            .model_limits
            .as_ref()
            .unwrap()
            .max_output_tokens,
        Some(32_768)
    );
    assert_eq!(output_cap(&profile), None);
}
