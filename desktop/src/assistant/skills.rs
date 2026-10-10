mod catalog;
mod concept_split;
mod isolation;
mod metadata;
mod rule_summary;
mod storyboard;
use metadata::{model_catalog, model_metadata};
pub mod storage;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use tauri::Manager;

/// The shipped creative catalog. Each Skill is one capability with a short entry
/// document and optional method references.
const SKILLS: [(&str, &str, &str); 9] = [
    (
        "video-analysis",
        "视频拆解",
        "原视频的声画记录、节奏、结构与表达机制",
    ),
    (
        "creative-concepts",
        "创意策划",
        "选题、观看动机、核心事件、商品关系与方向比较",
    ),
    (
        "ad-script",
        "声画编剧",
        "将选定方向写成动作、台词、文字、声音与段落时长",
    ),
    (
        "creative-ad-director",
        "创意导演",
        "观看主张、分镜设计、真人表演、摄影外观与节奏",
    ),
    (
        "image-production",
        "图片制作",
        "图片提示词、分镜画格、参考素材与画面检查",
    ),
    (
        "product-video-production",
        "视频制作",
        "视频提示词、输入用途、生成修复与成片判断",
    ),
    (
        "storyboard-image-production",
        "动作分镜板制作",
        "每镜头四格动作板、状态连续性与图片检查",
    ),
    (
        "storyboard-video-production",
        "分镜板视频制作",
        "整板参考输入、动作顺序、连续运动与视频检查",
    ),
    (
        "video-editing",
        "剪辑与后期",
        "选段、节奏、调色、转场与声音",
    ),
];
pub fn root(app: &tauri::AppHandle) -> Result<PathBuf> {
    let base = if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
    } else {
        app.path().resource_dir()?
    };
    directory(&base)
}

pub(super) fn directory(base: &Path) -> Result<PathBuf> {
    let root = base.join("skills");
    ensure!(
        root.is_dir(),
        "Mstudio 内置技能资源缺失，请重新构建或安装应用"
    );
    Ok(root)
}

fn enabled(setting: &str, id: &str) -> Result<bool> {
    if setting.is_empty() {
        return Ok(true);
    }
    let ids: Vec<String> = serde_json::from_str(setting)?;
    Ok(ids.iter().any(|s| s == &format!("skill-{id}")))
}
pub fn initialize(app: &tauri::AppHandle) -> Result<()> {
    let store = app.state::<crate::database::Store>();
    let db = store.db.lock().unwrap();
    storage::init(&db)?;
    if !storage::initialized(&db)? {
        storage::seed(&db, &root(app)?)?;
    }
    catalog::migrate(&db, &root(app)?)?;
    concept_split::migrate(&db, &root(app)?)?;
    isolation::migrate(&db, &root(app)?)?;
    rule_summary::migrate(&db, &root(app)?)?;
    rule_summary::migrate_methods(&db, &root(app)?)?;
    rule_summary::migrate_video_motion(&db, &root(app)?)?;
    rule_summary::migrate_image_storyboard(&db, &root(app)?)?;
    rule_summary::migrate_image_cases(&db, &root(app)?)?;
    storyboard::migrate(&db, &root(app)?)?;
    rule_summary::migrate_board_regeneration(&db, &root(app)?)?;
    rule_summary::migrate_pure_rules(&db, &root(app)?)?;
    rule_summary::migrate_object_tools(&db, &root(app)?)?;
    super::video_analysis::install(&db, &root(app)?)?;
    Ok(())
}
pub fn runtime_catalog(app: &tauri::AppHandle, setting: &str) -> Result<Value> {
    initialize(app)?;
    storage::catalog(
        &app.state::<crate::database::Store>().db.lock().unwrap(),
        setting,
    )
}
#[tauri::command]
pub fn creative_skills(app: tauri::AppHandle) -> Result<Value, String> {
    runtime_catalog(&app, "").map_err(|e| e.to_string())
}
#[tauri::command]
pub fn read_creative_skill(
    app: tauri::AppHandle,
    id: String,
    path: Option<String>,
    offset: Option<usize>,
) -> Result<Value, String> {
    (|| {
        initialize(&app)?;
        storage::read(
            &app.state::<crate::database::Store>().db.lock().unwrap(),
            "",
            &id,
            path.as_deref().unwrap_or("SKILL.md"),
            offset.unwrap_or(0),
            false,
        )
    })()
    .map_err(|e: anyhow::Error| e.to_string())
}
#[tauri::command]
pub fn save_creative_skill(
    app: tauri::AppHandle,
    id: String,
    path: String,
    text: String,
    revision: i64,
) -> Result<Value, String> {
    (|| {
        initialize(&app)?;
        storage::save(
            &app.state::<crate::database::Store>().db.lock().unwrap(),
            &id,
            &path,
            &text,
            revision,
        )
    })()
    .map_err(|e: anyhow::Error| e.to_string())
}
pub fn tool(app: &tauri::AppHandle, setting: &str, args: &Value) -> Result<Value> {
    initialize(app)?;
    let store = app.state::<crate::database::Store>();
    let db = store.db.lock().unwrap();
    if args["action"] == "skills" {
        return storage::catalog(&db, setting).map(model_catalog);
    }
    storage::read(
        &db,
        setting,
        args["skill"].as_str().context("Specify skill")?,
        args["path"].as_str().unwrap_or("SKILL.md"),
        args["offset"].as_u64().unwrap_or(0).min(200_000) as usize,
        true,
    )
}
pub fn guidance(catalog: &Value) -> String {
    let available: Vec<_> = catalog
        .as_array()
        .into_iter()
        .flatten()
        .filter(|s| s["available"] == true && s["enabled"] == true)
        .map(|s| {
            let s = model_metadata(s.clone());
            json!({"id":s["id"],"name":s["name"],"description":s["description"]})
        })
        .collect();
    format!(
        "Available Skills: {}. This catalog contains only this role's bound Skills, with descriptions rather than loaded instructions. Use the relevant SKILL.md for specialized work; load it with mstudio_read_skill only when its current full text is missing. Each read returns the complete remaining document and its own resources. Read local references only for a concrete task need; reuse current full text already in context. Each Skill is an isolated root: cross-directory paths are forbidden, even when another Skill is also bound. For work outside this role's capability, hand off the required task and project evidence to the responsible role instead of loading its Skill. Skills define domain constraints, not mandatory inspection, audit or approval workflows. Do not infer extra tool calls from a rule. Skill methods do not expand tool permissions or authorize generation. Database documents are authoritative. Use current tool schemas for writes, not legacy examples in documents.",
        json!(available)
    )
}
