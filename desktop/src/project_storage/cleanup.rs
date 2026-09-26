use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, params};
use std::path::{Component, Path, PathBuf};

// Only files inside these app-owned directories can ever enter the delete queue.
pub fn managed(root: &Path, path: &Path) -> Result<Option<PathBuf>> {
    if !root.try_exists()? {
        return Ok(None);
    }
    if !path.is_absolute() {
        return Ok(None);
    }
    let Ok(relative) = path.strip_prefix(root) else {
        return Ok(None);
    };
    if !relative
        .components()
        .all(|c| matches!(c, Component::Normal(_)))
    {
        return Ok(None);
    }
    let parts: Vec<_> = relative.components().collect();
    if parts.len() < 2 {
        return Ok(None);
    }
    let folder = parts[0].as_os_str().to_string_lossy();
    if !crate::storage::FOLDERS.contains(&folder.as_ref()) {
        return Ok(None);
    }
    let canonical_root = root.canonicalize()?;
    let mut current = root.to_path_buf();
    for part in parts {
        current.push(part);
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) => {
                // Never follow symlinks, including a symlinked managed directory.
                if metadata.file_type().is_symlink() {
                    return Ok(None);
                }
                ensure!(
                    current.canonicalize()?.starts_with(&canonical_root),
                    "素材路径超出应用目录"
                );
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => break,
            Err(e) => return Err(e.into()),
        }
    }
    Ok(Some(path.to_path_buf()))
}

pub fn run(db: &Connection, root: &Path, project: Option<&str>) -> Result<()> {
    let mut statement = db.prepare(
        "SELECT project_id,path FROM pending_file_deletions WHERE ?1 IS NULL OR project_id=?1",
    )?;
    let rows = statement
        .query_map([project], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut errors = vec![];
    for (id, raw) in rows {
        let result = (|| -> Result<()> {
            if let Some(path) = managed(root, Path::new(&raw))? {
                let result = if path.is_dir() {
                    std::fs::remove_dir_all(&path)
                } else {
                    std::fs::remove_file(&path)
                };
                if let Err(e) = result
                    && e.kind() != std::io::ErrorKind::NotFound
                {
                    return Err(e).with_context(|| format!("无法清理 {}", path.display()));
                }
            }
            db.execute(
                "DELETE FROM pending_file_deletions WHERE project_id=?1 AND path=?2",
                params![id, raw],
            )?;
            Ok(())
        })();
        if let Err(e) = result {
            errors.push(e.to_string());
        }
    }
    ensure!(
        errors.is_empty(),
        "项目记录已删除，部分文件清理失败，请重试：{}",
        errors.join("；")
    );
    Ok(())
}
