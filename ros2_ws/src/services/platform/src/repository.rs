use crate::execution::{ExecutionFeedback, ExecutionResult};
use crate::task::{Primitive, TaskRecord};
use crate::{DeviceId, TaskId};

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum TaskRepositoryError {
    #[error("task already exists")]
    DuplicateTask,
    #[error("device is busy")]
    BusyDevice,
    #[error("task does not exist")]
    UnknownTask,
    #[error("task target is invalid")]
    InvalidTarget,
    #[error("task storage failed: {0}")]
    Storage(String),
}

/// The single source of truth for accepted task records and their transitions.
///
/// Each operation is atomic from its caller's perspective. Implementations must
/// retain terminal records while considering only non-terminal records active for
/// device occupancy.
pub trait TaskRepository: Send + Sync {
    fn create_task(
        &self,
        device_id: DeviceId,
        primitive: Primitive,
        target: Vec<i32>,
        deadline_ms: Option<u64>,
    ) -> Result<TaskRecord, TaskRepositoryError>;

    fn get_task(&self, task_id: &TaskId) -> Result<TaskRecord, TaskRepositoryError>;

    fn list_tasks(&self) -> Result<Vec<TaskRecord>, TaskRepositoryError>;

    fn apply_feedback(
        &self,
        feedback: ExecutionFeedback,
    ) -> Result<TaskRecord, TaskRepositoryError>;

    fn apply_result(&self, result: ExecutionResult) -> Result<TaskRecord, TaskRepositoryError>;
}
