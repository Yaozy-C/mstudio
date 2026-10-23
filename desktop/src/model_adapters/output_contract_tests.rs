use super::*;
use serde_json::json;

fn two_outputs() -> Vec<ModelOutput> {
    vec![
        ModelOutput {
            kind: "image".into(),
            url: "https://example.com/a.png".into(),
        },
        ModelOutput {
            kind: "image".into(),
            url: "https://example.com/b.png".into(),
        },
    ]
}
#[test]
fn completion_requires_current_outputs_and_writes_their_actual_count() {
    let mut job = json!({"status":"IN_PROGRESS","outputCount":99});
    for value in [
        json!({"status":"COMPLETED"}),
        json!({"status":"COMPLETED","result":{"outputs":two_outputs()}}),
        json!({"status":"COMPLETED","outputs":[]}),
    ] {
        let before = job.clone();
        let update: ProviderUpdate = serde_json::from_value(value).unwrap();
        assert!(update.apply(&mut job).is_err());
        assert_eq!(job, before);
    }
    ProviderUpdate::completed(two_outputs(), json!({"provider":"raw"}))
        .unwrap()
        .apply(&mut job)
        .unwrap();
    assert_eq!(job["outputCount"], 2);
    assert_eq!(job["outputs"][1]["url"], "https://example.com/b.png");
    let cached = ProviderUpdate::completed_job(&job).unwrap();
    cached.apply(&mut job).unwrap();
    assert_eq!(job["result"], json!({"provider":"raw"}));
    job["outputCount"] = json!(1);
    assert!(ProviderUpdate::completed_job(&job).is_err());
}
#[test]
fn failed_and_cancelled_updates_preserve_actual_partial_results() {
    for status in ["FAILED", "CANCELLED"] {
        let mut job = json!({"status":"IN_PROGRESS"});
        let update: ProviderUpdate =
            serde_json::from_value(json!({"status":status,"outputs":two_outputs()})).unwrap();
        update.apply(&mut job).unwrap();
        assert_eq!(job["outputCount"], 2);
        let update: ProviderUpdate = serde_json::from_value(json!({"status":status})).unwrap();
        update.apply(&mut job).unwrap();
        assert_eq!(job["outputs"].as_array().unwrap().len(), 2);
        assert_eq!(job["outputCount"], 2);
    }
}
