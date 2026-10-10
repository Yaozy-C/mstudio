use super::*;
use crate::assistant::harness::{schema, schema_definition};
fn checked(name: &str, args: Value) -> Result<Value, Value> {
    let definition = definitions()
        .into_iter()
        .find(|tool| tool.name == name)
        .unwrap();
    let issues = schema::issues(&definition.parameters, &args);
    if !issues.is_empty() {
        return Err(schema::rejection(issues));
    }
    Ok(decode(name, args).unwrap())
}
#[test]
fn object_readers_reject_cross_object_fields_and_dotted_paths() {
    for tool in definitions() {
        schema_definition::check(&tool.parameters).unwrap();
        let props = tool.parameters["properties"].as_object().unwrap();
        assert!(props.len() <= 6, "{}", tool.name);
        assert!(!props.contains_key("section"));
        for field in tool.parameters["properties"]["fields"]["items"]["enum"]
            .as_array()
            .into_iter()
            .flatten()
        {
            assert!(!field.as_str().unwrap().contains('.'));
        }
    }
    for bad in [
        json!({"ids":["s"],"fields":["shot.frames"]}),
        json!({"ids":["s"],"fields":["status"]}),
        json!({"nodeIds":["s"]}),
        json!({"ids":[]}),
        json!({}),
    ] {
        assert!(checked("mstudio_read_shots", bad).is_err());
    }
    assert!(checked("mstudio_read_assets", json!({"fields":["dialogue"]})).is_err());
    assert!(checked("mstudio_read_screenplay", json!({"paragraphIds":["p"]})).is_err());
    assert!(decode("mstudio_inspect", json!({})).is_none());
    assert!(decode("mstudio_read_unsupported", json!({})).is_none());
}
#[test]
fn direct_fields_map_to_private_reader_and_flatten_results() {
    let args = checked(
        "mstudio_read_shots",
        json!({"ids":["s"],"fields":["order","duration","frames"]}),
    )
    .unwrap();
    assert_eq!(
        args,
        json!({"action":"inspect","nodeIds":["s"],"fields":["shot.order","shot.duration","frames"]})
    );
    let raw = json!({"revision":5,"details":[{"id":"s","kind":"shot","shot":{"order":3,"duration":5.3,"frames":[{"assetId":"a"}]}}],"missingNodeIds":["missing"]});
    let value = result("mstudio_read_shots", raw);
    assert_eq!(value["items"][0]["frames"][0]["assetId"], "a");
    assert_eq!(value["items"][0]["order"], 3);
    assert!(value["items"][0].get("shot").is_none());
    assert_eq!(value["missingIds"], json!(["missing"]));
    let invalid = result(
        "mstudio_read_shots",
        json!({"details":[{"id":"script","kind":"screenplay"}]}),
    );
    assert_eq!(invalid["invalidKindIds"], json!(["script"]));
}
#[test]
fn screenplay_reads_have_explicit_parent_and_paragraph_scope() {
    let args = checked("mstudio_read_screenplay",json!({"id":"script","paragraphIds":["p3"],"fields":["action","duration"],"textOffset":1000})).unwrap();
    assert_eq!(args["nodeIds"], json!(["script"]));
    assert_eq!(args["scriptFields"], json!(["action", "duration"]));
    assert_eq!(args["textOffset"], 1000);
    let value = result(
        "mstudio_read_screenplay",
        json!({"details":[{"id":"script","kind":"screenplay","screenplay":{"script":[{"id":"p3","action":{"text":"saved"}}],"nextScriptOffset":10}}]}),
    );
    assert_eq!(value["items"][0]["script"][0]["id"], "p3");
    assert_eq!(value["items"][0]["nextScriptOffset"], 10);
}
#[test]
fn split_reads_run_through_real_domain_and_preserve_generation_continuation() {
    let doc = json!({"id":"p","name":"Test","revision":0,"brief":"","width":1080,"height":1920,"fps":30,"nodes":[
        {"id":"script","kind":"screenplay","title":"Script","text":"","screenplay":{"script":[{"id":"p1","title":"Opening","duration":3,"action":"Saved action"}]}},
        {"id":"s","kind":"shot","title":"Shot","text":"Saved staging","shot":{"screenplayId":"script","scriptId":"p1","order":1,"duration":3,"frames":[],"prompt":"Saved prompt","dialogue":"Hello"}}
    ],"assets":[],"clips":[],"tracks":[],"captions":[{"id":"caption","text":"Hi","start":0,"end":1}],"production":{"drafts":{"task":{"key":"task","kind":"video","prompt":"Prompt","status":"AWAITING_CONFIRMATION","turnId":"turn","createdAt":1,"mode":"single","modelId":"model","inputs":[],"position":{"x":0,"y":0}}}}});
    for (name, args) in [
        ("mstudio_read_shots", json!({"ids":["s"]})),
        (
            "mstudio_read_screenplay",
            json!({"id":"script","paragraphIds":["p1"]}),
        ),
        ("mstudio_read_captions", json!({"fields":["text","end"]})),
        (
            "mstudio_read_generation",
            json!({"taskKey":"task","fields":["status"]}),
        ),
    ] {
        let decoded = checked(name, args).unwrap();
        assert!(
            schema::issues(&crate::assistant::tool_schema::schema(), &decoded).is_empty(),
            "{name}: {decoded}"
        );
        let raw = crate::project_service::runtime::execute(
            json!({"action":"inspect","document":doc,"args":decoded}),
        )
        .unwrap();
        let value = result(name, raw);
        let item = &value["items"][0];
        match name {
            "mstudio_read_shots" => {
                assert_eq!(item["text"], "Saved staging");
                assert_eq!(item["prompt"]["text"], "Saved prompt");
                assert!(item.get("shot").is_none());
            }
            "mstudio_read_screenplay" => {
                assert_eq!(item["script"][0]["action"]["text"], "Saved action")
            }
            "mstudio_read_captions" => assert_eq!(item["end"], 1),
            _ => {
                assert_eq!(item["status"], "AWAITING_CONFIRMATION");
                assert!(item["continuation"].is_object());
            }
        }
    }
}
