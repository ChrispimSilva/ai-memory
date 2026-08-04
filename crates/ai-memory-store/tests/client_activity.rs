//! `client_activity` — per-client MCP tool-call buckets.

use ai_memory_store::Store;

#[tokio::test]
async fn buckets_accumulate_and_aggregate_deterministically() {
    let tmp = tempfile::tempdir().unwrap();
    let store = Store::open(tmp.path()).unwrap();

    // Two flushes into the same (client, day) bucket accumulate; a second
    // client and a second day stay separate.
    store
        .writer
        .bump_client_activity(vec![
            ("vscode".into(), 100, 3, 1),
            ("claude-desktop".into(), 100, 2, 0),
        ])
        .await
        .unwrap();
    store
        .writer
        .bump_client_activity(vec![
            ("vscode".into(), 100, 2, 0),
            ("vscode".into(), 101, 1, 1),
        ])
        .await
        .unwrap();

    let all = store.reader.client_activity_since(None).await.unwrap();
    let seen: Vec<(&str, u64, u64)> = all
        .iter()
        .map(|c| (c.client.as_str(), c.reads, c.writes))
        .collect();
    assert_eq!(
        seen,
        vec![("vscode", 6, 2), ("claude-desktop", 2, 0)],
        "summed across days, volume-desc",
    );

    // The window bound is inclusive on the day bucket.
    let recent = store.reader.client_activity_since(Some(101)).await.unwrap();
    let seen: Vec<(&str, u64, u64)> = recent
        .iter()
        .map(|c| (c.client.as_str(), c.reads, c.writes))
        .collect();
    assert_eq!(seen, vec![("vscode", 1, 1)]);

    // A future bound excludes everything rather than ignoring the argument.
    assert!(
        store
            .reader
            .client_activity_since(Some(4_000_000))
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn equal_volumes_keep_a_stable_name_order() {
    let tmp = tempfile::tempdir().unwrap();
    let store = Store::open(tmp.path()).unwrap();
    store
        .writer
        .bump_client_activity(vec![("zed".into(), 10, 1, 0), ("cursor".into(), 10, 1, 0)])
        .await
        .unwrap();
    let first = store.reader.client_activity_since(None).await.unwrap();
    let again = store.reader.client_activity_since(None).await.unwrap();
    assert_eq!(first, again);
    assert_eq!(first[0].client, "cursor", "name tiebreak, not scan order");
}
