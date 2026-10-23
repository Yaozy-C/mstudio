//! Opt-in visual probes through the production transport; never run in ordinary CI.
use super::*;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use futures::{StreamExt, stream};
use sha2::{Digest, Sha256};
use std::path::Path;

fn save(path: &Path, value: &Value) -> anyhow::Result<()> {
    std::fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

async fn portrait(root: &Path, model: &str, case: Value) -> anyhow::Result<Value> {
    let id = case["id"].as_str().context("case needs id")?;
    ensure!(id.len() == 2 && id.bytes().all(|c| c.is_ascii_digit()));
    let directory = root.join(id);
    std::fs::create_dir_all(&directory)?;
    let mut input = json!({"model":model,"prompt":case["prompt"],"n":1});
    if let Some(path) = case["editTarget"].as_str() {
        let bytes = std::fs::read(path)?;
        ensure!(
            bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
            "edit target must be PNG"
        );
        input["image"] = json!([format!("data:image/png;base64,{}", STANDARD.encode(bytes))]);
    }
    let (_, payload) = codex_request::payload(&input)?;
    let request_path = directory.join("request.json");
    let output = directory.join("portrait.png");
    if request_path.exists() {
        let previous: Value = serde_json::from_slice(&std::fs::read(&request_path)?)?;
        ensure!(
            previous == payload,
            "saved request differs; choose a fresh output directory"
        );
        ensure!(
            output.exists(),
            "previous attempt has no image; inspect it before resubmitting"
        );
        return Ok(json!({"id":id,"status":"REUSED","output":output}));
    }
    save(&request_path, &payload)?;
    println!("Portrait {id}: generating with {model}");
    let started = std::time::Instant::now();
    persist(&directory, &update("IN_QUEUE", None, None)?)?;
    let result = tokio::time::timeout(
        Duration::from_secs(600),
        generate(&input, &directory, CancellationToken::new()),
    )
    .await;
    let generated = match result {
        Ok(Ok(value)) => value,
        other => {
            let error = match other {
                Ok(Err(error)) => error.to_string(),
                _ => "generation timed out; server may still be running".to_string(),
            };
            let summary = json!({"id":id,"status":"FAILED","error":error,
                "elapsedSeconds":started.elapsed().as_secs_f64()});
            save(&directory.join("summary.json"), &summary)?;
            return Ok(summary);
        }
    };
    let outputs = CODEX.outputs(&generated)?;
    ensure!(outputs.len() == 1, "expected one portrait");
    let (_, bytes) = super::super::image_data::image_bytes(&outputs[0].url)?;
    ensure!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
    std::fs::write(&output, bytes)?;
    let summary = json!({"id":id,"label":case["label"],"status":"COMPLETED",
        "requestedModel":model,"actualModel":"not returned by transport",
        "elapsedSeconds":started.elapsed().as_secs_f64(),"output":output,
        "editTarget":case["editTarget"]});
    save(&directory.join("summary.json"), &summary)?;
    println!("Portrait {id}: saved {}", output.display());
    Ok(summary)
}

#[tokio::test]
#[ignore = "requires local Codex login; consumes one generation per manifest case"]
async fn live_codex_portrait_assets() -> anyhow::Result<()> {
    let manifest = PathBuf::from(std::env::var("MSTUDIO_PORTRAIT_MANIFEST")?);
    let root = manifest.parent().context("manifest needs parent")?;
    let spec: Value = serde_json::from_slice(&std::fs::read(&manifest)?)?;
    let model = spec["model"]
        .as_str()
        .context("explicit configured model required")?;
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let mut sources = Vec::new();
    for resource in [
        "SKILL.md",
        "CORE.md",
        "references/reference-assets.md",
        "references/character-assets.md",
        "references/photographic-people.md",
        "references/scene-assets.md",
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
        !cases.is_empty() && cases.len() <= 8,
        "probe allows 1–8 cases"
    );
    let results: Vec<_> = stream::iter(cases.iter().cloned())
        .map(|case| portrait(root, model, case))
        .buffer_unordered(2)
        .collect()
        .await;
    let results = results.into_iter().collect::<anyhow::Result<Vec<_>>>()?;
    save(
        &root.join("results.json"),
        &json!({"transport":"production codex::generate",
        "model":model,"results":results}),
    )?;
    ensure!(
        results.iter().all(|r| r["status"] != "FAILED"),
        "inspect failed case summaries"
    );
    Ok(())
}
