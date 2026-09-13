use gateway::api::ws::{Event, EventHub};

#[tokio::test]
async fn hub_broadcasts_to_all_subscribers() {
    let hub = EventHub::new(16);
    let mut a = hub.subscribe();
    let mut b = hub.subscribe();

    hub.broadcast(Event::task(
        "T-1",
        "feedback",
        serde_json::json!({"progress": 0.5, "state": "running"}),
    ));

    let ea = a.recv().await.unwrap();
    let eb = b.recv().await.unwrap();
    assert_eq!(ea.0, eb.0);
    assert_eq!(ea.0["goal_id"], "T-1");
    assert_eq!(ea.0["event"], "feedback");
    assert_eq!(ea.0["progress"], 0.5);
}

#[tokio::test]
async fn diagnosis_event_is_tagged() {
    let hub = EventHub::new(4);
    let mut rx = hub.subscribe();
    hub.broadcast(Event::diagnosis(serde_json::json!({
        "diagnosis_id": "D-1", "severity": "mild"
    })));
    let ev = rx.recv().await.unwrap();
    assert_eq!(ev.0["event"], "diagnosis");
    assert_eq!(ev.0["diagnosis_id"], "D-1");
}
