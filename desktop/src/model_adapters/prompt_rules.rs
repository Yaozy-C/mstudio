//! Model-owned prompting guidance, separate from creative Skills and credentials.
use crate::models::media::MediaModel;
use anyhow::Result;
use rusqlite::{Connection, OptionalExtension};
use serde_json::{Value, json};

pub const H3: &str = include_str!("prompt_rules/h3.md");
pub fn init(db: &Connection) -> Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS model_prompt_rules(id TEXT PRIMARY KEY,text TEXT NOT NULL,revision INTEGER NOT NULL DEFAULT 1)")?;
    Ok(())
}
pub fn guidance(db: &Connection, model: &MediaModel) -> Result<Value> {
    let (mode, rules) = match (model.plugin.as_str(), model.endpoint.as_str()) {
        ("fal", endpoint @ ("minimax/h3/text-to-video" | "minimax/h3/image-to-video" | "minimax/h3/reference-to-video")) => {
            let body: String = db.query_row("SELECT text FROM model_prompt_rules WHERE id='minimax-h3'", [], |r| r.get(0)).optional()?.unwrap_or_else(|| H3.into());
            let mode = match endpoint {
                "minimax/h3/text-to-video" => "text-to-video: no reference inputs",
                "minimax/h3/image-to-video" => "image-to-video: image_url is the actual first frame; end_image_url is the optional last frame",
                _ => "full-reference: reference_image_urls are appearance/composition references, not actual first-frame controls",
            };
            (mode, body)
        }
        ("codex-image", _) => ("image", "Write an image prompt for the configured Codex image model. Explicit ordered images are reference/edit inputs; distinguish edits from new scenes. Use the adapter's exposed image parameters; do not invent video, audio or frame-control inputs. The configured display name does not prove a specific underlying image model version.".into()),
        ("gemini-native", _) => ("image", "Write an image prompt for the configured Gemini image model. Describe the visible output and ordered reference roles. Use the selected model's supported image parameters; do not use video shot timestamps, sound fields or first/last-frame syntax.".into()),
        _ => (model.kind.as_str(), "No model-specific prompt grammar is registered for this endpoint. Use model-independent creative description and only capabilities exposed by its adapter. Do not borrow another model's syntax or infer controls from its name.".into()),
    };
    Ok(
        json!({"modelId":model.id,"kind":model.kind,"provider":model.plugin,"endpoint":model.endpoint,"mode":mode,"rules":rules}),
    )
}
