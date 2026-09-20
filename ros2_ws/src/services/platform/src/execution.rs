use crate::domain::{DeviceDescriptor, DeviceState, TaskId};
use crate::task::Task;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum ExecutionError {
    #[error("execution is busy")]
    Busy,
    #[error("execution failed: {0}")]
    Failed(String),
}

pub trait Executor: Send + Sync {
    fn descriptor(&self) -> DeviceDescriptor;
    fn execute(&self, task: Task) -> Result<(), ExecutionError>;
    fn state(&self) -> DeviceState;
}
