use super::*;
use std::sync::Barrier;

fn concurrent_edits(targets: [&str; 2]) -> (Vec<Value>, Value) {
    let (store, _, profile) = super::tests::fixture();
    let barrier = Barrier::new(2);
    let results =
        std::thread::scope(|scope| {
            let mut workers = vec![];
            for (index, target) in targets.into_iter().enumerate() {
                let (store, profile, barrier) = (&store, &profile, &barrier);
                workers.push(scope.spawn(move || {
                    let turn = format!("child-{index}");
                    execute(
                        store,
                        profile,
                        "p",
                        &turn,
                        "read",
                        json!({
                            "action":"inspect", "nodeIds":[target],
                        }),
                    )
                    .unwrap();
                    // Both children observe the old state before either can write.
                    barrier.wait();
                    execute(store, profile, "p", &turn, &format!("write-{index}"), json!({
                    "action":"edit", "operations":[{
                        "op":"update_node", "id":target, "text":format!("from child {index}"),
                    }],
                })).unwrap()
                }));
            }
            workers
                .into_iter()
                .map(|worker| worker.join().unwrap())
                .collect::<Vec<_>>()
        });
    let document = load(&store.db.lock().unwrap(), "p").unwrap();
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
    (results, document)
}

#[test]
fn independent_children_preserve_both_concurrent_edits() {
    let (results, document) = concurrent_edits(["a", "b"]);
    assert!(
        results.iter().all(|result| result["applied"] == true),
        "{results:?}"
    );
    assert_eq!(document["nodes"][0]["text"], "from child 0");
    assert_eq!(document["nodes"][1]["text"], "from child 1");
    assert_eq!(document["revision"], 2);
}

#[test]
fn concurrent_children_cannot_silently_overwrite_the_same_observed_target() {
    let (results, document) = concurrent_edits(["a", "a"]);
    let winner = results
        .iter()
        .position(|result| result["applied"] == true)
        .unwrap();
    let loser = &results[1 - winner];
    assert_eq!(loser["code"], "TARGET_CONFLICT", "{loser}");
    assert_eq!(loser["conflicts"][0]["code"], "TARGET_CHANGED");
    assert_eq!(document["nodes"][0]["text"], format!("from child {winner}"));
    assert_eq!(document["revision"], 1);
}
