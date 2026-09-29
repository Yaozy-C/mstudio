use super::*;

#[test]
fn inspect_accepts_exact_task_lookup_but_not_edit_revision() {
    let schema = tool_schema::for_profile(&profiles::defaults(""));
    let fields = ACTIONS
        .iter()
        .find(|(name, _, _)| *name == "inspect")
        .unwrap()
        .2;
    let properties: serde_json::Map<_, _> = fields
        .iter()
        .map(|field| ((*field).to_owned(), schema["properties"][*field].clone()))
        .collect();
    let exposed = json!({"type":"object","properties":properties,"additionalProperties":false});
    assert!(super::super::schema::validate(&exposed, &json!({"section":"generation","turnId":"batch","status":"FAILED","fields":["status","targetNodeId","resultAssetIds","error","trackingPaused"]})).is_ok());
    assert!(super::super::schema::validate(&exposed, &json!({"nodeIds":["plan"],"fields":["script"],"paragraphIds":["p4"],"scriptFields":["duration"]})).is_ok());
    assert!(
        super::super::schema::validate(
            &exposed,
            &json!({"section":"generation","taskKey":"retry:target"})
        )
        .is_ok()
    );
    assert!(
        super::super::schema::validate(&exposed, &json!({"section":"generation","taskKey":123}))
            .is_err()
    );
    assert!(
        super::super::schema::validate(
            &exposed,
            &json!({"nodeIds":["shot_01_hook"],"revision":680})
        )
        .is_err()
    );
}
