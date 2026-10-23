use super::*;
use crate::assistant::config::Profile;
use serde_json::json;

#[test]
fn unknown_capacity_is_not_an_unlimited_execution_budget() {
    let root = std::env::temp_dir().join(format!("context-config-{}", mstudio::media::id()));
    let store = crate::database::Store::open(root.clone()).unwrap();
    {
        let db = store.db.lock().unwrap();
        db.execute("INSERT INTO service_connections VALUES('local','Local','openai-compatible','http://127.0.0.1:1234/v1','')", []).unwrap();
        let mut model = Model {
            id: "unknown".into(),
            name: "Unknown model".into(),
            connection_id: Some("local".into()),
            has_key: false,
            profile: Profile {
                endpoint: "http://127.0.0.1:1234/v1".into(),
                model: "uncatalogued".into(),
                adapter: "openai-compatible".into(),
                ..Default::default()
            },
        };
        // Settings can retain an incomplete model, but it cannot become executable.
        storage::save(&db, model.clone()).unwrap();
        let error = resolve(&db, None, Some("unknown"))
            .err()
            .unwrap()
            .to_string();
        assert!(error.contains("刷新容量或手动填写"));
        assert!(catalog(&db, None).unwrap().default_id.is_none());
        model.profile.context_window = Some(200_000);
        model.profile.limits.context_window_mode = Some(context::Mode::Manual);
        storage::save(&db, model).unwrap();
        let explicit = resolve(&db, None, Some("unknown")).unwrap().0;
        assert_eq!(explicit.profile.context_window(), Some(200_000));
        assert_eq!(context::output_cap(&explicit.profile), None);
    }
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn service_metadata_does_not_invent_output_defaults() {
    for adapter in ["anthropic-native", "gemini-native", "openai-compatible"] {
        let mut profile = Profile {
            endpoint: "https://private.example".into(),
            model: "new-model".into(),
            adapter: adapter.into(),
            ..Default::default()
        };
        assert!(context::window(&profile).is_none());
        assert!(context::output_cap(&profile).is_none());
        let limits = context::from_service(
            &profile,
            &json!({"contextWindow":500000,"maxOutputTokens":32768}),
        )
        .unwrap();
        assert_eq!(limits.default_max_tokens, None);
        profile.limits.model_limits = Some(limits);
        assert_eq!(context::window(&profile), Some(500_000));
        assert_eq!(context::output_cap(&profile), None);
        if adapter == "anthropic-native" {
            assert!(
                context_policy::validate_execution(&profile)
                    .unwrap_err()
                    .to_string()
                    .contains("手动填写输出上限")
            );
        } else {
            context_policy::validate_execution(&profile).unwrap();
        }
        profile.limits.max_output_tokens = Some(1234);
        assert_eq!(context::output_cap(&profile), Some(1234));
        context_policy::validate_execution(&profile).unwrap();
        profile.limits.context_window_mode = Some(context::Mode::Manual);
        assert!(context_policy::validate_policy(&profile).is_err());
    }
    let codex = Profile {
        adapter: "codex".into(),
        ..Default::default()
    };
    context_policy::validate_execution(&codex).unwrap();
    assert_eq!(context::window(&codex), None);
    assert_eq!(context::output_cap(&codex), None);
}
