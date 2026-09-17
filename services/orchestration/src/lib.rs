use platform::SensorProvider;
use platform::{DeviceId, TaskId};
use platform::{ExecutionCommand, ExecutionFeedback, ExecutionResult};
use platform::{Task, TaskState};
use std::collections::HashMap;
use std::sync::Arc;

use crate::device::DeviceRegistry;
use crate::error::OrchestrationError;

mod device;
mod error;

pub trait ExecutionPort: Send + Sync {
    fn execute(&self, command: ExecutionCommand) -> Result<(), String>;
    fn cancel(&self, task_id: &TaskId) -> Result<(), String>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActiveTask {
    pub task: Task,
    pub device_id: DeviceId,
    pub state: TaskState,
    pub progress: f32,
    pub phase: String,
}

pub struct Orchestrator {
    registry: DeviceRegistry,
    execution: Arc<dyn ExecutionPort>,
    sensors: Arc<dyn SensorProvider>,
    active: HashMap<TaskId, ActiveTask>,
    device_tasks: HashMap<DeviceId, TaskId>,
}

impl Orchestrator {
    pub fn new(
        registry: DeviceRegistry,
        execution: Arc<dyn ExecutionPort>,
        sensors: Arc<dyn SensorProvider>,
    ) -> Self {
        Self {
            registry,
            execution,
            sensors,
            active: HashMap::new(),
            device_tasks: HashMap::new(),
        }
    }

    pub fn registry_mut(&mut self) -> &mut DeviceRegistry {
        &mut self.registry
    }

    pub fn submit(&mut self, task: Task) -> Result<DeviceId, OrchestrationError> {
        if self.active.contains_key(&task.id) {
            return Err(OrchestrationError::Duplicate);
        }
        let device_id = self
            .registry
            .select(&task)
            .ok_or(OrchestrationError::NoDevice)?;
        if self.device_tasks.contains_key(&device_id) {
            return Err(OrchestrationError::Busy);
        }
        let command = ExecutionCommand {
            task_id: task.id.clone(),
            primitive: task.primitive.as_str().into(),
            payload: task.parameters.clone(),
            deadline_ms: task.deadline_ms,
        };
        self.execution
            .execute(command)
            .map_err(OrchestrationError::Execution)?;
        self.device_tasks.insert(device_id.clone(), task.id.clone());
        self.active.insert(
            task.id.clone(),
            ActiveTask {
                task,
                device_id: device_id.clone(),
                state: TaskState::Accepted,
                progress: 0.0,
                phase: "accepted".into(),
            },
        );
        Ok(device_id)
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
        self.device_tasks.remove(&active.device_id);
        active.state = if result.state == "succeeded" {
            TaskState::Succeeded
        } else if result.state == "canceled" {
            TaskState::Canceled
        } else {
            TaskState::Failed
        };
        Ok(active)
    }

    pub fn cancel(&mut self, id: &TaskId) -> Result<(), OrchestrationError> {
        let active = self.active.get(id).ok_or(OrchestrationError::UnknownTask)?;
        self.execution
            .cancel(id)
            .map_err(OrchestrationError::Execution)?;
        let device = active.device_id.clone();
        self.active.remove(id);
        self.device_tasks.remove(&device);
        Ok(())
    }

    pub fn sensor_provider(&self) -> &Arc<dyn SensorProvider> {
        &self.sensors
    }

    pub fn task(&self, id: &TaskId) -> Option<&ActiveTask> {
        self.active.get(id)
    }

    pub fn list_tasks(&self) -> Vec<ActiveTask> {
        self.active.values().cloned().collect()
    }
}
