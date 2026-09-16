use domain_contract::{DeviceDescriptor, DeviceState, TaskId};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionCommand {
    pub task_id: TaskId,
    pub primitive: String,
    pub payload: serde_json::Value,
    pub deadline_ms: Option<u64>,
}

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
    pub error_code: Option<String>,
    pub message: String,
}

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum ExecutionError {
    #[error("unsupported command: {0}")]
    UnsupportedCommand(String),
    #[error("execution is busy")]
    Busy,
    #[error("execution failed: {0}")]
    Failed(String),
}

pub trait Executor: Send + Sync {
    fn descriptor(&self) -> DeviceDescriptor;
    fn execute(&self, command: ExecutionCommand) -> Result<ExecutionHandle, ExecutionError>;
    fn cancel(&self, task_id: &TaskId) -> Result<(), ExecutionError>;
    fn state(&self) -> DeviceState;
}

pub type ExecutionHandle = Arc<dyn ExecutionHandlePort>;

pub trait ExecutionHandlePort: Send + Sync {
    fn result(&self) -> Option<ExecutionResult>;
}
