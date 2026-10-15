use super::{File, files};
use crate::database::Store;
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tauri::State;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub project: String,
    pub target: File,
    pub models: Vec<File>,
    pub signature: String,
}
impl Snapshot {
    pub fn create(project: String, target: File, mut models: Vec<File>) -> Result<Self> {
        ensure!(!target.code.trim().is_empty(), "请先生成或填写预演代码");
        target.validate()?;
        models.sort_by(|a, b| a.id.cmp(&b.id));
        let signature = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(
                &json!({"runtime":1,"project":project,"target":target,"models":models})
            )?)
        );
        Ok(Self {
            project,
            target,
            models,
            signature,
        })
    }
}
pub struct PrevisRuntime {
    pub address: String,
    sessions: Arc<Mutex<HashMap<String, Snapshot>>>,
}
/// Start the private runtime only on demand; a bind failure cannot block startup.
#[derive(Default)]
pub struct RuntimeState(Mutex<Option<Arc<PrevisRuntime>>>);
impl RuntimeState {
    pub fn service(&self) -> Result<Arc<PrevisRuntime>> {
        let mut runtime = self.0.lock().unwrap();
        if runtime.is_none() {
            *runtime = Some(Arc::new(PrevisRuntime::start()?));
        }
        Ok(runtime.as_ref().unwrap().clone())
    }
}
impl PrevisRuntime {
    pub fn start() -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let address = format!("http://{}", listener.local_addr()?);
        let sessions = Arc::new(Mutex::new(HashMap::new()));
        let state = sessions.clone();
        let host = listener.local_addr()?.to_string();
        std::thread::Builder::new()
            .name("previs-resources".into())
            .spawn(move || {
                let active = Arc::new(AtomicUsize::new(0));
                for socket in listener.incoming().flatten() {
                    if active.load(Ordering::Relaxed) >= 16 {
                        continue;
                    }
                    active.fetch_add(1, Ordering::Relaxed);
                    let state = state.clone();
                    let host = host.clone();
                    let active = active.clone();
                    std::thread::spawn(move || {
                        let _ = serve(socket, &host, &state);
                        active.fetch_sub(1, Ordering::Relaxed);
                    });
                }
            })?;
        Ok(Self { address, sessions })
    }
    pub fn open(&self, snapshot: Snapshot) -> Result<String> {
        let mut sessions = self.sessions.lock().unwrap();
        ensure!(sessions.len() < 32, "预演窗口过多，请关闭旧窗口");
        let id = uuid::Uuid::new_v4().to_string();
        sessions.insert(id.clone(), snapshot);
        Ok(id)
    }
    pub fn get(&self, id: &str, project: &str) -> Result<Snapshot> {
        let s = self
            .sessions
            .lock()
            .unwrap()
            .get(id)
            .cloned()
            .context("预演环境已关闭，请重新加载")?;
        ensure!(s.project == project, "预演环境不属于此项目");
        Ok(s)
    }
}
fn serve(
    mut socket: TcpStream,
    host: &str,
    sessions: &Mutex<HashMap<String, Snapshot>>,
) -> Result<()> {
    socket.set_read_timeout(Some(Duration::from_secs(3)))?;
    socket.set_write_timeout(Some(Duration::from_secs(10)))?;
    let mut bytes = Vec::new();
    let mut buf = [0; 1024];
    while !bytes.windows(4).any(|w| w == b"\r\n\r\n") {
        let n = socket.read(&mut buf)?;
        ensure!(n > 0 && bytes.len() + n <= 8192, "请求头无效");
        bytes.extend_from_slice(&buf[..n]);
    }
    let request = std::str::from_utf8(&bytes)?;
    let mut parts = request.lines().next().unwrap_or("").split_whitespace();
    let method = parts.next().unwrap_or("");
    let uri = parts.next().unwrap_or("");
    let hostname = request.lines().find_map(|l| {
        l.split_once(':')
            .filter(|(k, _)| k.eq_ignore_ascii_case("host"))
            .map(|(_, v)| v.trim())
    });
    if method != "GET" || hostname != Some(host) {
        return respond(&mut socket, 403, "text/plain", b"Forbidden");
    }
    let route = uri.split('?').next().unwrap_or("");
    let mut path = route.trim_start_matches('/').split('/');
    let session = path.next().unwrap_or("");
    let file = path.next().unwrap_or("");
    if path.next().is_some() {
        return respond(&mut socket, 404, "text/plain", b"Not found");
    }
    let snapshot = sessions.lock().unwrap().get(session).cloned();
    let Some(snapshot) = snapshot else {
        return respond(&mut socket, 404, "text/plain", b"Not found");
    };
    let body = resource(&snapshot, file);
    match body {
        Some((mime, bytes)) => respond(&mut socket, 200, mime, &bytes),
        None => respond(&mut socket, 404, "text/plain", b"Not found"),
    }
}
fn resource(s: &Snapshot, file: &str) -> Option<(&'static str, Vec<u8>)> {
    match file {
        "runtime.html"=>Some(("text/html; charset=utf-8",include_bytes!("../../../frontend/src/previs/runtime/runtime.html").to_vec())),
        "runtime.js"=>Some(("text/javascript; charset=utf-8",include_bytes!("../../../frontend/src/previs/runtime/runtime.js").to_vec())),
        "runtime.css"=>Some(("text/css",include_bytes!("../../../frontend/src/previs/runtime/runtime.css").to_vec())),
        "three.LICENSE.txt"=>Some(("text/plain; charset=utf-8",include_bytes!("../../../frontend/node_modules/three/LICENSE").to_vec())),
        "three.module.js"=>Some(("text/javascript",include_bytes!("../../../frontend/node_modules/three/build/three.module.min.js").to_vec())),
        "three.core.min.js"=>Some(("text/javascript",include_bytes!("../../../frontend/node_modules/three/build/three.core.min.js").to_vec())),
        "manifest.json"=>Some(("application/json",serde_json::to_vec(&json!({"target":{"id":s.target.id,"kind":s.target.kind,"duration":s.target.duration,"width":s.target.width,"height":s.target.height,"fps":s.target.fps,"bindings":s.target.bindings},"models":s.models.iter().map(|m|json!({"id":m.id,"name":m.name})).collect::<Vec<_>>(),"signature":s.signature})).ok()?)),
        "shot.js" if s.target.kind=="shot"=>Some(("text/javascript",s.target.code.as_bytes().to_vec())),
        _=>s.models.iter().find(|m|format!("model-{}.js",m.id)==file).map(|m|("text/javascript",m.code.as_bytes().to_vec())),
    }
}
fn respond(socket: &mut TcpStream, status: u16, mime: &str, bytes: &[u8]) -> Result<()> {
    // No IPC, CDN, filesystem, or other origins are available inside the code frame.
    write!(
        socket,
        "HTTP/1.1 {status} OK\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nContent-Security-Policy: default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' data: blob:; connect-src 'self'; object-src 'none'; frame-src 'none'; worker-src 'none'; base-uri 'none'\r\n\r\n",
        bytes.len()
    )?;
    socket.write_all(bytes)?;
    Ok(())
}
#[tauri::command]
pub fn open_previs_runtime(
    store: State<Store>,
    runtime: State<RuntimeState>,
    project_id: String,
    file_id: String,
) -> Result<serde_json::Value, String> {
    (||->Result<_>{
        let runtime=runtime.service()?;
        let db=store.db.lock().unwrap();let target=files::read(&db,&project_id,&file_id)?.context("预演文件不存在")?;
        files::validate_target(&db,&project_id,&target)?;
        let all=files::list(&db,&project_id)?;
        let models=all.into_iter().filter(|m|m.kind=="model" && (target.kind=="model" && m.id==target.id || target.asset_ids.contains(&m.id))).collect();
        let snapshot=Snapshot::create(project_id,target,models)?;let signature=snapshot.signature.clone();
        let revision=snapshot.target.revision;
        let id=runtime.open(snapshot)?;Ok(json!({"sessionId":id,"url":format!("{}/{id}/runtime.html",runtime.address),"signature":signature,"fileRevision":revision}))
    })().map_err(|e|e.to_string())
}
#[tauri::command]
pub fn close_previs_runtime(runtime: State<RuntimeState>, session_id: String) {
    if let Some(runtime) = runtime.0.lock().unwrap().as_ref() {
        runtime.sessions.lock().unwrap().remove(&session_id);
    }
}
