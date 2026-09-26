use super::{FOLDERS, rewrite};
use crate::database::Store;
use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

fn files(root: &Path, folder: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    if !folder.try_exists()? {
        return Ok(());
    }
    let meta = fs::symlink_metadata(folder)?;
    ensure!(
        !meta.file_type().is_symlink(),
        "存储目录包含符号链接，请先移走：{}",
        folder.display()
    );
    if meta.is_dir() {
        for item in fs::read_dir(folder)? {
            files(root, &item?.path(), out)?;
        }
    } else {
        ensure!(meta.is_file(), "存储目录包含特殊文件：{}", folder.display());
        out.push(folder.strip_prefix(root)?.into());
    }
    Ok(())
}

fn equal(a: &Path, b: &Path) -> Result<bool> {
    let mut a = fs::File::open(a)?;
    let mut b = fs::File::open(b)?;
    if a.metadata()?.len() != b.metadata()?.len() {
        return Ok(false);
    }
    let mut left = [0u8; 65536];
    let mut right = [0u8; 65536];
    loop {
        let n = a.read(&mut left)?;
        if n == 0 {
            return Ok(true);
        }
        b.read_exact(&mut right[..n])?;
        if left[..n] != right[..n] {
            return Ok(false);
        }
    }
}

fn rewrite_records(db: &Connection, from: &Path, to: &Path) -> Result<()> {
    for (table, column) in [
        ("projects", "document"),
        ("assets", "data"),
        ("jobs", "data"),
        ("agent_messages", "payload"),
        ("agent_events", "payload"),
    ] {
        let mut stmt = db.prepare(&format!("SELECT rowid,{column} FROM {table}"))?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for (id, raw) in rows {
            let mut value: Value = serde_json::from_str(&raw)?;
            let previous = value.clone();
            rewrite(&mut value, from, to);
            if value != previous {
                db.execute(
                    &format!("UPDATE {table} SET {column}=?1 WHERE rowid=?2"),
                    params![value.to_string(), id],
                )?;
            }
        }
    }
    for table in ["project_files", "pending_file_deletions"] {
        let mut stmt = db.prepare(&format!("SELECT rowid,path FROM {table}"))?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for (id, path) in rows {
            let mut value = json!(path);
            rewrite(&mut value, from, to);
            if value != path {
                db.execute(
                    &format!("UPDATE {table} SET path=?1 WHERE rowid=?2"),
                    params![value.as_str(), id],
                )?;
            }
        }
    }
    Ok(())
}

pub(super) fn migrate(
    store: &Store,
    target: &Path,
    allow: impl FnOnce(&Path) -> Result<()>,
) -> Result<Value> {
    let source = store.media_root();
    ensure!(
        source.is_dir(),
        "当前存储目录不可用，请先连接原存储设备再迁移"
    );
    let canonical_source = source.canonicalize()?;
    ensure!(target.is_absolute(), "请选择绝对路径");
    let parent = target.parent().context("存储路径无效")?.canonicalize()?;
    let target = parent.join(target.file_name().context("存储路径无效")?);
    if canonical_source == target {
        return Ok(json!({"directory":target,"files":0,"warning":""}));
    }
    ensure!(
        !target.starts_with(&canonical_source) && !canonical_source.starts_with(&target),
        "新旧存储目录不能相互包含"
    );
    if target.try_exists()? {
        ensure!(
            !fs::symlink_metadata(&target)?.file_type().is_symlink(),
            "存储目录不能是符号链接"
        );
        ensure!(
            target.is_dir() && fs::read_dir(&target)?.next().is_none(),
            "目标 Mstudio 目录已有内容，请选择另一个空目录，避免覆盖文件"
        );
    }
    let db = store.db.lock().unwrap();
    let running: bool = db.query_row("SELECT EXISTS(SELECT 1 FROM agent_messages WHERE json_extract(attribution,'$.status')='running')", [], |r| r.get(0))?;
    ensure!(!running, "Agent 正在执行任务，请等待结束或停止后再迁移");
    drop(db);
    let mut entries = vec![];
    for folder in FOLDERS {
        files(&source, &source.join(folder), &mut entries)?;
    }
    let stage = parent.join(format!(".mstudio-migration-{}", mstudio::media::id()));
    fs::create_dir(&stage)?;
    let mut installed = false;
    let result = (|| -> Result<()> {
        for relative in &entries {
            let destination = stage.join(relative);
            fs::create_dir_all(destination.parent().unwrap())?;
            fs::copy(source.join(relative), &destination).with_context(|| {
                format!("无法复制 {}，请检查权限和剩余空间", relative.display())
            })?;
            fs::File::open(&destination)?.sync_all()?;
            ensure!(
                equal(&source.join(relative), &destination)?,
                "文件校验失败：{}",
                relative.display()
            );
        }
        // Rename only after the complete copy is verified; never overwrite an existing library.
        if target.exists() {
            fs::remove_dir(&target)?;
        }
        fs::rename(&stage, &target)?;
        installed = true;
        allow(&target)?;
        let mut location = store.location.read().unwrap().clone();
        location.previous.push(source.clone());
        location.directory = target.clone();
        let mut db = store.db.lock().unwrap();
        let tx = db.transaction()?;
        rewrite_records(&tx, &source, &target)?;
        tx.execute("INSERT INTO settings VALUES('file-storage',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [serde_json::to_string(&location)?])?;
        tx.commit()?;
        *store.location.write().unwrap() = location;
        Ok(())
    })();
    if let Err(error) = result {
        // These are newly created copies only. Original records and files are untouched.
        let cleanup = fs::remove_dir_all(if installed { &target } else { &stage });
        return Err(if let Err(cleanup) = cleanup {
            error.context(format!("临时副本清理失败：{cleanup}"))
        } else {
            error
        });
    }
    let mut retained = 0;
    for relative in &entries {
        let old = source.join(relative);
        // A file changed outside the app during migration must not be discarded.
        if !equal(&old, &target.join(relative)).unwrap_or(false) || fs::remove_file(old).is_err() {
            retained += 1;
        }
    }
    Ok(
        json!({"directory":target,"files":entries.len(),"warning": if retained > 0 { format!("新目录已生效；原目录有 {retained} 个文件因发生变化或无法清理而保留，请检查。") } else { String::new() }}),
    )
}
