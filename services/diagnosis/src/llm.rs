//! LLM prompt construction and the client seam (RFC-009 §5.5).
//!
//! The concrete OpenAI-compatible HTTP client lives in the ROS node crate where
//! an async runtime is already present; this crate owns the pure, testable
//! prompt and validation logic.

use crate::aggregator::Snapshot;

pub const SYSTEM_PROMPT: &str =
    "你是医学辅助分析助手。仅依据下面提供的【参考资料】给出健康建议级（advisory）\
分析，不得编造资料之外的结论，不做医疗诊断或报警。必须输出严格 JSON，字段为：\
severity(枚举 normal|mild|moderate|severe|critical)、summary(中文简述)、\
possible_causes(字符串数组)、recommendations(字符串数组)、confidence(0-1 浮点)、\
disclaimer(免责声明)。";

/// Build the `(system, user)` prompt, injecting RAG context as constraints.
pub fn build_messages(snapshot: &Snapshot, context: &str) -> (String, String) {
    let lines: Vec<String> = snapshot
        .sources
        .iter()
        .map(|s| {
            format!(
                "- {}: mean={:?} min={:?} max={:?} latest={:?} trend={} valid={}",
                s.data_type, s.mean, s.min, s.max, s.latest, s.trend, s.valid
            )
        })
        .collect();
    let context = if context.is_empty() {
        "（无可用资料）"
    } else {
        context
    };
    let user = format!(
        "触发类型: {}\n体征快照:\n{}\n\n【参考资料】\n{}\n\n请基于上述资料输出 JSON。",
        snapshot.trigger_type,
        lines.join("\n"),
        context
    );
    (SYSTEM_PROMPT.to_string(), user)
}

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("LLM base_url not configured")]
    Disabled,
    #[error("LLM request failed: {0}")]
    Request(String),
    #[error("LLM response malformed: {0}")]
    Malformed(String),
}

/// Client seam. Implementations may be remote (OpenAI-compatible) or fake.
pub trait LlmClient: Send + Sync {
    fn complete(&self, system: &str, user: &str) -> Result<String, LlmError>;
}
