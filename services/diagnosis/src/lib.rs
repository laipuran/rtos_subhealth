//! ROS Subhealth diagnosis service.
//!
//! Windowed aggregation, rule-based anomaly detection, RAG retrieval and
//! LLM-backed structured diagnosis (RFC-009). The core logic is pure Rust so it
//! can be unit-tested without ROS; the ROS node lives in `ros2_ws`.

pub mod aggregator;
pub mod anomaly;
pub mod llm;
pub mod schema;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
