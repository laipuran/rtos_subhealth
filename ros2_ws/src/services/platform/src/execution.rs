use crate::domain::TaskId;
use futures_core::Stream;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionFeedback {
    pub task_id: TaskId,
    pub progress: f32,
    pub phase: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub task_id: TaskId,
    pub state: String,
}

pub type ExecutionFeedbackStream =
    Pin<Box<dyn Stream<Item = Result<ExecutionFeedback, ExecutionError>> + Send>>;
pub type ExecutionResultFuture =
    Pin<Box<dyn Future<Output = Result<ExecutionResult, ExecutionError>> + Send>>;

pub struct ExecutionSession {
    pub feedback: ExecutionFeedbackStream,
    pub result: ExecutionResultFuture,
}

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum ExecutionError {
    #[error("execution failed: {0}")]
    Failed(String),
}
