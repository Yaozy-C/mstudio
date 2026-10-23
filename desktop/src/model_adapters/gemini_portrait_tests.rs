//! Opt-in portrait comparison using the real Gemini adapter and existing connection.
use super::*;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use futures::{StreamExt, stream};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

fn save(path: &Path, value: &Value) -> Result<()> {
    std::fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

async fn portrait(root: &Path, spec: &Value, key: &str, base: &str, case: Value) -> Result<Value> {
    let id = case["id"].as_str().context("case needs id")?;
    ensure!(id.len() == 2 && id.bytes().all(|c| c.is_ascii_digit()));
    let directory = root.join(id);
    std::fs::create_dir_all(&directory)?;
    let mut input = json!({"model":spec["model"],"prompt":case["prompt"],
        "generationConfig":spec["generationConfig"]});
    if let Some(path) = case["editTarget"].as_str() {
        let bytes = std::fs::read(path)?;
        ensure!(
            bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
            "edit target must be PNG"
        );
        input["image_urls"] = json!([format!("data:image/png;base64,{}", STANDARD.encode(bytes))]);
    }
    let request = payload(&input)?;
    let request_path = directory.join("request.json");
    let summary_path = directory.join("summary.json");
    if request_path.exists() {
        let previous: Value = serde_json::from_slice(&std::fs::read(&request_path)?)?;
        ensure!(
            previous == request,
            "request changed; choose a fresh output directory"
        );
        let summary: Value = serde_json::from_slice(&std::fs::read(&summary_path)?)?;
        ensure!(
            summary["status"] == "COMPLETED",
            "inspect previous attempt before resubmitting"
        );
        return Ok(summary);
    }
    save(&request_path, &request)?;
    let started = std::time::Instant::now();
    println!("Gemini portrait {id}: generating with {}", spec["model"]);
    let result = Gemini.submit(key, base, &input, &json!({}), None).await;
    let summary = match result {
        Ok(update) => {
            let outputs = update.outputs.context("missing outputs")?;
            ensure!(outputs.len() == 1, "expected one final image");
            let (mime, bytes) = image_bytes(&outputs[0].url)?;
            let extension = match mime {
                "image/png" => "png",
                "image/jpeg" => "jpg",
                "image/webp" => "webp",
                _ => unreachable!(),
            };
            let output = directory.join(format!("portrait.{extension}"));
            std::fs::write(&output, bytes)?;
            println!("Gemini portrait {id}: saved {}", output.display());
            json!({"id":id,"label":case["label"],"status":"COMPLETED",
                "requestedModel":spec["model"],"output":output,"mimeType":mime,
                "elapsedSeconds":started.elapsed().as_secs_f64(),"editTarget":case["editTarget"]})
        }
        Err(error) => {
            let safe = error.to_string().replace(key, "[REDACTED]");
            println!("Gemini portrait {id}: failed: {safe}");
            json!({"id":id,"status":"FAILED","error":safe,
                "elapsedSeconds":started.elapsed().as_secs_f64()})
        }
    };
    save(&summary_path, &summary)?;
    Ok(summary)
}

#[tokio::test]
#[ignore = "uses configured Google connection; consumes one image per manifest case"]
async fn live_gemini_portrait_assets() -> Result<()> {
    let manifest = PathBuf::from(std::env::var("MSTUDIO_GEMINI_PORTRAIT_MANIFEST")?);
    let root = manifest.parent().context("manifest needs parent")?;
    let spec: Value = serde_json::from_slice(&std::fs::read(&manifest)?)?;
    let database = spec["database"]
        .as_str()
        .context("database path required")?;
    let connection_id = spec["connectionId"]
        .as_str()
        .context("connection id required")?;
    let db = rusqlite::Connection::open_with_flags(
        database,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    let service = crate::models::connections::get(&db, connection_id)?;
    ensure!(
        service.kind == "gemini-native",
        "use an existing Google native connection"
    );
    endpoint(&service.endpoint)?;
    let key = crate::models::connections::key(&db, connection_id)?;
    ensure!(!key.is_empty(), "Google key is not configured");
    drop(db);
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let mut sources = Vec::new();
    for resource in [
        "SKILL.md",
        "references/character-assets.md",
        "references/photographic-people.md",
        "references/scene-execution.md",
        "references/visual-quality.md",
    ] {
        let path = repo.join("skills/image-production").join(resource);
        let content = std::fs::read_to_string(&path)?;
        sources.push(
            json!({"path":path,"sha256":format!("{:x}",Sha256::digest(content.as_bytes())),
            "content":content}),
        );
    }
    save(&root.join("skill-snapshot.json"), &json!(sources))?;
    let cases = spec["entries"].as_array().context("entries required")?;
    ensure!(
        !cases.is_empty() && cases.len() <= 9,
        "probe allows 1–9 cases"
    );
    let results: Vec<_> = stream::iter(cases.iter().cloned())
        .map(|case| portrait(root, &spec, &key, &service.endpoint, case))
        .buffer_unordered(2)
        .collect()
        .await;
    let results = results.into_iter().collect::<Result<Vec<_>>>()?;
    save(
        &root.join("results.json"),
        &json!({"transport":"production Gemini::submit",
        "model":spec["model"],"results":results}),
    )?;
    ensure!(
        results.iter().all(|r| r["status"] == "COMPLETED"),
        "inspect failed cases"
    );
    Ok(())
}
