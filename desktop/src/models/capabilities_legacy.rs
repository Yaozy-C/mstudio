use super::*;

const LEGACY: &[&str] = &[
    include_str!("../../../frontend/src/models/config/legacy/fal-ai_nano-banana-2.json"),
    include_str!("../../../frontend/src/models/config/legacy/fal-ai_nano-banana-2_edit.json"),
    include_str!("../../../frontend/src/models/config/legacy/minimax_h3-max_image-to-video.json"),
    include_str!(
        "../../../frontend/src/models/config/legacy/minimax_h3-max_reference-to-video.json"
    ),
    include_str!("../../../frontend/src/models/config/legacy/minimax_h3-max_text-to-video.json"),
    include_str!("../../../frontend/src/models/config/legacy/minimax_h3_image-to-video.json"),
    include_str!("../../../frontend/src/models/config/legacy/minimax_h3_reference-to-video.json"),
    include_str!("../../../frontend/src/models/config/legacy/minimax_h3_text-to-video.json"),
    include_str!("../../../frontend/src/models/config/legacy/openai_gpt-image-2.5_flare_edit.json"),
    include_str!(
        "../../../frontend/src/models/config/legacy/openai_gpt-image-2.5_flare_text-to-image.json"
    ),
    include_str!(
        "../../../frontend/src/models/config/legacy/openai_gpt-image-2.5_sunburst_edit.json"
    ),
    include_str!(
        "../../../frontend/src/models/config/legacy/openai_gpt-image-2.5_sunburst_text-to-image.json"
    ),
];

pub fn legacy_declaration(plugin: &str, endpoint: &str) -> Option<Capabilities> {
    if plugin != "fal" {
        return None;
    }
    LEGACY.iter().find_map(|data| {
        let record: serde_json::Value =
            serde_json::from_str(data).expect("valid legacy configuration");
        (record["endpoint"] == endpoint).then(|| {
            serde_json::from_value(record["capabilities"].clone())
                .expect("valid legacy capabilities")
        })
    })
}

/// Stamps declarations onto stored models once, then never runs again.
pub fn migrate(db: &rusqlite::Connection) -> Result<()> {
    if super::super::setting(db, MIGRATION_KEY)?.is_some() {
        return Ok(());
    }
    let mut models = super::super::media::read(db)?;
    let mut changed = false;
    for model in &mut models {
        if model.capabilities.is_some() {
            continue;
        }
        if let Some(declared) = legacy_declaration(&model.plugin, &model.endpoint) {
            model.capabilities = Some(declared);
            changed = true;
        }
    }
    if changed {
        super::super::media::write(db, &models)?;
    }
    db.execute(
        "INSERT INTO settings VALUES(?1,'1') ON CONFLICT(key) DO UPDATE SET value='1'",
        [MIGRATION_KEY],
    )?;
    Ok(())
}
