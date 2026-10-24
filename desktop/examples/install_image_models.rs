//! Explicit native-model installer; no Python or downloads during inference.
use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
};

fn digest(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 1024 * 1024];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[tokio::main]
async fn main() -> Result<()> {
    let directory = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .context("用法：install_image_models /absolute/models")?,
    );
    ensure!(directory.is_absolute(), "模型目录须为绝对路径");
    std::fs::create_dir_all(&directory)?;
    let client = reqwest::Client::new();
    for item in mstudio_image_runtime::model_files()? {
        let path = directory.join(&item.file);
        if path.exists() {
            ensure!(digest(&path)? == item.sha256, "模型摘要不符：{}", item.file);
            println!("Verified {}", item.file);
            continue;
        }
        let partial = path.with_extension("download");
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&partial)?;
        let mut response = client.get(&item.url).send().await?.error_for_status()?;
        let mut hash = Sha256::new();
        while let Some(chunk) = response.chunk().await? {
            hash.update(&chunk);
            file.write_all(&chunk)?;
        }
        file.sync_all()?;
        ensure!(
            format!("{:x}", hash.finalize()) == item.sha256,
            "下载摘要不符：{}",
            item.file
        );
        std::fs::rename(partial, &path)?;
        println!("Installed {}", item.file);
    }
    Ok(())
}
