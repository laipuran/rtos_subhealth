//! Parsing and structural validation of LLM diagnosis output (RFC-009 §6).

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const SEVERITY_LEVELS: [&str; 5] = ["normal", "mild", "moderate", "severe", "critical"];
const REQUIRED: [&str; 6] = [
    "severity",
    "summary",
    "possible_causes",
    "recommendations",
    "confidence",
    "disclaimer",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosisJson {
    pub severity: String,
    pub summary: String,
    pub possible_causes: Vec<String>,
    pub recommendations: Vec<String>,
    pub confidence: f64,
    pub disclaimer: String,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum SchemaError {
    #[error("no JSON object found in LLM output")]
    NoJson,
    #[error("LLM output is not a JSON object")]
    NotObject,
    #[error("missing fields: {0:?}")]
    Missing(Vec<String>),
    #[error("invalid severity: {0}")]
    InvalidSeverity(String),
    #[error("confidence is not a number")]
    InvalidConfidence,
}

/// Extract a JSON object from raw text, tolerating ```json fences and prose.
pub fn extract_json(text: &str) -> Result<Value, SchemaError> {
    let mut candidate = text.trim().to_string();
    if let Ok(re) = Regex::new(r"(?s)```(?:json)?\s*(.*?)```") {
        if let Some(caps) = re.captures(&candidate) {
            candidate = caps[1].trim().to_string();
        }
    }
    if let Ok(v) = serde_json::from_str::<Value>(&candidate) {
        return Ok(v);
    }
    let re = Regex::new(r"(?s)\{.*\}").map_err(|_| SchemaError::NoJson)?;
    let m = re.find(&candidate).ok_or(SchemaError::NoJson)?;
    serde_json::from_str::<Value>(m.as_str()).map_err(|_| SchemaError::NoJson)
}

fn to_string_list(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Array(items)) => items
            .iter()
            .map(|v| match v {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            })
            .collect(),
        Some(Value::String(s)) => vec![s.clone()],
        Some(other) => vec![other.to_string()],
        None => Vec::new(),
    }
}

/// Parse and validate an LLM response into a [`DiagnosisJson`].
pub fn parse_diagnosis(content: &str) -> Result<DiagnosisJson, SchemaError> {
    let value = extract_json(content)?;
    let Value::Object(obj) = &value else {
        return Err(SchemaError::NotObject);
    };
    let missing: Vec<String> = REQUIRED
        .iter()
        .filter(|k| !obj.contains_key(**k))
        .map(|k| k.to_string())
        .collect();
    if !missing.is_empty() {
        return Err(SchemaError::Missing(missing));
    }
    let severity = obj["severity"].as_str().unwrap_or("").to_string();
    if !SEVERITY_LEVELS.contains(&severity.as_str()) {
        return Err(SchemaError::InvalidSeverity(severity));
    }
    let confidence = obj["confidence"]
        .as_f64()
        .ok_or(SchemaError::InvalidConfidence)?;

    Ok(DiagnosisJson {
        severity,
        summary: obj["summary"].as_str().unwrap_or("").to_string(),
        possible_causes: to_string_list(obj.get("possible_causes")),
        recommendations: to_string_list(obj.get("recommendations")),
        confidence,
        disclaimer: obj["disclaimer"].as_str().unwrap_or("").to_string(),
    })
}

pub fn passes_confidence(d: &DiagnosisJson, confidence_min: f64) -> bool {
    d.confidence >= confidence_min
}
