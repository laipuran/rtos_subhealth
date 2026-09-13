//! Mock physiological sensor publisher.
//!
//! Publishes a `PhysioSample` per sensor at 1 Hz. Set `PHYSIO_SCENARIO=anomaly`
//! to intermittently emit out-of-range values for testing the diagnosis layer.

use std::time::Duration;

use anyhow::Result;
use rclrs::*;

use diagnosis_interfaces::msg::PhysioSample;

// topic, data_src, data_type, normal value, anomalous value
const SENSORS: [(&str, &str, &str, f32, f32); 6] = [
    ("/physio/mock_spo2", "mock_spo2", "spo2", 97.0, 88.0),
    ("/physio/mock_heart_rate", "mock_heart_rate", "heart_rate", 75.0, 130.0),
    (
        "/physio/mock_bp_systolic",
        "mock_bp_systolic",
        "systolic_mmhg",
        120.0,
        160.0,
    ),
    (
        "/physio/mock_bp_diastolic",
        "mock_bp_diastolic",
        "diastolic_mmhg",
        78.0,
        100.0,
    ),
    ("/physio/mock_body_temp", "mock_body_temp", "body_temp_c", 36.6, 38.6),
    (
        "/physio/mock_respiratory_rate",
        "mock_respiratory_rate",
        "respiratory_rate",
        16.0,
        26.0,
    ),
];

fn main() -> Result<()> {
    let context = Context::default_from_env()?;
    let mut executor = context.create_basic_executor();
    let node = executor.create_node("physio_mock")?;

    let anomalous = std::env::var("PHYSIO_SCENARIO").as_deref() == Ok("anomaly");

    let mut publishers = Vec::new();
    for (topic, src, data_type, normal, anomaly) in SENSORS {
        let publisher = node.create_publisher::<PhysioSample>(topic)?;
        publishers.push((src, data_type, normal, anomaly, publisher));
    }

    let mut tick: u32 = 0;
    let _timer = node.create_timer_repeating(Duration::from_secs(1), move || {
        tick = tick.wrapping_add(1);
        let emit_anomaly = anomalous && tick % 5 == 0;
        for (src, data_type, normal, anomaly, publisher) in &publishers {
            let mut msg = PhysioSample::default();
            msg.data_src = (*src).into();
            msg.data_type = (*data_type).into();
            let base = if emit_anomaly { *anomaly } else { *normal };
            msg.data = base + (tick as f32 * 0.7).sin() * 0.5;
            msg.valid = true;
            let _ = publisher.publish(&msg);
        }
    })?;

    executor.spin(SpinOptions::default()).first_error()?;
    Ok(())
}
