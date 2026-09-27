//! Provider adapters are retained; execution belongs to the host harness.
use super::{
    config::Profile,
    harness::{self, Host, ProjectHost, session::Session},
};
use rig_core::completion::message::{Message, UserContent};
use serde_json::{Value, json};
use tauri::Manager;

#[cfg(test)]
pub(crate) async fn complete(
    profile: &Profile,
    key: &str,
    messages: Value,
    tool: Option<super::tools::ProjectTool>,
) -> Result<String, String> {
    complete_with_resume(profile, key, messages, tool, None, None).await
}
pub(crate) async fn complete_with_resume(
    profile: &Profile,
    key: &str,
    messages: Value,
    tool: Option<super::tools::ProjectTool>,
    resume: Option<&str>,
    model_id: Option<&str>,
) -> Result<String, String> {
    let token = tool.as_ref().map(|t| t.token.clone()).unwrap_or_default();
    let mut host = ProjectHost {
        media_profile: profile.clone(),
        tool,
        token,
        delegation: None,
    };
    let mut messages: Vec<_> = messages
        .as_array()
        .ok_or("对话格式无效")?
        .iter()
        .map(convert)
        .collect();
    let mut fork_history = Vec::new();
    if let Some(t) = &host.tool {
        let store = t.app.state::<crate::database::Store>();
        let mut binding = json!({"provider":profile.adapter,"endpoint":profile.endpoint,"model":profile.model,"agentId":t.profile.id,"revision":t.profile.revision,"tools":host.definitions()});
        harness::session_selection::bind_task(&store, &t.project, &t.turn, resume, &mut binding)?;
        fork_history = harness::handoff::completed(&store, &t.project, &t.turn, &binding)?;
        let restored = if let Some(turn) = resume {
            harness::session::restore(&store, &t.project, turn, &binding)?
        } else {
            harness::session::latest(&store, &t.project, &binding)?
        };
        harness::session_selection::record_selection(
            &store,
            &t.project,
            &t.turn,
            &binding,
            restored.as_ref().map_or(0, Vec::len),
        )?;
        if let Some(mut restored) = restored {
            let mut additions = messages.iter().rev().take(2).cloned().collect::<Vec<_>>();
            additions.reverse();
            if resume.is_none()
                && additions.first().is_some_and(|fresh| {
                    harness::session::project_snapshot_text(fresh).is_some_and(|text| {
                        restored
                            .iter()
                            .rev()
                            .find_map(harness::session::project_snapshot_text)
                            .is_some_and(|old| harness::session::same_project_snapshot(old, text))
                    })
                })
            {
                additions.remove(0);
            }
            if resume.is_some() {
                additions.pop();
            } // Original user input is already committed.
            // Reconcile the system prompt and inject fresh project state at the step boundary.
            if let (Some(Message::System { content }), Some(Message::System { content: old })) =
                (messages.first(), restored.first_mut())
            {
                *old = content.clone();
            }
            restored.extend(additions);
            messages = restored;
        }
        harness::session::start(&store, &t.project, &t.turn, binding, &messages)?;
    }
    host.delegation = Some(harness::delegation::Context {
        profile: profile.clone(),
        key: key.into(),
        model_id: model_id.map(str::to_owned),
        fork_history,
        deadline: tokio::time::Instant::now() + std::time::Duration::from_secs(20 * 60),
    });
    let model = super::provider::builder(profile, key)?
        .build()
        .model_handle()
        .clone();
    let mut session = Session::new(messages);
    if host
        .tool
        .as_ref()
        .is_some_and(|tool| tool.prompt.trim() == "/compact")
    {
        let changed = harness::compact_manual(&model, profile, &host, &mut session).await?;
        let reply = if changed {
            "已压缩较早的对话，上下文已更新。"
        } else {
            "当前对话没有可压缩的较早内容。"
        };
        session.append(&host, Message::assistant(reply))?;
        return Ok(reply.into());
    }
    harness::run(&model, profile, &host, session, host.tool.is_some(), key).await
}

pub(crate) fn convert(value: &Value) -> Message {
    if value["role"] == "system" {
        return Message::System {
            content: value["content"].as_str().unwrap_or("").into(),
        };
    }
    if value["role"] == "assistant" {
        return Message::assistant(value["content"].as_str().unwrap_or(""));
    }
    if let Some(text) = value["content"].as_str() {
        return Message::user(text);
    }
    let content = value["content"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|part| match part["type"].as_str()? {
            "text" => Some(UserContent::text(part["text"].as_str().unwrap_or(""))),
            "image_url" => {
                let url = part["image_url"]["url"].as_str()?;
                if let Some((mime, data)) = url
                    .strip_prefix("data:")
                    .and_then(|s| s.split_once(";base64,"))
                {
                    use rig_core::completion::message::ImageMediaType;
                    let format = match mime {
                        "image/jpeg" => ImageMediaType::JPEG,
                        "image/png" => ImageMediaType::PNG,
                        "image/webp" => ImageMediaType::WEBP,
                        "image/gif" => ImageMediaType::GIF,
                        _ => return None,
                    };
                    Some(UserContent::image_base64(data, Some(format), None))
                } else {
                    Some(UserContent::image_url(url, None, None))
                }
            }
            "media" => serde_json::from_value(part["content"].clone()).ok(),
            _ => None,
        })
        .collect();
    Message::User { content }
}
