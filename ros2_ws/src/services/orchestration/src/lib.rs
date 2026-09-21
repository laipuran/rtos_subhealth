use platform::{
    ExecutionError, ExecutionFeedback, ExecutionResult, ExecutionSession, Task, TaskRecord,
    TaskRepository, TaskRepositoryError, TaskState,
};
use std::sync::Arc;
use std::{future::Future, pin::Pin};

mod error;

pub use error::OrchestrationError;

pub trait ExecutionPort: Send + Sync {
    fn execute(
        &self,
        task: Task,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionSession, ExecutionError>> + Send + '_>>;
}

pub struct Orchestrator {
    execution: Arc<dyn ExecutionPort>,
    repository: Arc<dyn TaskRepository>,
}

impl Orchestrator {
    pub fn new(execution: Arc<dyn ExecutionPort>, repository: Arc<dyn TaskRepository>) -> Self {
        Self {
            execution,
            repository,
        }
    }

    pub async fn submit(&self, task: Task) -> Result<ExecutionSession, OrchestrationError> {
        let record = self
            .repository
            .create_task(
                task.device_id,
                task.primitive,
                task.target,
                task.deadline_ms,
            )
            .map_err(OrchestrationError::from)?;
        let session = self
            .execution
            .execute(record.task.clone())
            .await
            .map_err(|error| {
                let _ = self.repository.apply_result(ExecutionResult {
                    task_id: record.task.id.clone(),
                    state: "failed".into(),
                });
                OrchestrationError::Execution(error.to_string())
            })?;
        Ok(session)
    }

    pub fn feedback(&self, feedback: ExecutionFeedback) -> Result<TaskRecord, OrchestrationError> {
        reject_if_terminal(&feedback.task_id, self.repository.as_ref())?;
        let record = self
            .repository
            .apply_feedback(feedback)
            .map_err(OrchestrationError::from)?;
        Ok(record)
    }

    pub fn complete(&self, result: ExecutionResult) -> Result<TaskRecord, OrchestrationError> {
        reject_if_terminal(&result.task_id, self.repository.as_ref())?;
        let record = self
            .repository
            .apply_result(result)
            .map_err(OrchestrationError::from)?;
        Ok(record)
    }
}

fn reject_if_terminal(
    task_id: &platform::TaskId,
    repository: &dyn TaskRepository,
) -> Result<(), OrchestrationError> {
    let record = repository
        .get_task(task_id)
        .map_err(OrchestrationError::from)?;
    if matches!(record.state, TaskState::Succeeded | TaskState::Failed) {
        return Err(OrchestrationError::TerminalTask);
    }
    Ok(())
}

impl From<TaskRepositoryError> for OrchestrationError {
    fn from(error: TaskRepositoryError) -> Self {
        match error {
            TaskRepositoryError::DuplicateTask => Self::Duplicate,
            TaskRepositoryError::BusyDevice => Self::Busy,
            TaskRepositoryError::UnknownTask => Self::UnknownTask,
            TaskRepositoryError::InvalidTarget => Self::InvalidTarget,
            TaskRepositoryError::Storage(error) => Self::Repository(error),
        }
    }
}
