use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use tauri::Manager;

const SKILLS: [(&str, &str, &str); 8] = [
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
pub fn catalog(root: &Path, setting: &str) -> Result<Value> {
    SKILLS
        .iter()
        .map(|(id, name, description)| {
            let path = root.join(id).join("SKILL.md");
            Ok(json!({"id":id,"name":name,"description":description,
            "available":path.is_file(),"enabled":enabled(setting,id)?,"path":path}))
        })
        .collect::<Result<Vec<_>>>()
        .map(|v| json!(v))
}
pub fn read(
    root: &Path,
    setting: &str,
    id: &str,
    resource: &str,
    offset: usize,
    active_only: bool,
) -> Result<Value> {
    ensure!(SKILLS.iter().any(|s| s.0 == id), "未安装此创作 skill");
    ensure!(!active_only || enabled(setting, id)?, "此 skill 已停用");
    let resource = resource.split('#').next().unwrap_or("SKILL.md");
    ensure!(
        !Path::new(resource).is_absolute(),
        "请使用 skill 内相对路径"
    );
    let path = root
        .join(id)
        .join(resource)
        .canonicalize()
        .context("规则文件不存在")?;
    ensure!(
        path.extension().and_then(|s| s.to_str()) == Some("md"),
        "仅能读取 Markdown 规则，不能运行脚本"
    );
    let owner = SKILLS
        .iter()
        .find(|s| {
            root.join(s.0)
                .canonicalize()
                .is_ok_and(|p| path.starts_with(p))
        })
        .context("规则路径超出已安装的创作 skills")?;
    ensure!(
        !active_only || enabled(setting, owner.0)? || dependency(id, owner.0),
        "引用的 skill 已停用"
    );
    ensure!(path.metadata()?.len() <= 200_000, "规则文件过大");
    let text = std::fs::read_to_string(&path)?;
    let chars: Vec<_> = text.chars().collect();
    let start = offset.min(chars.len());
    let end = (start + 4000).min(chars.len());
    let content: String = chars[start..end].iter().collect();
    let base = root.join(owner.0).canonicalize()?;
    let relative = path.strip_prefix(&base)?.to_string_lossy();
    Ok(
        json!({"skill":owner.0,"path":relative,"text":content,"offset":start,
        "nextOffset":if end < chars.len(){Some(end)}else{None},
        "totalCharacters":chars.len(),"resources":if relative == "SKILL.md" {resources(&base)}else{vec![]}}),
    )
}
fn resources(root: &Path) -> Vec<String> {
    let mut result = vec![];
    for dir in ["references", "assets"] {
        if let Ok(files) = std::fs::read_dir(root.join(dir)) {
            for file in files.flatten().take(100) {
                if file.path().extension().and_then(|s| s.to_str()) == Some("md") {
                    result.push(format!("{dir}/{}", file.file_name().to_string_lossy()));
                }
            }
        }
    }
    result.sort();
    result
}
#[tauri::command]
pub fn creative_skills(app: tauri::AppHandle) -> Result<Value, String> {
    (|| catalog(&root(&app)?, ""))().map_err(|e| e.to_string())
}
#[tauri::command]
pub fn read_creative_skill(
    app: tauri::AppHandle,
    id: String,
    path: Option<String>,
    offset: Option<usize>,
) -> Result<Value, String> {
    (|| {
        read(
            &root(&app)?,
            "",
            &id,
            path.as_deref().unwrap_or("SKILL.md"),
            offset.unwrap_or(0),
            false,
        )
    })()
    .map_err(|e| e.to_string())
}
pub fn tool(app: &tauri::AppHandle, setting: &str, args: &Value) -> Result<Value> {
    let root = root(app)?;
    if args["action"] == "skills" {
        return catalog(&root, setting);
    }
    read(
        &root,
        setting,
        args["skill"].as_str().context("请指定 skill")?,
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
        .map(|s| json!({"id":s["id"],"name":s["name"],"description":s["description"]}))
        .collect();
    format!(
        "\n已启用的 Mstudio 内置 skills：{}。以上仅为可用目录，不代表正文已加载。根据本轮任务和技能描述选择适用 Skill；用户明确指定时读取该 Skill。统筹、创意、脚本、分镜、制作、剪辑和审片等专业任务，先用 mstudio_read_skill, skill=ID, path=SKILL.md 读取适用规则，再开展对应工作；普通问答和不涉及专业方法的查询无需读取。不要仅因角色绑定就读取全部 Skills。若返回 nextOffset，继续读取主规则；之后仅在当前步骤需要时读取相关 references，不按角色预先加载，也不一次塞入全部文件。当前上下文已有完整规则时直接复用；仅有阅读记录或摘要时，按需重新读取所需规则。相对路径基于该 skill 根目录，跨技能链接也可按原路径读取；用 mstudio_skills 查看 Skill 目录，主规则读取结果中的 resources 列出可读参考文件。规则来自 Mstudio 仓库并随应用发布，独立于 Codex，遵循当前用户明确要求；商品技能的商品/市场偏好仅用于适用任务，不强加到通用视频。规则中项目文件、版本和审核记录对应当前工程、引用和聊天，不新增版本或审批面板；逐镜观看变化、机位、动作来源/支撑→路径/接触→去向、主体/相机速度与切口写入 shot 的 text，时长写 shot.duration，画格使用 shot.frames；不要发明字段。素材选择依据以镜头 ID、assetId 和源区间在交接中说明，实际剪辑使用 clip 的 trimIn/trimOut/start/speed。没有文件或 shell 工具时不尝试创建 PROJECT.md、review.json 或运行技能脚本，使用现有工程与消息记录同等证据；没有动态/音频输入就明确未验收。新委托不自动继承旧剧情与旧批准。当前用户已授权的范围继续有效。工具未提供的文件读写、浏览、图像生成、子 Agent 或 shell 能力不能假装执行，说明具体缺口并继续可做的部分。技能阅读不会调用视频模型或产生生成费用。",
        json!(available)
    )
}

// Read-only rule dependencies do not add tools or editing permissions.
fn dependency(from: &str, to: &str) -> bool {
    match from {
        "ad-script" => matches!(to, "creative-ad-director" | "product-storyboard"),
        "storyboard-art" => matches!(to, "creative-ad-director" | "product-storyboard"),
        "ad-team" => matches!(
            to,
            "creative-ad-director"
                | "product-storyboard"
                | "product-video-production"
                | "ad-script"
                | "storyboard-art"
        ),
        "creative-ad-director" => matches!(to, "product-storyboard" | "product-video-production"),
        "product-video-production" => matches!(to, "product-storyboard" | "creative-ad-director"),
        "product-storyboard" => matches!(to, "product-video-production" | "creative-ad-director"),
        _ => false,
    }
}
