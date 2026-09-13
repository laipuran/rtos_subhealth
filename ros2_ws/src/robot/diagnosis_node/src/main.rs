//! Physiological diagnosis node.
//!
//! Subscribes to the per-sensor `PhysioSample` topics, maintains rolling windows
//! (via the `diagnosis` crate), publishes aggregated `VitalsStream` at 1 Hz, and
//! emits a rule-based `DiagnosisResult` when an anomaly is detected (with a
//! cooldown). LLM/RAG enrichment is a later layer on top of this.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use rclrs::*;

use diagnosis::aggregator::{build_snapshot, Sample, SourceSnapshot, Window};
use diagnosis::anomaly::{default_thresholds, is_anomalous, Thresholds};
use diagnosis_interfaces::msg::{DiagnosisMetric, DiagnosisResult, PhysioSample, VitalsStream};

const WINDOW_S: f64 = 60.0;
const DIAG_COOLDOWN_S: f64 = 30.0;
const TOPICS: [&str; 6] = [
    "/physio/mock_spo2",
    "/physio/mock_heart_rate",
    "/physio/mock_bp_systolic",
    "/physio/mock_bp_diastolic",
    "/physio/mock_body_temp",
    "/physio/mock_respiratory_rate",
];

fn now_s() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

fn metric_from(s: &SourceSnapshot) -> DiagnosisMetric {
    let mut m = DiagnosisMetric::default();
    m.data_src = s.data_src.clone();
    m.data_type = s.data_type.clone();
    m.latest = s.latest.unwrap_or(0.0) as f32;
    m.mean = s.mean.unwrap_or(0.0) as f32;
    m.min = s.min.unwrap_or(0.0) as f32;
    m.max = s.max.unwrap_or(0.0) as f32;
    m.trend = s.trend.clone();
    m.valid = s.valid;
    m
}

fn main() -> Result<()> {
    let context = Context::default_from_env()?;
    let mut executor = context.create_basic_executor();
    let node = executor.create_node("diagnosis_node")?;

    let windows: Arc<Mutex<HashMap<String, Window>>> = Arc::new(Mutex::new(HashMap::new()));
    let thresholds: Arc<Thresholds> = Arc::new(default_thresholds());
    let last_diag = Arc::new(Mutex::new(0.0_f64));

    let result_pub = node.create_publisher::<DiagnosisResult>("/diagnosis/results")?;
    let vitals_pub = node.create_publisher::<VitalsStream>("/diagnosis/monitor")?;

    let mut subscriptions = Vec::new();
    for topic in TOPICS {
        let windows = Arc::clone(&windows);
        let thresholds = Arc::clone(&thresholds);
        let last_diag = Arc::clone(&last_diag);
        let result_pub = result_pub.clone();
        subscriptions.push(node.create_subscription::<PhysioSample, _>(
            topic,
            move |msg: PhysioSample| {
            let t = now_s();
            let src = msg.data_src.clone();
            let data_type = msg.data_type.clone();
            let value = msg.data as f64;

            let snapshot = {
                let mut map = windows.lock().unwrap();
                let window = map.entry(src.clone()).or_insert_with(|| {
                    Window::new(src.clone(), data_type.clone(), WINDOW_S)
                });
                window.prune(t);
                window.add(if msg.valid {
                    Sample::valid(t, value)
                } else {
                    Sample::invalid(t)
                });
                build_snapshot(map.values(), "anomaly")
            };

            let anomalous: Vec<String> = snapshot
                .sources
                .iter()
                .filter(|s| {
                    s.valid && is_anomalous(&s.data_type, s.latest.unwrap_or(0.0), thresholds.as_ref())
                })
                .map(|s| format!("{}={:.1}", s.data_type, s.latest.unwrap_or(0.0)))
                .collect();
            if anomalous.is_empty() {
                return;
            }

            let proceed = {
                let mut last = last_diag.lock().unwrap();
                if t - *last >= DIAG_COOLDOWN_S {
                    *last = t;
                    true
                } else {
                    false
                }
            };
            if !proceed {
                return;
            }

            let mut result = DiagnosisResult::default();
            result.diagnosis_id = format!("diag-{}", (t * 1000.0) as u64);
            result.source_ids = snapshot.sources.iter().map(|s| s.data_src.clone()).collect();
            result.trigger_type = "anomaly".into();
            result.severity = if anomalous.len() >= 2 {
                "severe".into()
            } else {
                "moderate".into()
            };
            result.summary = format!("规则检测到异常指标: {}", anomalous.join(", "));
            result.possible_causes = vec![
                "传感器噪声或测量误差".into(),
                "受试者生理状态波动".into(),
            ];
            result.recommendations = vec![
                "复测并人工确认".into(),
                "必要时联系医护人员".into(),
            ];
            result.confidence = 0.6;
            result.disclaimer = "仅供参考，不构成医疗建议".into();
            result.metrics = snapshot.sources.iter().map(metric_from).collect();
            let _ = result_pub.publish(&result);
        },
        )?);
    }

    let vitals_windows = Arc::clone(&windows);
    let _vitals_timer = node.create_timer_repeating(Duration::from_millis(1000), move || {
        let map = vitals_windows.lock().unwrap();
        let snapshot = build_snapshot(map.values(), "periodic");
        let mut msg = VitalsStream::default();
        msg.metrics = snapshot.sources.iter().map(metric_from).collect();
        let _ = vitals_pub.publish(&msg);
    })?;

    executor.spin(SpinOptions::default()).first_error()?;
    Ok(())
}
