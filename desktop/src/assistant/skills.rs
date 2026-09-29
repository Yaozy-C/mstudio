mod prompt_migration;
mod prompt_scope;
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
        return storage::catalog(&db, setting);
    }
    storage::read(
        &db,
        setting,
        args["skill"].as_str().context("请指定 skill")?,
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
        .map(|s| json!({"id":s["id"],"name":s["name"],"description":s["description"]}))
        .collect();
    format!(
        "\n已启用的 Mstudio 内置 skills：{}。目录外的 CORE.md 是已自动加载的数据库核心规则；其余正文按需读取。根据本轮任务和技能描述选择适用 Skill；用户明确指定时读取该 Skill。已装配提示词 Skill 的 Agent 应用该 Skill 全文加载的依赖规则，未装配的角色不代写 Prompt；其他任务核心规则足够时直接执行；需要专业细则时用 mstudio_read_skill 读取对应 SKILL.md 或 references，普通问答无需读取。不要仅因角色绑定就读取全部 Skills。若返回 nextOffset，继续读取主规则；之后仅在当前步骤需要时读取相关 references，不按角色预先加载，也不一次塞入全部文件。当前上下文已有完整规则时直接复用；仅有阅读记录或摘要时，按需重新读取所需规则。相对路径基于该 skill 根目录，跨技能链接也可按原路径读取；用 mstudio_skills 查看 Skill 目录，主规则读取结果中的 resources 列出可读参考文件。规则正文以应用数据库为准，应用文件仅提供首次默认值，独立于 Codex，遵循当前用户明确要求；商品技能的商品/市场偏好仅用于适用任务，不强加到通用视频。规则中项目文件、版本和审核记录对应当前工程、引用和聊天，不新增版本或审批面板；逐镜观看变化、机位、动作来源/支撑→路径/接触→去向、主体/相机速度与切口写入 shot 的 text，时长写 shot.duration，画格使用 shot.frames；不要发明字段。素材选择依据以镜头 ID、assetId 和源区间在交接中说明，实际剪辑使用 clip 的 trimIn/trimOut/start/speed。没有文件或 shell 工具时不尝试创建 PROJECT.md、review.json 或运行技能脚本，使用现有工程与消息记录同等证据；没有动态/音频输入就明确未验收。新委托不自动继承旧剧情与旧批准。当前用户已授权的范围继续有效。工具未提供的文件读写、浏览、图像生成、子 Agent 或 shell 能力不能假装执行，说明具体缺口并继续可做的部分。技能阅读不会调用视频模型或产生生成费用。",
        json!(available)
    ) + "\n已自动加载的核心规则（无需重复读取）：\n"
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
