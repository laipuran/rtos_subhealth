use platform::{
    ExecutionError, ExecutionFeedback, ExecutionResult, ExecutionSession, Task, TaskRecord,
    TaskRepository, TaskRepositoryError,
};
use std::sync::Arc;
use std::{future::Future, pin::Pin};

mod error;

pub use error::OrchestrationError;

pub trait ExecutionPort: Send + Sync {
    fn validate<'a>(
        &'a self,
        task: &'a Task,
    ) -> Pin<Box<dyn Future<Output = Result<(), ExecutionError>> + Send + 'a>>;

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

    pub async fn submit(
        &self,
        task: Task,
    ) -> Result<(TaskRecord, ExecutionSession), OrchestrationError> {
        self.execution
            .validate(&task)
            .await
            .map_err(|error| OrchestrationError::Execution(error.to_string()))?;
        let record = self
            .repository
            .create_task(
                task.device_id,
                task.primitive,
                task.target,
                task.deadline_ms,
            )
            .map_err(OrchestrationError::from)?;
        let session =
            self.execution
                .execute(record.task.clone())
                .await
                .map_err(|error| {
                    match self.repository.apply_result(ExecutionResult {
                        task_id: record.task.id.clone(),
                        state: "failed".into(),
                    }) {
                        Err(repository_error) => OrchestrationError::from(repository_error),
                        Ok(_) => OrchestrationError::Execution(error.to_string()),
                    }
                })?;
        Ok((record, session))
    }

    pub fn feedback(&self, feedback: ExecutionFeedback) -> Result<TaskRecord, OrchestrationError> {
        let record = self
            .repository
            .apply_feedback(feedback)
            .map_err(OrchestrationError::from)?;
        Ok(record)
    }

    pub fn complete(&self, result: ExecutionResult) -> Result<TaskRecord, OrchestrationError> {
        let record = self
            .repository
            .apply_result(result)
            .map_err(OrchestrationError::from)?;
        Ok(record)
    }
}

impl From<TaskRepositoryError> for OrchestrationError {
    fn from(error: TaskRepositoryError) -> Self {
        match error {
            TaskRepositoryError::DuplicateTask => Self::Duplicate,
            TaskRepositoryError::BusyDevice => Self::Busy,
            TaskRepositoryError::UnknownTask => Self::UnknownTask,
            TaskRepositoryError::TerminalTask => Self::TerminalTask,
            TaskRepositoryError::InvalidTarget => Self::InvalidTarget,
            TaskRepositoryError::Storage(error) => Self::Repository(error),
        }
    }
}
