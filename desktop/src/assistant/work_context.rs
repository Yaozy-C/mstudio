use serde_json::{Value, json};
/// UI focus is explicit reference data, never permission to edit an unrelated object.
pub fn resolve(doc: &Value, work: &Value, target: Option<&str>) -> Result<Value, String> {
    if work.is_null() {
        return Ok(Value::Null);
    }
    let view = work["view"].as_str().ok_or("缺少工作区类型")?;
    if !["script", "storyboard", "film"].contains(&view) {
        return Err("工作区类型无效".into());
    }
    if view != "script" {
        return Ok(json!({"view":view}));
    }
    let id = target.or(work["screenplayId"].as_str());
    let Some(id) = id else {
        return Ok(json!({"view":view,"task":"新建脚本"}));
    };
    let screenplay = doc["nodes"]
        .as_array()
        .and_then(|nodes| nodes.iter().find(|n| n["id"] == id))
        .ok_or("当前任务对象已移除")?;
    if screenplay["kind"] != "screenplay" {
        return Ok(json!({"view":view,"targetNodeId":id}));
    }
    let paragraph = if work["screenplayId"] == id {
        work["paragraphId"].as_str()
    } else {
        None
    };
    if let Some(paragraph) = paragraph
        && !screenplay["screenplay"]["script"]
            .as_array()
            .is_some_and(|list| list.iter().any(|s| s["id"] == paragraph))
    {
        return Err("当前段落已移除，请重新选择".into());
    }
    Ok(
        json!({"view":view,"screenplayId":id,"title":screenplay["title"],"paragraphId":paragraph,"instruction":"这是当前编辑对象。用户明确指定的范围优先；修改脚本要写回此方案的 screenplay.script，沿用原段落 ID，不创建普通文字卡片。不自动推进到生成媒体。"}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_target_overrides_focused_paragraph_and_missing_objects_fail() {
        let doc = json!({"nodes":[{"id":"a","kind":"screenplay","screenplay":{"script":[{"id":"p"}]}},{"id":"b","kind":"screenplay"}]});
        let work = json!({"view":"script","screenplayId":"a","paragraphId":"p"});
        assert_eq!(resolve(&doc, &work, None).unwrap()["paragraphId"], "p");
        assert!(resolve(&doc, &work, Some("b")).unwrap()["paragraphId"].is_null());
        assert!(resolve(&doc, &work, Some("missing")).is_err());
        assert!(
            resolve(
                &doc,
                &json!({"view":"script","screenplayId":"a","paragraphId":"gone"}),
                None
            )
            .is_err()
        );
    }
}
