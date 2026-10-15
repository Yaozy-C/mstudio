//! Active media reads are separate from project text and explicit user attachments.
use crate::assistant::profiles::{self, AgentProfile};
use rig_core::completion::ToolDefinition;
use serde_json::{Value, json};

pub fn check(profile: &AgentProfile) -> Result<(), Value> {
    if profiles::allows(profile, "read_media") {
        Ok(())
    } else {
        Err(json!({"error":"Media content read permission required","code":"FORBIDDEN"}))
    }
}

pub fn definitions(profile: &AgentProfile) -> Vec<ToolDefinition> {
    if check(profile).is_err() {
        return vec![];
    }
    vec![ToolDefinition {
        name: "mstudio_read_image".try_into().expect("nonempty tool name"),
        description: "Read real image pixels or a video frame directly into the model. Use an assetId from a receipt or inspected project state. For video, time is source seconds unless clipId is supplied; then it is clip-local seconds, accounting for trim/speed and current grading, excluding transitions, overlays and captions. transition:true with the incoming clipId uses seconds from transition start and returns the actual transition composite, excluding overlays, captions and audio. Sample relevant beginnings, middles, endings and both sides of joins; samples do not prove full playback. Requires image-input support.".into(),
        parameters: json!({"type":"object","properties":{"assetId":{"type":"string"},"clipId":{"type":"string"},"transition":{"type":"boolean"},"time":{"type":"number","minimum":0}},"required":["assetId"],"additionalProperties":false}),
    }, ToolDefinition {
        name: "mstudio_reopen_image".try_into().expect("nonempty tool name"),
        description: "Reopen a historical offloaded image by imageId from its notice. Only offloaded images in this project are accessible; returns actual image content.".into(),
        parameters: json!({"type":"object","properties":{"imageId":{"type":"string"}},"required":["imageId"],"additionalProperties":false}),
    }, ToolDefinition {
        name: "mstudio_reopen_media".try_into().expect("nonempty tool name"),
        description: "Reopen an offloaded audio, video, document or text attachment by mediaId from its notice only when needed again. Returns attachment content for the next model request; processed content then becomes a reference again. Requires compatible model input support.".into(),
        parameters: json!({"type":"object","properties":{"mediaId":{"type":"string"}},"required":["mediaId"],"additionalProperties":false}),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_and_execution_guard_follow_media_permission() {
        for mut profile in profiles::builtins() {
            let allowed = profile.id != "coordinator" && profile.id != "video-analyst";
            assert_eq!(check(&profile).is_ok(), allowed);
            assert_eq!(definitions(&profile).len(), if allowed { 3 } else { 0 });
            profile.tool_ids.retain(|id| id != "media-read");
            assert_eq!(check(&profile).unwrap_err()["code"], "FORBIDDEN");
            assert!(definitions(&profile).is_empty());
        }
    }
}
