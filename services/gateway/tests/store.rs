use gateway::model::task::{Goal, TaskRecord, TaskUpdate};
use gateway::store::task_store::TaskStore;

#[test]
fn round_trip_task() {
    let store = TaskStore::open_in_memory().unwrap();
    let rec = TaskRecord::new("T-1", Goal::new("go_to_tag").with_tags(vec![42]));
    store.add(&rec).unwrap();

    let got = store.get("T-1").unwrap().unwrap();
    assert_eq!(got.goal.type_, "go_to_tag");
    assert_eq!(got.goal.target_tags, vec![42]);
    assert_eq!(got.state, "accepted");

    store
        .update(
            "T-1",
            &TaskUpdate {
                state: Some("running".into()),
                progress: Some(0.5),
                ..Default::default()
            },
        )
        .unwrap();
    let updated = store.get("T-1").unwrap().unwrap();
    assert_eq!(updated.state, "running");
    assert_eq!(updated.progress, 0.5);
    assert_eq!(store.list_active().unwrap().len(), 1);
}

#[test]
fn final_state_round_trips() {
    let store = TaskStore::open_in_memory().unwrap();
    store
        .add(&TaskRecord::new("T-2", Goal::new("hold")))
        .unwrap();
    store
        .update(
            "T-2",
            &TaskUpdate {
                state: Some("succeeded".into()),
                final_state: Some("succeeded".into()),
                ..Default::default()
            },
        )
        .unwrap();
    let rec = store.get("T-2").unwrap().unwrap();
    assert_eq!(rec.final_state.as_deref(), Some("succeeded"));
    assert!(store.list_active().unwrap().is_empty());
}

#[test]
fn unknown_task_is_none() {
    let store = TaskStore::open_in_memory().unwrap();
    assert!(store.get("missing").unwrap().is_none());
    assert!(!store.exists("missing").unwrap());
}
