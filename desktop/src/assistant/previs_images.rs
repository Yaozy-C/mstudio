//! Use the same project ownership checks and provider image transport as chat.
use crate::database::Store;
use anyhow::{Result, ensure};
use rig_core::message::Message;
use serde_json::{Value, json};

pub fn message(
    store: &Store,
    doc: &Value,
    prompt: &str,
    ids: &[String],
    profile: &super::config::Profile,
) -> Result<Message> {
    ensure!(ids.len() <= 12, "本次最多引用 12 张图片");
    for id in ids {
        let asset = crate::previs::media::find(doc, id);
        ensure!(
            asset.is_some_and(|a| a["kind"] == "image"),
            "请选择当前项目中的图片"
        );
    }
    let refs = ids
        .iter()
        .map(|id| super::attachments::Reference {
            kind: "asset".into(),
            id: id.clone(),
        })
        .collect::<Vec<_>>();
    let payload = super::attachments::payload(store, doc, prompt, &refs, profile)?;
    Ok(super::agent::convert(
        &json!({"role":"user","content":payload}),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rig_core::message::UserContent;

    #[test]
    fn previs_images_reach_both_provider_transports_and_fail_closed() {
        let (root, store, doc) = super::super::attachment_tests::fixture();
        let mut profile = super::super::attachment_tests::multimodal();
        let ids = vec!["image".into(), "image".into()];
        let Message::User { content } =
            message(&store, &doc, "Edit this model", &ids, &profile).unwrap()
        else {
            panic!("expected user message");
        };
        assert_eq!(
            content
                .iter()
                .filter(|c| matches!(c, UserContent::Image(_)))
                .count(),
            1
        );
        assert!(serde_json::to_string(&content).unwrap().contains("data"));
        profile.adapter = "codex".into();
        let input = message(&store, &doc, "Edit this model", &ids, &profile).unwrap();
        let turn = super::super::codex_input::turn_input(&[input]).unwrap();
        assert!(
            turn.as_array()
                .unwrap()
                .iter()
                .any(|part| part["type"] == "localImage")
        );
        assert!(!turn.to_string().contains("base64"));
        for invalid in [
            vec!["foreign".into()],
            vec!["video".into()],
            vec!["image".into(); 13],
        ] {
            assert!(message(&store, &doc, "Edit", &invalid, &profile).is_err());
        }
        profile.inputs.image = false;
        assert!(message(&store, &doc, "Edit", &ids, &profile).is_err());
        assert!(message(&store, &doc, "Text only", &[], &profile).is_ok());
        profile.inputs.image = true;
        std::fs::remove_file(root.join("assets/image.jpg")).unwrap();
        assert!(message(&store, &doc, "Edit", &ids, &profile).is_err());
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }
}
