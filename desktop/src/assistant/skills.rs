mod catalog;
mod metadata;
use metadata::{model_catalog, model_metadata};
pub mod storage;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use tauri::Manager;

/// The shipped creative catalog. Each Skill is one capability with a short entry
/// document and optional method references.
const SKILLS: [(&str, &str, &str); 5] = [
    (
        "ad-script",
        "创意与声画脚本",
        "内容优先的创意、观看回报、声画脚本与本地研究",
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
        "Available Skills: {}. This catalog contains descriptions, not loaded instructions. Before specialized work, load the relevant SKILL.md with mstudio_read_skill; each read returns the complete remaining document. Choose it from the task in front of you: writing or repairing content reads the scriptwriting Skill, deciding shots, staging, on-camera behaviour, photographic appearance or pacing reads the direction Skill, producing stills or image prompts reads image production, writing a video prompt or generating and judging a clip reads video production, and cutting, grading or joining shots reads editing. A task involving real people, a limited duration with several events, or believable skin and light is already a reason to read the matching method, not only its entry document. Read linked references only to resolve a concrete task need and reuse full text already in context. Skill methods do not require separate Agents or expand tool permissions. Choose delegation by task needs, not professional titles in a guide. Database documents are authoritative; mstudio_skills lists resources. Use current tool schemas for writes, not legacy examples in documents. Reading a Skill does not submit generation or authorize actions.",
        json!(available)
    )
}

// Read-only rule dependencies do not add tools or editing permissions.
fn dependency(from: &str, to: &str) -> bool {
    match from {
        "ad-script" => to == "creative-ad-director",
        "image-production" => to == "creative-ad-director",
        "product-video-production" => matches!(to, "creative-ad-director" | "image-production"),
        "video-editing" => matches!(to, "creative-ad-director" | "product-video-production"),
        "creative-ad-director" => matches!(
            to,
            "ad-script" | "image-production" | "product-video-production" | "video-editing"
        ),
        _ => false,
    }
}
