use crate::{database::Store, models::media};
use anyhow::{Result, ensure};
use serde_json::{Value, json};

pub fn catalog(store: &Store, args: &Value) -> Result<Value> {
    let models: Vec<_> = media::resolved(&store.db.lock().unwrap())?
        .into_iter()
        .filter(|m| m.enabled)
        .collect();
    if let Some(id) = args["mediaModelId"].as_str() {
        let model = models
            .iter()
            .find(|m| m.id == id)
            .ok_or_else(|| anyhow::anyhow!("媒体模型不存在或已停用"))?;
        return crate::model_adapters::prompt_rules::guidance(&store.db.lock().unwrap(), model);
    }
    let offset = args["offset"].as_u64().unwrap_or(0).min(100) as usize;
    let entries: Vec<_> = models.iter().skip(offset).take(20)
        .map(|m| json!({"id":m.id,"name":m.name,"kind":m.kind,"provider":m.plugin,"endpoint":m.endpoint})).collect();
    Ok(
        json!({"models":entries,"nextOffset":if offset+entries.len()<models.len(){Some(offset+entries.len())}else{None},
        "usage":"按输出类型选择模型；通过 edit operations request_generation 创建聊天时间记录中的生成任务卡，mediaKind 指定输出类型。使用用户选定模型，不擅自替换。任务提交不等于生成完成。写 Prompt 前使用 mediaModelId 读取所选模型规则，若已注入相同模型的完整规则可复用。具体输入由模型 adapter 校验，不要猜测模型能力。"}),
    )
}
pub fn validate_selection(store: &Store, args: &Value) -> Result<()> {
    for op in args["operations"].as_array().into_iter().flatten() {
        if op["op"] == "request_generation" && op.get("mediaModelId").is_some() {
            let id = op["mediaModelId"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("媒体模型 ID 无效"))?;
            let models = media::resolved(&store.db.lock().unwrap())?;
            let model = models
                .iter()
                .find(|m| m.id == id)
                .ok_or_else(|| anyhow::anyhow!("媒体模型不存在，请先读取模型目录"))?;
            ensure!(model.enabled, "媒体模型已停用，请重新选择");
            if let Some(kind) = op["mediaKind"].as_str() {
                ensure!(model.kind == kind, "媒体模型类型与生成任务不匹配");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_catalog_excludes_disabled_and_rejects_invalid_selection() {
        let root =
            std::env::temp_dir().join(format!("mstudio-media-tools-{}", mstudio::media::id()));
        let store = Store::open(root.clone()).unwrap();
        let models = json!([
            {"id":"image","name":"Image","kind":"image","plugin":"fal","endpoint":"fal-ai/flux/schnell","params":{},"enabled":true},
            {"id":"video","name":"Video","kind":"video","plugin":"fal","endpoint":"fal-ai/video","params":{},"enabled":false}
        ]);
        store
            .set_setting("media-models", &models.to_string())
            .unwrap();
        let result = catalog(&store, &json!({})).unwrap();
        assert_eq!(result["models"].as_array().unwrap().len(), 1);
        assert_eq!(result["models"][0]["id"], "image");
        for (id, valid) in [("image", true), ("video", false), ("missing", false)] {
            let args =
                json!({"operations":[{"op":"request_generation","id":"shot","mediaModelId":id}]});
            assert_eq!(validate_selection(&store, &args).is_ok(), valid);
        }
        assert!(validate_selection(&store, &json!({"operations":[{"op":"request_generation","mediaModelId":"image","mediaKind":"video"}]})).is_err());
        let profiles = super::super::profiles::builtins();
        let production = profiles.iter().find(|p| p.id == "production").unwrap();
        let reviewer = profiles.iter().find(|p| p.id == "reviewer").unwrap();
        assert!(super::super::profiles::allows(production, "models"));
        assert!(!super::super::profiles::allows(reviewer, "models"));
        let profile = serde_json::to_value(production).unwrap();
        assert!(profile.get("modelId").is_none());
        assert!(profile.get("mediaModelIds").is_none());
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }
}
