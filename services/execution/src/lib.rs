use domain_contract::{DeviceState, TaskId};
use execution_contract::{
    ExecutionCommand, ExecutionError, ExecutionHandle, ExecutionResult, Executor,
};
use sensor_contract::SensorProvider;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeState {
    Accepted,
    Running,
    Succeeded,
    Canceled,
    Failed(String),
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RuntimeError {
    #[error("task already active")]
    Busy,
    #[error("task is not active")]
    NotActive,
    #[error("executor error: {0}")]
    Executor(String),
}

struct ActiveExecution {
    handle: ExecutionHandle,
    state: RuntimeState,
}

pub struct ExecutionRuntime<E, S>
where
    E: Executor + 'static,
    S: SensorProvider + 'static,
{
    executor: Arc<E>,
    sensors: Arc<S>,
    active: Mutex<HashMap<TaskId, ActiveExecution>>,
}

impl<E, S> ExecutionRuntime<E, S>
where
    E: Executor + 'static,
    S: SensorProvider + 'static,
{
    pub fn new(executor: Arc<E>, sensors: Arc<S>) -> Self {
        Self {
            executor,
            sensors,
            active: Mutex::new(HashMap::new()),
        }
    }

    pub fn submit(&self, command: ExecutionCommand) -> Result<ExecutionHandle, RuntimeError> {
        let mut active = self.active.lock().expect("execution mutex poisoned");
        if active.contains_key(&command.task_id) {
            return Err(RuntimeError::Busy);
        }
        let id = command.task_id.clone();
        let handle = self
            .executor
            .execute(command)
            .map_err(|error| RuntimeError::Executor(error.to_string()))?;
        active.insert(
            id,
            ActiveExecution {
                handle: handle.clone(),
                state: RuntimeState::Running,
            },
        );
        Ok(handle)
    }

    pub fn cancel(&self, task_id: &TaskId) -> Result<(), RuntimeError> {
        self.executor
            .cancel(task_id)
            .map_err(|error| RuntimeError::Executor(error.to_string()))?;
        let mut active = self.active.lock().expect("execution mutex poisoned");
        let Some(task) = active.get_mut(task_id) else {
            return Err(RuntimeError::NotActive);
        };
        task.state = RuntimeState::Canceled;
        Ok(())
    }

    pub fn state(&self, task_id: &TaskId) -> Option<RuntimeState> {
        self.active
            .lock()
            .expect("execution mutex poisoned")
            .get(task_id)
            .map(|entry| entry.state.clone())
    }

    pub fn sensor_provider(&self) -> &Arc<S> {
        &self.sensors
    }

    pub fn device_state(&self) -> DeviceState {
        self.executor.state()
    }

    pub fn result(&self, task_id: &TaskId) -> Option<ExecutionResult> {
        self.active
            .lock()
            .expect("execution mutex poisoned")
            .get(task_id)
            .and_then(|entry| entry.handle.result())
    }
}

impl From<ExecutionError> for RuntimeError {
    fn from(value: ExecutionError) -> Self {
        Self::Executor(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain_contract::{DeviceDescriptor, DeviceId};
    use execution_contract::{ExecutionHandlePort, ExecutionResult};
    use futures_util::stream;
    use sensor_contract::{SensorDescriptor, SensorFilter, SensorSample, SensorStream};

    struct Handle;
    impl ExecutionHandlePort for Handle {
        fn result(&self) -> Option<ExecutionResult> {
            None
        }
    }

    struct Backend;
    impl Executor for Backend {
        fn descriptor(&self) -> DeviceDescriptor {
            DeviceDescriptor {
                id: DeviceId("fake".into()),
                name: "fake".into(),
                capabilities: vec![],
                primitives: vec!["stop".into()],
                sensors: vec![],
            }
        }
        fn execute(&self, _command: ExecutionCommand) -> Result<ExecutionHandle, ExecutionError> {
            Ok(Arc::new(Handle))
        }
        fn cancel(&self, _task_id: &TaskId) -> Result<(), ExecutionError> {
            Ok(())
        }
        fn state(&self) -> DeviceState {
            DeviceState {
                device_id: DeviceId("fake".into()),
                healthy: true,
                message: String::new(),
                updated_at_ms: 0,
            }
        }
    }

    struct Sensors;
    impl SensorProvider for Sensors {
        fn descriptors(&self) -> Vec<SensorDescriptor> {
            vec![]
        }
        fn latest(&self, _id: &domain_contract::SensorId) -> Option<SensorSample> {
            None
        }
        fn subscribe(&self, _filter: SensorFilter) -> SensorStream {
            Box::pin(stream::empty())
        }
    }

    #[test]
    fn execution_uses_sensor_provider_and_rejects_duplicate_task() {
        let runtime = ExecutionRuntime::new(Arc::new(Backend), Arc::new(Sensors));
        let command = ExecutionCommand {
            task_id: TaskId("one".into()),
            primitive: "stop".into(),
            payload: serde_json::Value::Null,
            deadline_ms: None,
        };
        runtime.submit(command.clone()).unwrap();
        assert!(matches!(runtime.submit(command), Err(RuntimeError::Busy)));
        assert!(runtime.sensor_provider().descriptors().is_empty());
    }
}
