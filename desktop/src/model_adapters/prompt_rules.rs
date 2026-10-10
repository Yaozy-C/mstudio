//! Model-owned prompting guidance, separate from creative Skills and credentials.
use crate::models::media::MediaModel;
use anyhow::Result;
use rusqlite::{Connection, OptionalExtension};
use serde_json::{Value, json};

pub fn init(db: &Connection) -> Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS model_prompt_rules(id TEXT PRIMARY KEY,text TEXT NOT NULL,revision INTEGER NOT NULL DEFAULT 1)")?;
    Ok(())
}
pub fn guidance(db: &Connection, model: &MediaModel) -> Result<Value> {
    let config: Value = serde_json::from_str(include_str!(
        "../../../frontend/src/models/config/prompt-rules.json"
    ))?;
    let binding = config["bindings"].as_array().and_then(|entries| {
        entries.iter().find(|entry| {
            entry["plugin"] == model.plugin
                && entry.get("endpoint").is_none_or(|v| v == &model.endpoint)
        })
    });
    let rule_id = binding
        .and_then(|b| b["ruleId"].as_str())
        .unwrap_or("default");
    let input_description = binding
        .and_then(|b| b["inputDescription"].as_str())
        .unwrap_or(&model.kind);
    let rules: String = db
        .query_row(
            "SELECT text FROM model_prompt_rules WHERE id=?1",
            [rule_id],
            |row| row.get(0),
        )
        .optional()?
        .unwrap_or_else(|| config["rules"][rule_id].as_str().unwrap_or_default().into());
    Ok(
        json!({"modelId":model.id,"kind":model.kind,"provider":model.plugin,"endpoint":model.endpoint,"inputDescription":input_description,"rules":rules,"capabilities":crate::project_service::runtime::execute(json!({"action":"capabilities","model":model}))?}),
    )
}
