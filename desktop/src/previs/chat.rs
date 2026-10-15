use super::{File, files};
use crate::{database::Store, project_storage};
use anyhow::{Context, Result, ensure};
use rusqlite::params;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::HashMap, sync::Mutex};
use tauri::{Manager, State};
use tokio_util::sync::CancellationToken;

const CONTRACT: &str = r#"You edit one Three.js previsualization file in a plain chat. No tools, project Agent, Skills or task loops exist here. Return a JSON object, no markdown:
{"message":"brief explanation in user's language","code":"complete updated JS module, or null for explanation only","assetNames":["existing model ID or name"],"newModels":[{"name":"unique descriptive name","description":"dimensions, parts and usage","code":"export function createModel({THREE}) {... return a THREE.Group;}"}]}
Models are named reusable JavaScript factories, not complete scenes. Prefer project models from the inventory. Never alter an existing shared model while editing a shot. If insufficient, add a distinctly named reusable model through newModels. When editing a model, return its updated factory in code and no newModels; optionally return description with the full updated dimensions, units and named moving parts so other shots can use the model without reading its code. Do not invent product geometry/claims as verified facts; mark rough geometry in description.
Shot interface: export async function createShot({THREE,models,seed}) { const scene=new THREE.Scene();const camera=new THREE.PerspectiveCamera(...);const bag=models.create('existing ID or name');scene.add(bag);return {scene,camera,update(t){/*absolute seconds, deterministic, reset every affected property*/},dispose(){}}; }
Model interface: export function createModel({THREE}) { const root=new THREE.Group(); /*geometry, materials*/ root.userData.parts={lid:someGroup};return root; }. Publish movable parts in userData.parts and describe them. models.create produces a fresh instance on every call. The shot owns assembly, poses, animation and camera; put reusable product/character geometry into models. assetNames must list every used model (including newly created names). Names bind to stable IDs on save. Geometry uses meters, right-handed Y-up; t is seconds. Models already exist: include code only for the current file, never repeat Three.js library or old file versions. No imports from CDN, npm, local filesystem; THREE and models are injected. No DOM/UI, renderer, RAF loop, export/network code: Runtime owns these. seed is fixed; use a deterministic generator for randomness. Ensure camera aspect is set by Runtime and all animation can seek backwards. Render clean frames with no UI overlays. Stay within given duration. Every response is one edit, no automatic retries. For small changes you may replace code with null and provide patches:[{before:'unique exact existing substring',after:'replacement'}]. Do not use code and patches together."#;

