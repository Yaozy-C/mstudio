use super::runtime::{RuntimeState, Snapshot};
use crate::{database::Store, project_storage};
use anyhow::{Context, Result, ensure};
use mstudio::{media, model::Asset};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::{
    Manager,
    ipc::{InvokeBody, Request},
};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Export {
    pub id: String,
    pub file_id: String,
    pub file_revision: u64,
    pub model_revisions: std::collections::BTreeMap<String, u64>,
    pub signature: String,
    pub asset: Asset,
    pub shared_asset_id: Option<String>,
}
struct Job {
    snapshot: Snapshot,
    work: PathBuf,
    received: usize,
    count: usize,
    token: CancellationToken,
    finishing: bool,
}
#[derive(Default)]
pub struct Renders(Mutex<HashMap<String, Arc<Mutex<Job>>>>);
fn get(state: &Renders, id: &str, project: &str) -> Result<Arc<Mutex<Job>>> {
    let job = state
        .0
        .lock()
        .unwrap()
        .get(id)
        .cloned()
        .context("预演导出已结束")?;
    ensure!(
        job.lock().unwrap().snapshot.project == project,
        "导出不属于当前项目"
    );
    Ok(job)
}
#[tauri::command]
pub async fn begin_previs_render(
    app: tauri::AppHandle,
    project_id: String,
    session_id: String,
) -> Result<serde_json::Value, String> {
    let store = app.state::<Store>();
    let _guard = project_storage::working(&store, &project_id)
        .await
        .map_err(|e| e.to_string())?;
    (|| -> Result<_> {
        let snapshot = app
            .state::<RuntimeState>()
            .service()?
            .get(&session_id, &project_id)?;
        ensure!(snapshot.target.kind == "shot", "请选择镜头再导出视频");
        // The runtime session is the frozen specification, including model revisions.
        let count = (snapshot.target.duration * snapshot.target.fps as f64).ceil() as usize;
        ensure!(count > 0, "预演帧数无效");
        let state = app.state::<Renders>();
        let mut jobs = state.0.lock().unwrap();
        ensure!(jobs.len() < 4, "最多同时导出 4 个预演");
        let id = uuid::Uuid::new_v4().to_string();
        let work = store
            .media_root()
            .join("render-work")
            .join(format!("previs-{id}"));
        project_storage::track_file(&store, &project_id, &work)?;
        std::fs::create_dir_all(work.join("frames"))?;
        std::fs::write(work.join("snapshot.json"), serde_json::to_vec(&snapshot)?)?;
        jobs.insert(
            id.clone(),
            Arc::new(Mutex::new(Job {
                snapshot,
                work,
                received: 0,
                count,
                token: CancellationToken::new(),
                finishing: false,
            })),
        );
        Ok(serde_json::json!({"id":id,"frameCount":count}))
    })()
    .map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn append_previs_frame(
    app: tauri::AppHandle,
    request: Request<'_>,
) -> Result<(), String> {
    let header = |k| {
        request
            .headers()
            .get(k)
            .and_then(|v| v.to_str().ok())
            .ok_or("预演帧参数缺失")
    };
    let project = header("x-project-id")?.to_owned();
    let id = header("x-render-id")?.to_owned();
    let index = header("x-frame-index")?
        .parse::<usize>()
        .map_err(|_| "帧序号无效")?;
    let InvokeBody::Raw(bytes) = request.body() else {
        return Err("预演帧内容无效".into());
    };
    if bytes.len() < 24 || bytes.len() > 8 * 1024 * 1024 {
        return Err("预演帧大小无效".into());
    }
    let bytes = bytes.clone();
    let store = app.state::<Store>();
    let guard = project_storage::working(&store, &project)
        .await
        .map_err(|e| e.to_string())?;
    let job = get(&app.state::<Renders>(), &id, &project).map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let _guard = guard;
        (|| -> Result<()> {
            let mut j = job.lock().unwrap();
            ensure!(
                !j.token.is_cancelled() && !j.finishing,
                "导出已停止或开始编码"
            );
            ensure!(j.received == index && index < j.count, "预演帧顺序不一致");
            ensure!(
                bytes.starts_with(b"\x89PNG\r\n\x1a\n") && &bytes[12..16] == b"IHDR",
                "不是有效 PNG 帧"
            );
            ensure!(
                u32::from_be_bytes(bytes[16..20].try_into()?) == j.snapshot.target.width
                    && u32::from_be_bytes(bytes[20..24].try_into()?) == j.snapshot.target.height,
                "预演帧尺寸不一致"
            );
            std::fs::write(j.work.join("frames").join(format!("{index:06}.png")), bytes)?;
            j.received += 1;
            Ok(())
        })()
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn cancel_previs_render(
    app: tauri::AppHandle,
    project_id: String,
    id: String,
) -> Result<(), String> {
    let job = get(&app.state::<Renders>(), &id, &project_id).map_err(|e| e.to_string())?;
    let (work, finishing) = {
        let j = job.lock().unwrap();
        j.token.cancel();
        (j.work.clone(), j.finishing)
    };
    if !finishing {
        app.state::<Renders>().0.lock().unwrap().remove(&id);
        let _ = std::fs::remove_dir_all(work);
    }
    Ok(())
}
#[tauri::command]
pub async fn finish_previs_render(
    app: tauri::AppHandle,
    project_id: String,
    id: String,
) -> Result<Export, String> {
    let store = app.state::<Store>();
    let guard = project_storage::working(&store, &project_id)
        .await
        .map_err(|e| e.to_string())?;
    let job = get(&app.state::<Renders>(), &id, &project_id).map_err(|e| e.to_string())?;
    let (snapshot, work, token) = {
        let mut j = job.lock().unwrap();
        if j.received != j.count || j.finishing {
            return Err("预演帧尚未完整，或正在编码".into());
        }
        j.finishing = true;
        (j.snapshot.clone(), j.work.clone(), j.token.clone())
    };
    let output_dir = store
        .media_root()
        .join("exports")
        .join(format!("previs-{id}"));
    let result = encode(&app, &id, &snapshot, &work, &output_dir, &token)
        .await
        .map_err(|e| e.to_string());
    drop(guard);
    app.state::<Renders>().0.lock().unwrap().remove(&id);
    let _ = std::fs::remove_dir_all(&work);
    if result.is_err() {
        let _ = std::fs::remove_dir_all(output_dir);
    }
    result
}
async fn encode(
    app: &tauri::AppHandle,
    id: &str,
    s: &Snapshot,
    work: &std::path::Path,
    out: &std::path::Path,
    token: &CancellationToken,
) -> Result<Export> {
    let store = app.state::<Store>();
    project_storage::track_file(&store, &s.project, out)?;
    std::fs::create_dir_all(out)?;
    let output = out.join("previs.mp4");
    let mut command = tokio::process::Command::from(media::command("ffmpeg"));
    command
        .args([
            "-v",
            "error",
            "-y",
            "-framerate",
            &s.target.fps.to_string(),
            "-i",
        ])
        .arg(work.join("frames/%06d.png"))
        .args([
            "-c:v",
            "libx264",
            "-crf",
            "20",
            "-pix_fmt",
            "yuv420p",
            "-movflags",
            "+faststart",
        ])
        .arg(&output)
        .kill_on_drop(true);
    let child = command.output();
    let result = tokio::select! {
        r=tokio::time::timeout(std::time::Duration::from_secs(180),child)=>r.context("预演编码超时")??,
        _=token.cancelled()=>anyhow::bail!("预演导出已停止"),
    };
    ensure!(
        result.status.success(),
        "预演编码失败：{}",
        String::from_utf8_lossy(&result.stderr)
            .chars()
            .take(500)
            .collect::<String>()
    );
    ensure!(!token.is_cancelled(), "预演导出已停止");
    let project = s.project.clone();
    let target = s.target.clone();
    let model_revisions = s
        .models
        .iter()
        .map(|m| (m.id.clone(), m.revision))
        .collect();
    let signature = s.signature.clone();
    let id = id.to_owned();
    let app = app.clone();
    let prepared = out.join("media");
    tauri::async_runtime::spawn_blocking(move || -> Result<Export> {
        let mut asset = media::import(&output, &prepared)?;
        asset.name = format!("{} · 预演 v{}", target.name, target.revision);
        let export = Export {
            id,
            file_id: target.id,
            file_revision: target.revision,
            model_revisions,
            signature,
            asset,
            shared_asset_id: None,
        };
        let store = app.state::<Store>();
        let db = store.db.lock().unwrap();
        crate::project_service::load(&db, &project)?;
        db.execute(
            "INSERT INTO previs_exports VALUES(?1,?2,?3)",
            params![project, export.id, serde_json::to_string(&export)?],
        )?;
        Ok(export)
    })
    .await?
}
pub fn read_export(db: &rusqlite::Connection, project: &str, id: &str) -> Result<Export> {
    let raw: String = db
        .query_row(
            "SELECT data FROM previs_exports WHERE project_id=?1 AND id=?2",
            params![project, id],
            |r| r.get(0),
        )
        .context("预演视频不存在")?;
    Ok(serde_json::from_str(&raw)?)
}
