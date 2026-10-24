use crate::{database::Store, project_service};
use anyhow::{Context, Result, ensure};
use mstudio::{media, model::Asset};
use rusqlite::params;
use serde_json::json;
use std::path::PathBuf;

pub fn input(store: &Store, project: &str, id: &str) -> Result<PathBuf> {
    let document = project_service::load(&store.db.lock().unwrap(), project)?;
    let value = document["assets"]
        .as_array()
        .and_then(|assets| assets.iter().find(|a| a["id"] == id))
        .context("图片不属于当前项目")?;
    ensure!(value["discarded"] != true, "图片已弃用");
    let asset = store
        .assets()?
        .into_iter()
        .find(|a| a.id == id)
        .context("图片不存在")?;
    ensure!(
        asset.kind == "image" && !asset.missing,
        "请选择有效图片素材"
    );
    crate::previs::media::managed_path(store, &asset)
}

pub fn save(store: &Store, project: &str, name: &str, outputs: &[PathBuf]) -> Result<Vec<Asset>> {
    let mut assets = Vec::new();
    let result = (|| -> Result<()> {
        for (index, output) in outputs.iter().enumerate() {
            let mut asset = media::import(output, &store.media_root())?;
            ensure!(asset.kind == "image", "处理输出不是图片");
            asset.generated = true;
            asset.name = if index == 0 {
                name.into()
            } else {
                format!("{name} · 16-bit")
            };
            assets.push(asset);
        }
        let mut db = store.db.lock().unwrap();
        let tx = db.transaction()?;
        let before = project_service::load(&tx, project)?;
        let mut after = before.clone();
        let library = after["assets"].as_array_mut().context("项目素材目录无效")?;
        for asset in &assets {
            tx.execute(
                "INSERT INTO assets VALUES(?1,?2)",
                params![asset.id, serde_json::to_string(asset)?],
            )?;
            library.push(json!(asset));
        }
        project_service::persist(&tx, &before, &after)?;
        tx.commit()?;
        Ok(())
    })();
    if result.is_err() {
        for asset in &assets {
            let _ = std::fs::remove_file(&asset.path);
            if !asset.preview.is_empty() {
                let _ = std::fs::remove_file(&asset.preview);
            }
        }
    }
    result?;
    Ok(assets)
}
