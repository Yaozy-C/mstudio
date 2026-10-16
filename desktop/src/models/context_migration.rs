use super::{context, setting};
use crate::assistant::config::Profile;
use anyhow::{Result, ensure};
use rusqlite::{Connection, params};
use serde_json::{Value, json};

pub fn validate_policy(profile: &Profile) -> Result<()> {
    let output = profile.limits.max_output_tokens;
    ensure!(
        profile.adapter != "codex" || output.is_none(),
        "Codex 的输出上限由本机服务管理"
    );
    ensure!(
        output.is_none_or(|n| (1..=2_000_000).contains(&n)),
        "输出上限须为 1 到 2000000 token"
    );
    ensure!(
        profile
            .limits
            .compaction_headroom
            .is_none_or(|n| n <= 2_000_000),
        "压缩余量须在 0 到 2000000 token 之间"
    );
    if let Some(maximum) = context::automatic(profile).and_then(|limits| limits.max_output_tokens) {
        ensure!(
            output.is_none_or(|n| n <= maximum),
            "输出上限超过此模型公布的输出能力"
        );
    }
    if let Some(window) = profile.context_window() {
        ensure!(
            context::threshold(profile, window).is_some(),
            "上下文窗口不足以容纳输出预留和压缩余量，请调整这三项设置"
        );
    }
    Ok(())
}

/// Repair only the old unset/32K defaults for verified official routes, once.
/// Keep original JSON and arbitrary fields, credentials, preferences and overrides.
pub fn migrate(db: &Connection) -> Result<()> {
    const KEY: &str = "text-context-window-v1";
    if setting(db, KEY)?.is_some() {
        return Ok(());
    }
    let rows = db
        .prepare("SELECT id,data FROM model_profiles")?
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut changes = Vec::new();
    for (id, raw) in rows {
        let mut value: Value = serde_json::from_str(&raw)?;
        let model: super::Model = serde_json::from_value(value.clone())?;
        let Some(limits) = context::known(&model.profile) else {
            continue;
        };
        if value
            .get("contextWindowMode")
            .is_some_and(|mode| !mode.is_null())
            || model
                .profile
                .context_window
                .is_some_and(|window| window != 32_768)
        {
            continue;
        }
        value["contextWindow"] = json!(limits.context_window);
        value["contextWindowMode"] = json!("auto");
        value["modelLimits"] = serde_json::to_value(limits)?;
        changes.push((id, raw, value.to_string()));
    }
    db.execute_batch("SAVEPOINT text_context_migration")?;
    let result = (|| -> Result<()> {
        let backup: Vec<_> = changes
            .iter()
            .map(|(id, raw, _)| json!({"id":id,"data":raw}))
            .collect();
        db.execute(
            "INSERT INTO settings(key,value) VALUES('text-context-window-v1-backup',?1)",
            [serde_json::to_string(&backup)?],
        )?;
        for (id, _, updated) in &changes {
            db.execute(
                "UPDATE model_profiles SET data=?2 WHERE id=?1",
                params![id, updated],
            )?;
        }
        db.execute("INSERT INTO settings(key,value) VALUES(?1,'done')", [KEY])?;
        Ok(())
    })();
    if result.is_err() {
        db.execute_batch("ROLLBACK TO text_context_migration; RELEASE text_context_migration")?;
    } else {
        db.execute_batch("RELEASE text_context_migration")?;
    }
    result
}
