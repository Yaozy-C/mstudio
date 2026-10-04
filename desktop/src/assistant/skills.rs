mod contract_migration;
mod metadata;
use metadata::{model_catalog, model_metadata};
mod prompt_migration;
mod prompt_scope;
mod resource_upgrade;
pub mod storage;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use tauri::Manager;

const SKILLS: [(&str, &str, &str); 11] = [
    ("image-prompt", "图片提示词", "画面、姿态、支撑与参考转写"),
    ("video-prompt", "视频提示词", "动作、节奏、接触与声音转写"),
    (
        "asset-preparation",
        "参考素材",
        "白底人物、服装、商品与道具参考",
    ),
    ("color-grading", "专业调色", "色彩校正、镜头匹配和工具边界"),
    ("transition-design", "转场设计", "动作匹配、节奏和接缝转场"),
    (
        "ad-script",
        "创意与声画脚本",
        "创意、可见事件、画面文字与声音脚本",
    ),
    (
        "storyboard-art",
        "分镜画格",
        "静态构图、图片提示词与局部修图",
    ),
    (
        "product-storyboard",
        "脚本与分镜",
        "视频意图、故事、动作节拍与对应分镜",
    ),
    (
        "creative-ad-director",
        "创意导演",
        "创意广告、摄影运镜与模型提示转换",
    ),
    (
        "product-video-production",
        "视频制作",
        "参考素材、分段制作、局部修复与成片检查",
    ),
    ("ad-team", "团队统筹", "专业角色协作、任务交接与局部修订"),
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
    storage::install_asset_defaults(&db)?;
    storage::install_core_defaults(&db)?;
    prompt_migration::migrate(&db)?;
    prompt_scope::install(&db)?;
    contract_migration::migrate(&db)?;
    resource_upgrade::migrate(&db, &root(app)?)?;
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
    let core: Vec<_> = catalog
        .as_array()
        .into_iter()
        .flatten()
        .filter(|s| s["available"] == true && s["enabled"] == true)
        .filter_map(|s| {
            s["core"].as_str().map(|text| {
                format!(
                    "{} / CORE.md:\n{text}",
                    s["id"].as_str().unwrap_or_default()
                )
            })
        })
        .collect();
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
        "Enabled Mstudio Skills: {}. CORE.md bodies below are already loaded from the application database. Apply fully injected dependencies for assigned prompt-authoring Skills; other roles do not inherit prompt-authoring duties. Choose additional reading by the current task and Skill description, not merely role assignment. Read explicitly requested Skills unless their current full text is already present. Use mstudio_read_skill for SKILL.md or references only when core rules are insufficient; ordinary questions need no Skill read. Follow nextOffset to finish a needed document, then load only relevant references. A read receipt or summary is not the full rule text. Paths are relative to the Skill root; permitted cross-Skill links are supported. mstudio_skills lists resources. Database bodies are authoritative; bundled files seed defaults and are independent of Codex. Apply product/market preferences only to relevant tasks. Map document/revision/review instructions to existing project objects, references and chat; do not invent files, fields or approval panels. Use the current role’s tool schemas for all write parameters. Project record paths and legacy operation examples in Skills are data descriptions, not tool argument schemas or permission grants. Hand off source choices using shot/asset IDs and ranges. Without file or shell tools, use project records instead of creating PROJECT.md/review.json or running scripts. Without motion/audio evidence, state what remains unchecked. A new brief does not inherit an old story or approval, while current explicit authorization remains valid. Do not claim unavailable browsing, generation, delegation or file operations; report concrete gaps and continue supported work. Reading Skills does not submit paid generation tasks.",
        json!(available)
    ) + "\nAlready loaded core rules (do not reread unchanged bodies):\n"
        + &core.join("\n\n")
}

// Read-only rule dependencies do not add tools or editing permissions.
fn dependency(from: &str, to: &str) -> bool {
    match from {
        "image-prompt" | "video-prompt" => matches!(to, "creative-ad-director"),
        "asset-preparation" => matches!(to, "creative-ad-director"),
        "ad-script" => matches!(to, "creative-ad-director" | "product-storyboard"),
        "storyboard-art" => matches!(to, "creative-ad-director" | "product-storyboard"),
        "ad-team" => matches!(
            to,
            "creative-ad-director"
                | "product-storyboard"
                | "product-video-production"
                | "ad-script"
                | "storyboard-art"
                | "asset-preparation"
        ),
        "creative-ad-director" => matches!(to, "product-storyboard" | "product-video-production"),
        "product-video-production" => matches!(to, "product-storyboard" | "creative-ad-director"),
        "product-storyboard" => matches!(to, "product-video-production" | "creative-ad-director"),
        _ => false,
    }
}
