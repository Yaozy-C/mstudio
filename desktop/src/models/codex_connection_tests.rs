use super::*;

#[test]
fn only_explicit_method_not_found_means_images_are_unsupported() {
    assert!(
        !image_capability(Err(RpcError {
            code: -32601,
            message: "method absent".into()
        }
        .into()))
        .unwrap()
    );
    assert!(!image_capability(Ok(json!({"imageGeneration":false}))).unwrap());
    assert!(image_capability(Ok(json!({"imageGeneration":true}))).unwrap());
    let error = image_capability(Err(RpcError {
        code: -32000,
        message: "account unavailable".into(),
    }
    .into()))
    .unwrap_err();
    assert_eq!(error.downcast_ref::<RpcError>().unwrap().code, -32000);
    assert!(
        image_capability(Err(anyhow::anyhow!("transport failed")))
            .unwrap_err()
            .to_string()
            .contains("transport failed")
    );
    assert!(image_capability(Ok(json!({}))).is_err());
    assert!(image_capability(Ok(json!({"imageGeneration":"true"}))).is_err());
}

#[cfg(unix)]
#[tokio::test]
async fn account_discovery_preserves_capability_failure_without_a_second_process() {
    use std::os::unix::fs::PermissionsExt;
    let path = std::env::temp_dir().join(format!("codex-account-{}", mstudio::media::id()));
    std::fs::write(
        &path,
        r#"#!/bin/sh
read -r request
printf '%s\n' '{"id":1,"result":{}}'
read -r initialized
read -r request
printf '%s\n' '{"id":2,"result":{"account":{"type":"chatgpt"}}}'
read -r request
printf '%s\n' '{"id":3,"result":{"data":[{"model":"explicit-model"}]}}'
read -r request
printf '%s\n' '{"id":4,"error":{"code":-32000,"message":"capabilities unavailable"}}'
"#,
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut rpc = Rpc::connect(&path).await.unwrap();
    let error = inspect_rpc(&mut rpc).await.unwrap_err();
    assert_eq!(error.downcast_ref::<RpcError>().unwrap().code, -32000);
    assert!(error.to_string().contains("capabilities unavailable"));
    rpc.stop().await;
    std::fs::remove_file(path).unwrap();
}

#[test]
fn absent_default_saves_only_the_connection_and_preserves_explicit_choice() {
    let root = std::env::temp_dir().join(format!("codex-no-default-{}", mstudio::media::id()));
    let store = Store::open(root.clone()).unwrap();
    {
        let db = store.db.lock().unwrap();
        let service = connections::ServiceConnection {
            id: "explicit".into(),
            name: "Codex".into(),
            kind: "codex".into(),
            endpoint: "codex://local".into(),
            has_key: false,
            model_count: 0,
        };
        let report = json!({"models":[{"id":"arbitrary-first","displayName":"First","isDefault":false},{"id":"chosen-model","displayName":"Chosen","isDefault":false}],"images":false});
        let saved = sync(&db, service.clone(), &report).unwrap();
        assert_eq!(saved.model_count, 0);
        assert!(storage::catalog(&db, None).unwrap().profiles.is_empty());
        storage::save(
            &db,
            Model {
                id: "user-choice".into(),
                name: "Chosen".into(),
                connection_id: Some(service.id.clone()),
                has_key: false,
                profile: crate::assistant::config::Profile {
                    adapter: "codex".into(),
                    endpoint: "codex://local".into(),
                    model: "chosen-model".into(),
                    ..Default::default()
                },
            },
        )
        .unwrap();
        let saved = sync(&db, service, &report).unwrap();
        assert_eq!(saved.model_count, 1);
        let profiles = storage::catalog(&db, None).unwrap().profiles;
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].profile.model, "chosen-model");
    }
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn file_login_error_is_reported_without_discarding_text_models() {
    let report = discovery_report(
        vec![json!({"id":"explicit"})],
        true,
        Err(anyhow::anyhow!("Codex 登录文件无效")),
    );
    assert_eq!(report["status"], "ready");
    assert_eq!(report["models"][0]["id"], "explicit");
    assert_eq!(report["images"], false);
    assert_eq!(report["imageError"], "Codex 登录文件无效");
}
