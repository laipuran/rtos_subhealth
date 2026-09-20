use tokio::sync::{mpsc, oneshot};

use crate::RosTaskError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecuteCommand {
    pub task_id: String,
    pub device_id: String,
    pub primitive: PrimitiveCommand,
    pub target: Vec<i32>,
    pub deadline_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveCommand {
    GoToTag,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskFeedback {
    pub task_id: String,
    pub progress: f32,
    pub phase: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinalState {
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskResult {
    pub task_id: String,
    pub final_state: FinalState,
}

pub struct TaskSession {
    pub task_id: String,
    pub feedback: mpsc::Receiver<Result<TaskFeedback, RosTaskError>>,
    pub result: oneshot::Receiver<Result<TaskResult, RosTaskError>>,
}
