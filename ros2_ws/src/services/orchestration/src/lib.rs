use platform::{DeviceId, TaskId};
use platform::{ExecutionError, ExecutionFeedback, ExecutionResult, ExecutionSession};
use platform::{Task, TaskState};
use std::collections::HashMap;
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

#[derive(Debug, Clone, PartialEq)]
pub struct ActiveTask {
    pub task: Task,
    pub state: TaskState,
    pub progress: f32,
    pub phase: String,
}

pub struct Orchestrator {
    execution: Arc<dyn ExecutionPort>,
    active: HashMap<TaskId, ActiveTask>,
    device_tasks: HashMap<DeviceId, TaskId>,
}

impl Orchestrator {
    pub fn new(execution: Arc<dyn ExecutionPort>) -> Self {
        Self {
            execution,
            active: HashMap::new(),
            device_tasks: HashMap::new(),
        }
    }

    pub async fn submit(&mut self, task: Task) -> Result<ExecutionSession, OrchestrationError> {
        if self.active.contains_key(&task.id) {
            return Err(OrchestrationError::Duplicate);
        }
        let device_id = task.device_id.clone();
        if self.device_tasks.contains_key(&device_id) {
            return Err(OrchestrationError::Busy);
        }
        if task.target.is_empty() {
            return Err(OrchestrationError::InvalidTarget);
        }
        let session = self
            .execution
            .execute(task.clone())
            .await
            .map_err(|error| OrchestrationError::Execution(error.to_string()))?;
        self.device_tasks.insert(device_id.clone(), task.id.clone());
        self.active.insert(
            task.id.clone(),
            ActiveTask {
                task,
                state: TaskState::Accepted,
                progress: 0.0,
                phase: "accepted".into(),
            },
        );
        Ok(session)
    }

    pub fn feedback(
        &mut self,
        feedback: ExecutionFeedback,
    ) -> Result<&ActiveTask, OrchestrationError> {
        let active = self
            .active
            .get_mut(&feedback.task_id)
            .ok_or(OrchestrationError::UnknownTask)?;
        active.state = TaskState::Running;
        active.progress = feedback.progress.clamp(0.0, 1.0);
        active.phase = feedback.phase;
        Ok(active)
    }

    pub fn complete(&mut self, result: ExecutionResult) -> Result<ActiveTask, OrchestrationError> {
        let mut active = self
            .active
            .remove(&result.task_id)
            .ok_or(OrchestrationError::UnknownTask)?;
        self.device_tasks.remove(&active.task.device_id);
        active.state = if result.state == "succeeded" {
            TaskState::Succeeded
        } else {
            TaskState::Failed
        };
        Ok(active)
    }

    pub fn task(&self, id: &TaskId) -> Option<&ActiveTask> {
        self.active.get(id)
    }
}
