use diagnosis::aggregator::{build_snapshot, Sample, Window};
use diagnosis::llm::build_messages;

#[test]
fn prompt_includes_snapshot_and_context() {
    let mut w = Window::new("mock_spo2", "spo2", 60.0);
    w.add(Sample::valid(0.0, 95.0));
    let snap = build_snapshot([&w], "anomaly");
    let (system, user) = build_messages(&snap, "SpO2 低于 90 需关注");
    assert!(system.contains("JSON"));
    assert!(user.contains("anomaly"));
    assert!(user.contains("spo2"));
    assert!(user.contains("SpO2 低于 90 需关注"));
}

#[test]
fn prompt_uses_placeholder_when_context_missing() {
    let snap = build_snapshot([], "manual");
    let (_, user) = build_messages(&snap, "");
    assert!(user.contains("（无可用资料）"));
}