#[derive(Default)]
pub struct Chats(pub Mutex<HashMap<String, CancellationToken>>);
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub project_id: String,
    pub file_id: String,
    pub expected_revision: u64,
    pub prompt: String,
    pub model_id: String,
    pub request_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Reply {
    message: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    patches: Vec<Patch>,
    #[serde(default)]
    asset_names: Vec<String>,
    #[serde(default)]
    new_models: Vec<NewModel>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Patch {
    before: String,
    after: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NewModel {
    name: String,
    description: String,
    code: String,
}
fn parse(text: &str) -> Result<Reply> {
    let s = text.trim();
    let s = if s.starts_with("```json\n") {
        s.strip_prefix("```json\n")
            .and_then(|s| s.strip_suffix("```"))
            .context("JSON 回复不完整")?
            .trim()
    } else if s.starts_with("```\n") {
        s.strip_prefix("```\n")
            .and_then(|s| s.strip_suffix("```"))
            .context("JSON 回复不完整")?
            .trim()
    } else {
        s
    };
    let r: Reply =
        serde_json::from_str(s).context("模型回复格式无效，文件未修改；可重新发送要求")?;
    ensure!(
        r.message.len() <= 8000 && r.new_models.len() <= 10 && r.patches.len() <= 30,
        "回复过长"
    );
    ensure!(
        r.code.is_none() || r.patches.is_empty(),
        "回复不能同时包含完整代码和补丁"
    );
    Ok(r)
}
fn apply_patches(code: &str, patches: &[Patch]) -> Result<String> {
    let mut next = code.to_owned();
    for p in patches {
        ensure!(
            !p.before.is_empty() && next.matches(&p.before).count() == 1,
            "补丁目标不唯一，文件未修改"
        );
        next = next.replacen(&p.before, &p.after, 1);
    }
    Ok(next)
}
#[tauri::command]
pub fn cancel_previs_chat(chats: State<Chats>, request_id: String) {
    if let Some(t) = chats.0.lock().unwrap().get(&request_id) {
        t.cancel();
    }
}
#[tauri::command]
pub async fn previs_chat(app: tauri::AppHandle, request: Request) -> Result<Value, String> {
    super::identifier(&request.request_id).map_err(|e| e.to_string())?;
    let token = CancellationToken::new();
    {
        let state = app.state::<Chats>();
        let mut active = state.0.lock().unwrap();
        if active.contains_key(&request.request_id) {
            return Err("请求已在处理中".into());
        }
        active.insert(request.request_id.clone(), token.clone());
    }
    let result = run(&app, &request, &token).await.map_err(|e| e.to_string());
    app.state::<Chats>()
        .0
        .lock()
        .unwrap()
        .remove(&request.request_id);
    result
}
async fn run(app: &tauri::AppHandle, r: &Request, token: &CancellationToken) -> Result<Value> {
    ensure!(!r.prompt.trim().is_empty(), "请填写修改要求");
    let store = app.state::<Store>();
    // Do not hold a storage lock during a network completion. Recheck at commit.
    let (file, all, history, model, key) = {
        let db = store.db.lock().unwrap();
        let file = files::read(&db, &r.project_id, &r.file_id)?.context("请先创建预演文件")?;
        ensure!(
            file.revision == r.expected_revision,
            "文件已有新版本，请刷新"
        );
        let all = files::list(&db, &r.project_id)?;
        let history=db.prepare("SELECT role,text FROM previs_messages WHERE project_id=?1 AND file_id=?2 ORDER BY id DESC LIMIT 6")?
            .query_map(params![r.project_id,r.file_id],|row|Ok(json!({"role":row.get::<_,String>(0)?,"text":row.get::<_,String>(1)?})))?.collect::<rusqlite::Result<Vec<_>>>()?;
        let (model, key) = crate::models::resolve(&db, None, Some(&r.model_id))?;
        (file, all, history, model, key)
    };
    let inventory = all
        .iter()
        .filter(|f| f.kind == "model")
        .map(|m| json!({"id":m.id,"name":m.name,"description":m.description,"revision":m.revision}))
        .collect::<Vec<_>>();
    let context=json!({"currentFile":file,"modelInventory":inventory,"recentConversation":history.into_iter().rev().collect::<Vec<_>>(),"request":r.prompt}).to_string();
    let raw = super::provider::complete(&model, &key, CONTRACT, &context, token).await?;
    let reply = parse(&raw)?;
    let _guard = project_storage::working(&store, &r.project_id).await?;
    ensure!(!token.is_cancelled(), "预演 Chat 已停止，文件未修改");
    let mut db = store.db.lock().unwrap();
    let tx = db.transaction()?;
    let current = files::read(&tx, &r.project_id, &r.file_id)?.context("文件已删除")?;
    ensure!(
        current.revision == file.revision,
        "生成期间文件已修改，本次回复未覆盖新版本"
    );
    ensure!(
        file.kind == "shot" || reply.new_models.is_empty(),
        "编辑模型时不能新增其他模型"
    );
    let mut updated = current;
    let mut new_models = Vec::new();
    for new in reply.new_models {
        let m = File {
            id: mstudio::media::id(),
            kind: "model".into(),
            name: new.name,
            description: new.description,
            code: new.code,
            revision: 0,
            asset_ids: vec![],
            bindings: Default::default(),
            duration: 5.,
            width: 720,
            height: 720,
            fps: 24,
        };
        ensure!(!m.code.trim().is_empty(), "新增模型代码不能为空");
        new_models.push(files::save(&tx, &store, &r.project_id, m, 0)?);
    }
    if let Some(description) = reply.description {
        ensure!(file.kind == "model", "镜头修改不能重写共用模型说明");
        updated.description = description;
    }
    let changed = reply.code.is_some()
        || !reply.patches.is_empty()
        || updated.description != file.description;
    if changed {
        updated.code = if let Some(code) = reply.code {
            code
        } else {
            apply_patches(&file.code, &reply.patches)?
        };
        for id in &updated.asset_ids {
            if let Some(old) = all.iter().find(|asset| asset.id == *id) {
                ensure!(
                    files::read(&tx, &r.project_id, id)?
                        .is_some_and(|asset| asset.revision == old.revision),
                    "生成期间共用模型已修改，请重新发送"
                );
            }
        }
        ensure!(!updated.code.trim().is_empty(), "镜头代码不能为空");
        if updated.kind == "shot" && (!reply.asset_names.is_empty() || !new_models.is_empty()) {
            let models = files::list(&tx, &r.project_id)?;
            let mut ids = Vec::new();
            let mut bindings = updated.bindings.clone();
            for name in reply.asset_names {
                let asset = models
                    .iter()
                    .find(|a| {
                        a.kind == "model"
                            && (a.id == name
                                || a.name == name
                                || bindings.get(&name) == Some(&a.id))
                    })
                    .with_context(|| format!("找不到模型：{name}"))?;
                if let Some(old) = all.iter().find(|a| a.id == asset.id) {
                    ensure!(
                        old.revision == asset.revision,
                        "生成期间共用模型已修改，请重新发送"
                    );
                }
                ids.push(asset.id.clone());
                bindings.insert(name, asset.id.clone());
            }
            bindings.retain(|_, id| ids.contains(id));
            updated.asset_ids = ids;
            updated.bindings = bindings;
        }
        updated = files::save(&tx, &store, &r.project_id, updated, file.revision)?;
    } else {
        ensure!(new_models.is_empty(), "新增模型时须同时返回镜头代码");
    }
    tx.execute(
        "INSERT INTO previs_messages(project_id,file_id,role,text) VALUES(?1,?2,'user',?3)",
        params![r.project_id, r.file_id, r.prompt],
    )?;
    tx.execute(
        "INSERT INTO previs_messages(project_id,file_id,role,text) VALUES(?1,?2,'assistant',?3)",
        params![r.project_id, r.file_id, reply.message],
    )?;
    tx.execute("DELETE FROM previs_messages WHERE project_id=?1 AND file_id=?2 AND id NOT IN (SELECT id FROM previs_messages WHERE project_id=?1 AND file_id=?2 ORDER BY id DESC LIMIT 100)",params![r.project_id,r.file_id])?;
    tx.commit()?;
    Ok(json!({"file":updated,"message":reply.message,"newModels":new_models}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn patches_require_unique_exact_matches_and_preserve_other_code() {
        let next = apply_patches(
            "const a=1; const b=2;",
            &[Patch {
                before: "a=1".into(),
                after: "a=3".into(),
            }],
        )
        .unwrap();
        assert_eq!(next, "const a=3; const b=2;");
        assert!(
            apply_patches(
                "x x",
                &[Patch {
                    before: "x".into(),
                    after: "y".into()
                }]
            )
            .is_err()
        );
        assert!(
            parse(r#"{"message":"ok","code":"x","patches":[{"before":"a","after":"b"}]}"#).is_err()
        );
        assert!(parse(r#"{"message":"ok","operations":[{"op":"update_node"}]}"#).is_err());
    }
}
