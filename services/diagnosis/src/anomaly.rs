//! Rule-based anomaly detection with configurable thresholds (RFC-009 §6, §9.3).

use std::collections::HashMap;

use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Threshold {
    pub low: Option<f64>,
    pub high: Option<f64>,
}

pub type Thresholds = HashMap<String, Threshold>;

/// Default `(low, high)` bounds outside which a metric is abnormal.
pub fn default_thresholds() -> Thresholds {
    let mut t = Thresholds::new();
    t.insert(
        "spo2".into(),
        Threshold {
            low: Some(90.0),
            high: None,
        },
    );
    t.insert(
        "heart_rate".into(),
        Threshold {
            low: Some(60.0),
            high: Some(100.0),
        },
    );
    t.insert(
        "systolic_mmhg".into(),
        Threshold {
            low: None,
            high: Some(140.0),
        },
    );
    t.insert(
        "diastolic_mmhg".into(),
        Threshold {
            low: None,
            high: Some(90.0),
        },
    );
    t.insert(
        "body_temp_c".into(),
        Threshold {
            low: Some(35.0),
            high: Some(37.3),
        },
    );
    t.insert(
        "respiratory_rate".into(),
        Threshold {
            low: Some(12.0),
            high: Some(20.0),
        },
    );
    t
}

pub fn is_anomalous(data_type: &str, value: f64, thresholds: &Thresholds) -> bool {
    let Some(rule) = thresholds.get(data_type) else {
        return false;
    };
    if let Some(low) = rule.low {
        if value < low {
            return true;
        }
    }
    if let Some(high) = rule.high {
        if value > high {
            return true;
        }
    }
    false
}

/// Parse a JSON threshold table. Returns the default table on empty/invalid
/// input, matching the legacy `parse_thresholds`.
pub fn parse_thresholds(json: &str) -> Thresholds {
    if json.trim().is_empty() {
        return default_thresholds();
    }
    let Ok(parsed) = serde_json::from_str::<Value>(json) else {
        return default_thresholds();
    };
    let Some(obj) = parsed.as_object() else {
        return default_thresholds();
    };
    let mut out = Thresholds::new();
    for (key, value) in obj {
        let Some(entry) = value.as_object() else {
            continue;
        };
        let num = |v: Option<&Value>| v.and_then(Value::as_f64);
        out.insert(
            key.clone(),
            Threshold {
                low: num(entry.get("low")),
                high: num(entry.get("high")),
            },
        );
    }
    if out.is_empty() {
        default_thresholds()
    } else {
        out
    }
}
