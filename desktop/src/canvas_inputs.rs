use anyhow::{Context, Result, ensure};
use serde_json::Value;

/// Validate project ownership and selected inputs before video submission.
pub fn validate(doc: &Value, submitted: &Value) -> Result<()> {
    let task = &submitted["canvasGeneration"]["task"];
    ensure!(task["kind"] == "video", "生成类型与任务不一致");
    let inputs = task["inputs"].as_array().context("缺少画布输入")?;
    let refs = submitted["references"].as_array().context("缺少参考素材")?;
    let nodes = doc["nodes"].as_array().context("项目镜头不存在")?;
    let assets = doc["assets"].as_array().context("项目素材不存在")?;
    if let Some(owner) = task["ownerId"].as_str() {
        ensure!(
            submitted["id"] == owner
                && nodes
                    .iter()
                    .any(|n| n["id"] == owner && n["kind"] == "shot"),
            "目标镜头已移除或与生成任务不一致"
        );
    } else {
        ensure!(submitted["kind"] != "shot", "镜头任务缺少目标镜头");
    }
    for input in inputs.iter().filter(|r| r["role"] != "script") {
        let id = input["assetId"].as_str().context("生成输入缺少素材 ID")?;
        ensure!(
            refs.iter().any(|r| r["assetId"] == id),
            "生成输入与参考素材不一致"
        );
        let asset = assets
            .iter()
            .find(|a| a["id"] == id)
            .context("所选素材已移除")?;
        if input["frame"] == true {
            ensure!(asset["kind"] == "image", "分镜图必须是图片");
            let node = nodes
                .iter()
                .find(|n| n["id"] == input["nodeId"])
                .context("所选镜头已移除")?;
            ensure!(
                node["shot"]["frames"]
                    .as_array()
                    .is_some_and(|frames| frames.iter().any(|f| f["assetId"] == id)),
                "此图片不属于所选镜头"
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture() -> (Value, Value) {
        let doc = json!({"assets":[{"id":"a","kind":"image"},{"id":"b","kind":"image"}],"nodes":[{"id":"s","kind":"shot","text":"open","shot":{"frames":[{"assetId":"a","prompt":"first"},{"assetId":"b","prompt":"last"}]}}]});
        let submitted = json!({"id":"s","kind":"shot","references":[{"assetId":"a"},{"assetId":"b"}],"canvasGeneration":{"task":{"kind":"video","ownerId":"s","inputs":[{"assetId":"a","nodeId":"s","frame":true,"role":"first-frame"},{"assetId":"b","nodeId":"s","frame":true,"role":"last-frame"}]}}});
        (doc, submitted)
    }

    #[test]
    fn selected_frames_work_directly_and_script_edits_do_not_require_review() {
        let (mut doc, submitted) = fixture();
        validate(&doc, &submitted).unwrap();
        doc["nodes"][0]["text"] = json!("close");
        doc["nodes"][0]["shot"]["frames"][1]["prompt"] = json!("closer");
        validate(&doc, &submitted).unwrap();
    }

    #[test]
    fn deleted_frames_assets_and_mismatched_targets_still_fail() {
        let (doc, submitted) = fixture();
        let mut missing = doc.clone();
        missing["nodes"][0]["shot"]["frames"] = json!([]);
        assert!(validate(&missing, &submitted).is_err());
        missing = doc.clone();
        missing["assets"] = json!([]);
        assert!(validate(&missing, &submitted).is_err());
        let mut changed = submitted.clone();
        changed["canvasGeneration"]["task"]["ownerId"] = json!("other");
        assert!(validate(&doc, &changed).is_err());
        changed = submitted;
        changed["references"] = json!([]);
        assert!(validate(&doc, &changed).is_err());
    }

    #[test]
    fn image_references_do_not_require_a_storyboard_frame_record() {
        let (mut doc, mut submitted) = fixture();
        doc["nodes"][0]["shot"]["frames"] = json!([]);
        for input in submitted["canvasGeneration"]["task"]["inputs"]
            .as_array_mut()
            .unwrap()
        {
            input["frame"] = json!(false);
        }
        validate(&doc, &submitted).unwrap();
    }
}
