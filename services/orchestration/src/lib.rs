use domain_contract::{DeviceDescriptor, DeviceId, TaskId};
use execution_contract::{ExecutionCommand, ExecutionFeedback, ExecutionResult};
use sensor_contract::SensorProvider;
use std::collections::HashMap;
use std::sync::Arc;
use task_contract::{Task, TaskState};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum OrchestrationError {
    #[error("no device satisfies the task requirements")]
    NoDevice,
    #[error("device is busy")]
    Busy,
    #[error("task already exists")]
    Duplicate,
    #[error("task does not exist")]
    UnknownTask,
    #[error("execution port error: {0}")]
    Execution(String),
}

#[derive(Default)]
pub struct DeviceRegistry {
    devices: HashMap<DeviceId, DeviceDescriptor>,
}

impl DeviceRegistry {
    pub fn register(&mut self, descriptor: DeviceDescriptor) {
        self.devices.insert(descriptor.id.clone(), descriptor);
    }

    pub fn get(&self, id: &DeviceId) -> Option<&DeviceDescriptor> {
        self.devices.get(id)
    }

    pub fn select(&self, task: &Task) -> Option<DeviceId> {
        if let Some(id) = &task.device_id {
            return self
                .devices
                .get(id)
                .and_then(|device| supports(device, task).then(|| device.id.clone()));
        }
        self.devices
            .values()
            .find(|device| supports(device, task))
            .map(|device| device.id.clone())
    }
}

fn supports(device: &DeviceDescriptor, task: &Task) -> bool {
    task.required_capabilities
        .iter()
        .all(|required| device.capabilities.contains(required))
        && device
            .primitives
            .iter()
            .any(|primitive| primitive == task.primitive.as_str())
}

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain_contract::SensorId;
    use futures_util::stream;
    use sensor_contract::{SensorDescriptor, SensorFilter, SensorSample, SensorStream};
    use task_contract::{Primitive, TaskTarget};

    struct Sensors;
    impl SensorProvider for Sensors {
        fn descriptors(&self) -> Vec<SensorDescriptor> {
            vec![]
        }
        fn latest(&self, _id: &SensorId) -> Option<SensorSample> {
            None
        }
        fn subscribe(&self, _filter: SensorFilter) -> SensorStream {
            Box::pin(stream::empty())
        }
    }

    struct Execution;
    impl ExecutionPort for Execution {
        fn execute(&self, _command: ExecutionCommand) -> Result<(), String> {
            Ok(())
        }
        fn cancel(&self, _task_id: &TaskId) -> Result<(), String> {
            Ok(())
        }
    }

    fn descriptor() -> DeviceDescriptor {
        DeviceDescriptor {
            id: DeviceId("endpoint-1".into()),
            name: "generic".into(),
            capabilities: vec!["navigation".into()],
            primitives: vec!["stop".into()],
            sensors: vec![],
        }
    }

    #[test]
    fn selects_by_capability_not_device_type() {
        let mut registry = DeviceRegistry::default();
        registry.register(descriptor());
        let mut orchestrator = Orchestrator::new(registry, Arc::new(Execution), Arc::new(Sensors));
        let task = Task {
            id: TaskId("task-1".into()),
            device_id: None,
            required_capabilities: vec!["navigation".into()],
            primitive: Primitive::Stop,
            target: TaskTarget::None,
            parameters: serde_json::Value::Null,
            deadline_ms: None,
        };
        assert_eq!(
            orchestrator.submit(task).unwrap(),
            DeviceId("endpoint-1".into())
        );
    }
}
