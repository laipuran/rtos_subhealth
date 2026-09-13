use diagnosis::anomaly::{default_thresholds, is_anomalous, parse_thresholds, Threshold};

#[test]
fn spo2_below_90_is_anomalous() {
    let t = default_thresholds();
    assert!(is_anomalous("spo2", 88.0, &t));
    assert!(!is_anomalous("spo2", 96.0, &t));
}

#[test]
fn heart_rate_bounds_both_sides() {
    let t = default_thresholds();
    assert!(is_anomalous("heart_rate", 50.0, &t));
    assert!(is_anomalous("heart_rate", 120.0, &t));
    assert!(!is_anomalous("heart_rate", 80.0, &t));
}

#[test]
fn systolic_high_only() {
    let t = default_thresholds();
    assert!(is_anomalous("systolic_mmhg", 150.0, &t));
    assert!(!is_anomalous("systolic_mmhg", 60.0, &t));
}

#[test]
fn unknown_metric_is_never_anomalous() {
    let t = default_thresholds();
    assert!(!is_anomalous("unknown_metric", 999.0, &t));
}

#[test]
fn empty_thresholds_json_falls_back_to_default() {
    assert_eq!(parse_thresholds(""), default_thresholds());
    assert_eq!(parse_thresholds("not json"), default_thresholds());
}

#[test]
fn custom_thresholds_override_default() {
    let parsed = parse_thresholds(r#"{"spo2": {"low": 95.0, "high": null}}"#);
    assert_eq!(
        parsed.get("spo2"),
        Some(&Threshold {
            low: Some(95.0),
            high: None
        })
    );
    assert!(is_anomalous("spo2", 94.0, &parsed));
    assert!(!is_anomalous("spo2", 96.0, &parsed));
    assert!(!parsed.contains_key("heart_rate"));
}
