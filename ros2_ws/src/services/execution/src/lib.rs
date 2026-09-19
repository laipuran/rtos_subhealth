use platform::SensorProvider;
use platform::{DeviceState, TaskId};
use platform::{ExecutionCommand, ExecutionError, ExecutionHandle, ExecutionResult, Executor};
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
