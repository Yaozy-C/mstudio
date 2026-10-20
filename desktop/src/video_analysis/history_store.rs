//! Durable records use relative media names so the configured storage can move safely.
use super::{Session, Video};
use crate::assistant::standalone_analysis::Analysis;
use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub const FOLDER: &str = "video-analysis-history";
#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextInfo {
    pub title: String,
    pub work_id: Option<String>,
    pub source_url: Option<String>,
    pub model_name: String,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub id: String,
    pub created_at: u64,
    pub context: ContextInfo,
    pub video: Video,
    pub question: String,
    pub model_id: String,
    pub language: String,
    pub result: Analysis,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub id: String,
    pub title: String,
    pub created_at: u64,
    pub duration: f64,
    pub model_name: String,
    pub question: String,
    pub summary: String,
    pub poster: Option<String>,
    pub missing_video: bool,
}
#[derive(Serialize)]
pub struct Page {
    pub items: Vec<Summary>,
    pub total: usize,
}
pub fn init(db: &Connection) -> Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS video_analysis_records(id TEXT PRIMARY KEY, created INTEGER NOT NULL, title TEXT NOT NULL, data TEXT NOT NULL);
    CREATE INDEX IF NOT EXISTS video_analysis_records_created ON video_analysis_records(created DESC);")?;
    Ok(())
}
pub fn directory(root: &Path, id: &str) -> Result<PathBuf> {
    uuid::Uuid::parse_str(id)?;
    Ok(root.join(FOLDER).join(id))
}
pub fn media_path(root: &Path, record: &Record) -> Result<PathBuf> {
    let name = Path::new(&record.video.path);
    ensure!(
        name.components().count() == 1 && name.file_name().is_some_and(|n| n == name.as_os_str()),
        "记录中的视频路径无效"
    );
    Ok(directory(root, &record.id)?.join(name))
}
pub fn read(db: &Connection, id: &str) -> Result<Record> {
    uuid::Uuid::parse_str(id)?;
    let raw: Option<String> = db
        .query_row(
            "SELECT data FROM video_analysis_records WHERE id=?1",
            [id],
            |r| r.get(0),
        )
        .optional()?;
    let record: Record = serde_json::from_str(&raw.context("拆解记录不存在")?)?;
    ensure!(record.id == id, "拆解记录标识无效");
    Ok(record)
}
pub fn summary(root: &Path, record: Record) -> Result<Summary> {
    let poster = directory(root, &record.id)?.join("poster.jpg");
    Ok(Summary {
        missing_video: !media_path(root, &record)?.is_file(),
        poster: poster.is_file().then(|| poster.to_string_lossy().into()),
        id: record.id,
        title: record.context.title,
        created_at: record.created_at,
        duration: record.video.duration,
        model_name: record.context.model_name,
        question: record.question,
        summary: record.result.summary,
    })
}
pub fn list(db: &Connection, root: &Path, query: &str, offset: usize) -> Result<Page> {
    ensure!(query.chars().count() <= 200, "搜索内容过长");
    let pattern = format!(
        "%{}%",
        query
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_")
    );
    let filter = "title LIKE ?1 ESCAPE '\\' OR json_extract(data,'$.question') LIKE ?1 ESCAPE '\\' OR json_extract(data,'$.result.summary') LIKE ?1 ESCAPE '\\'";
    let total = db.query_row(
        &format!("SELECT count(*) FROM video_analysis_records WHERE {filter}"),
        [&pattern],
        |r| r.get(0),
    )?;
    let mut stmt = db.prepare(&format!("SELECT data FROM video_analysis_records WHERE {filter} ORDER BY created DESC, rowid DESC LIMIT 40 OFFSET ?2"))?;
    let rows = stmt
        .query_map(params![pattern, offset as i64], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let items = rows
        .iter()
        .map(|raw| summary(root, serde_json::from_str(raw)?))
        .collect::<Result<Vec<_>>>()?;
    Ok(Page { items, total })
}
pub fn stage(root: &Path, session: &Session, mut record: Record) -> Result<Record> {
    ensure!(
        record.context.title.chars().count() <= 240
            && record.context.model_name.chars().count() <= 160,
        "记录名称过长"
    );
    if let Some(url) = &record.context.source_url {
        super::super::imports::video_url::parse_url(url)?;
    }
    if record.context.title.trim().is_empty() {
        record.context.title = session.video.name.clone();
    }
    let id = uuid::Uuid::new_v4().to_string();
    let directory = directory(root, &id)?;
    let original = Path::new(&session.video.path);
    let extension = original
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("mp4");
    let name = format!("source.{extension}");
    std::fs::create_dir_all(&directory)?;
    let result = (|| -> Result<()> {
        link_or_copy(original, &directory.join(&name))?;
        std::fs::File::open(directory.join(&name))?.sync_all()?;
        record.id = id.clone();
        record.created_at = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
        record.video.path = name;
        // A missing preview must never discard a completed breakdown.
        let _ = mstudio::media::run(
            mstudio::media::command("ffmpeg")
                .args(["-v", "error", "-y", "-i"])
                .arg(original)
                .args([
                    "-frames:v",
                    "1",
                    "-vf",
                    "scale=320:180:force_original_aspect_ratio=decrease",
                    "-update",
                    "1",
                ])
                .arg(directory.join("poster.jpg")),
        );
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&directory);
    }
    result?;
    Ok(record)
}
pub fn insert(root: &Path, db: &Connection, record: &Record) -> Result<String> {
    let saved = db.execute(
        "INSERT INTO video_analysis_records(id,created,title,data) VALUES(?1,?2,?3,?4)",
        params![
            record.id,
            record.created_at as i64,
            record.context.title,
            serde_json::to_string(record)?
        ],
    );
    if saved.is_err() {
        let _ = std::fs::remove_dir_all(directory(root, &record.id)?);
    }
    saved?;
    Ok(record.id.clone())
}
pub fn link_or_copy(source: &Path, destination: &Path) -> Result<()> {
    if std::fs::hard_link(source, destination).is_err() {
        std::fs::copy(source, destination)?;
    }
    Ok(())
}
