use super::{
    CleanupItem, Report,
    graph::{self, Graph},
};
use anyhow::{Result, ensure};
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

pub struct Plan {
    pub graph: Graph,
    pub files: Vec<PathBuf>,
    pub report: Report,
}
fn category(path: &Path, root: &Path) -> &'static str {
    match path
        .strip_prefix(root)
        .ok()
        .and_then(|p| p.components().next())
        .and_then(|p| p.as_os_str().to_str())
    {
        Some("assets" | "previews" | "video-analysis-history") => "media",
        Some("previs" | "exports") => "previs",
        Some("reference-media") => "snapshots",
        Some("proxies" | "captions") => "cache",
        Some(folder) if crate::storage::FOLDERS.contains(&folder) => "temporary",
        _ => "other",
    }
}
fn walk(folder: &Path, out: &mut Vec<(PathBuf, fs::Metadata)>) -> Result<()> {
    let meta = match fs::symlink_metadata(folder) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    if meta.file_type().is_symlink() {
        return Ok(());
    }
    if meta.is_dir() {
        for entry in fs::read_dir(folder)? {
            walk(&entry?.path(), out)?;
        }
    } else if meta.is_file() {
        out.push((folder.into(), meta));
    }
    Ok(())
}
fn kept(path: &Path, root: &Path, graph: &Graph) -> bool {
    // These are intentional, durable records independent of projects.
    if path.starts_with(root.join(crate::video_analysis::history_store::FOLDER)) {
        return true;
    }
    if graph.paths.contains(path) {
        return true;
    }
    // Finished films are intentional user exports, even without a canvas link.
    if let Ok(relative) = path.strip_prefix(root.join("exports")) {
        let folder = relative
            .components()
            .next()
            .map(|p| p.as_os_str().to_string_lossy());
        let private = folder
            .as_deref()
            .and_then(|name| name.strip_prefix("previs-"))
            .is_some_and(|id| uuid::Uuid::parse_str(id).is_ok());
        return !private || relative.components().count() < 2;
    }
    if path.starts_with(root.join("proxies")) {
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if graph
            .ids
            .iter()
            .any(|id| name.starts_with(&format!("{id}-v1-")))
        {
            return true;
        }
    }
    false
}
pub fn busy(db: &Connection) -> Result<bool> {
    Ok(db.query_row("SELECT EXISTS(SELECT 1 FROM jobs WHERE coalesce(json_extract(data,'$.status'),'UNKNOWN') NOT IN ('COMPLETED','FAILED','CANCELLED')) OR EXISTS(SELECT 1 FROM agent_messages WHERE json_extract(attribution,'$.status') IN ('running','waiting')) OR EXISTS(SELECT 1 FROM subagent_runs WHERE status='running')", [], |r| r.get(0))?)
}
pub fn build(db: &Connection, root: &Path, data: &Path) -> Result<Plan> {
    ensure!(root.is_dir(), "存储目录不可用，请连接原存储设备");
    ensure!(
        !fs::symlink_metadata(root)?.file_type().is_symlink(),
        "存储目录不能是符号链接"
    );
    let graph = graph::build(db, root)?;
    let mut disk = vec![];
    walk(root, &mut disk)?;
    disk.sort_by(|a, b| a.0.cmp(&b.0));
    let mut report = Report {
        can_clean: !busy(db)?,
        ..Report::default()
    };
    let mut files = vec![];
    let mut hash = Sha256::new();
    hash.update(root.to_string_lossy().as_bytes());
    let mut seen = HashSet::new();
    for (path, meta) in &disk {
        let allocated = unique_size(meta, path, &mut seen);
        report.media_bytes += allocated;
        let key = category(path, root);
        let item = report.categories.entry(key.into()).or_default();
        item.bytes += allocated;
        item.files += 1;
        // An interrupted producer may still be returning an external result. Fresh files wait.
        let fresh = meta.modified()?.elapsed().unwrap_or_default() < Duration::from_secs(300);
        if key != "other"
            && !fresh
            && !kept(path, root, &graph)
            && crate::project_storage::managed(root, path)?.is_some()
        {
            files.push(path.clone());
            hash.update(path.to_string_lossy().as_bytes());
            hash.update(identity(meta, path).as_bytes());
            hash.update(meta.len().to_le_bytes());
            hash.update(
                meta.modified()?
                    .duration_since(SystemTime::UNIX_EPOCH)?
                    .as_nanos()
                    .to_le_bytes(),
            );
        }
    }
    let candidates: HashSet<_> = files.iter().collect();
    let mut links = std::collections::HashMap::<String, usize>::new();
    for (path, meta) in &disk {
        if candidates.contains(path) {
            *links.entry(identity(meta, path)).or_default() += 1;
        }
    }
    let mut counted = HashSet::new();
    for (path, meta) in &disk {
        let key = identity(meta, path);
        if candidates.contains(path)
            && counted.insert(key.clone())
            && links.get(&key).copied().unwrap_or(0) >= link_count(meta)
        {
            let bytes = physical_size(meta);
            report.reclaimable_bytes += bytes;
            report
                .categories
                .get_mut(category(path, root))
                .unwrap()
                .reclaimable_bytes += bytes;
        }
    }
    let mut app_files = vec![];
    if !data.starts_with(root) {
        walk(data, &mut app_files)?;
    }
    for (path, meta) in app_files {
        if !path.starts_with(root) {
            report.data_bytes += unique_size(&meta, &path, &mut seen);
        }
    }
    let mut records: Vec<_> = graph
        .assets
        .iter()
        .filter_map(|a| a["id"].as_str())
        .filter(|id| !graph.ids.contains(*id))
        .map(str::to_owned)
        .collect();
    records.extend(graph.private.iter().filter_map(|(p, a)| {
        a["id"]
            .as_str()
            .filter(|id| !graph.ids.contains(*id))
            .map(|id| format!("{p}/{id}"))
    }));
    for sql in [
        "SELECT 'blob:'||digest FROM content_blobs b WHERE NOT EXISTS(SELECT 1 FROM job_content WHERE digest=b.digest) AND NOT EXISTS(SELECT 1 FROM event_content WHERE digest=b.digest)",
        "SELECT 'snapshot:'||digest FROM stored_media m WHERE NOT EXISTS(SELECT 1 FROM content_blobs b WHERE b.media_digest=m.digest AND (EXISTS(SELECT 1 FROM job_content WHERE digest=b.digest) OR EXISTS(SELECT 1 FROM event_content WHERE digest=b.digest)))",
    ] {
        records.extend(
            db.prepare(sql)?
                .query_map([], |r| r.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?,
        );
    }
    records.extend(db.prepare("SELECT DISTINCT asset_id FROM project_assets WHERE NOT EXISTS(SELECT 1 FROM assets WHERE id=asset_id)")?
        .query_map([], |r| r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?.into_iter()
        .filter(|id| !graph.ids.contains(id)).map(|id| format!("ownership:{id}")));
    records.sort();
    for record in &records {
        hash.update(record.as_bytes());
    }
    report.items = files
        .iter()
        .take(100)
        .map(|path| CleanupItem {
            kind: "file".into(),
            name: path.strip_prefix(root).unwrap().to_string_lossy().into(),
            bytes: fs::metadata(path).map(|m| m.len()).unwrap_or(0),
        })
        .collect();
    for asset in graph
        .assets
        .iter()
        .chain(graph.private.iter().map(|(_, a)| a))
    {
        if report.items.len() >= 100 {
            break;
        }
        if asset["id"]
            .as_str()
            .is_some_and(|id| !graph.ids.contains(id))
        {
            report.items.push(CleanupItem {
                kind: "record".into(),
                name: asset["name"].as_str().unwrap_or("未命名素材").into(),
                bytes: 0,
            });
        }
    }
    for record in records
        .iter()
        .filter(|s| s.starts_with("blob:") || s.starts_with("snapshot:"))
    {
        if report.items.len() >= 100 {
            break;
        }
        report.items.push(CleanupItem {
            kind: "record".into(),
            name: format!("无引用快照 · {}", &record[..record.len().min(20)]),
            bytes: 0,
        });
    }
    report.unused_records = records.len() as u64;
    report.unused_files = files.len() as u64;
    report.total_bytes = report.media_bytes + report.data_bytes;
    report.fingerprint = format!("{:x}", hash.finalize());
    Ok(Plan {
        graph,
        files,
        report,
    })
}
fn identity(meta: &fs::Metadata, path: &Path) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let _ = path;
        format!("{}:{}", meta.dev(), meta.ino())
    }
    #[cfg(not(unix))]
    {
        let _ = meta;
        path.to_string_lossy().into()
    }
}
fn link_count(meta: &fs::Metadata) -> usize {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        meta.nlink() as usize
    }
    #[cfg(not(unix))]
    {
        let _ = meta;
        1
    }
}
fn physical_size(meta: &fs::Metadata) -> u64 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        meta.blocks() * 512
    }
    #[cfg(not(unix))]
    {
        meta.len()
    }
}
fn unique_size(meta: &fs::Metadata, path: &Path, seen: &mut HashSet<String>) -> u64 {
    if seen.insert(identity(meta, path)) {
        physical_size(meta)
    } else {
        0
    }
}
