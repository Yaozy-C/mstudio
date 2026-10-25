//! Initial output names come from the generation task.
use anyhow::{Context, Result};
use rusqlite::Connection;
use serde_json::{Value, json};

pub fn result_name(db: &Connection, job: &Value) -> Result<String> {
    let project = job["projectId"].as_str().context("任务缺少项目")?;
    let document = super::load(db, project)?;
    let value =
        super::runtime::execute(json!({"action":"result_name","document":document,"job":job}))?;
    if let Some(error) = value.get("error") {
        anyhow::bail!("生成结果命名失败：{error}");
    }
    value["name"]
        .as_str()
        .map(str::to_owned)
        .context("生成结果缺少任务名称")
}
