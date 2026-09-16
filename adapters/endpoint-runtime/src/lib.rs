use domain_contract::{DeviceId, DeviceState};
use execution_contract::{ExecutionCommand, ExecutionError, ExecutionHandle, Executor};
use sensor_contract::SensorProvider;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointConfig {
    pub device_type: String,
    pub device_id: DeviceId,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum EndpointError {
    #[error("endpoint device type is empty")]
    EmptyType,
    #[error("unsupported endpoint device type: {0}")]
    UnsupportedType(String),
    #[error("backend error: {0}")]
    Backend(String),
}

pub trait BackendSdk: Executor {
    fn stop(&self) -> Result<(), String>;
}

pub type BackendFactory = Box<dyn Fn(&EndpointConfig) -> Arc<dyn BackendSdk> + Send + Sync>;

#[derive(Default)]
pub struct EndpointRegistry {
    factories: HashMap<String, BackendFactory>,
}

impl EndpointRegistry {
    pub fn register(&mut self, device_type: impl Into<String>, factory: BackendFactory) {
        self.factories.insert(device_type.into(), factory);
    }

    pub fn create(&self, config: &EndpointConfig) -> Result<Endpoint, EndpointError> {
        if config.device_type.trim().is_empty() {
            return Err(EndpointError::EmptyType);
        }
        let factory = self
            .factories
            .get(&config.device_type)
            .ok_or_else(|| EndpointError::UnsupportedType(config.device_type.clone()))?;
        Ok(Endpoint::new(factory(config)))
    }
}

pub struct Endpoint {
    backend: Arc<dyn BackendSdk>,
}

impl Endpoint {
    fn new(backend: Arc<dyn BackendSdk>) -> Self {
        Self { backend }
    }

    pub fn descriptor(&self) -> domain_contract::DeviceDescriptor {
        self.backend.descriptor()
    }
    pub fn state(&self) -> DeviceState {
        self.backend.state()
    }
    pub fn execute(&self, command: ExecutionCommand) -> Result<ExecutionHandle, ExecutionError> {
        self.backend.execute(command)
    }
    pub fn cancel(&self, task_id: &domain_contract::TaskId) -> Result<(), ExecutionError> {
        self.backend.cancel(task_id)
    }
    pub fn stop(&self) -> Result<(), EndpointError> {
        self.backend.stop().map_err(EndpointError::Backend)
    }
}

pub struct EndpointSensor<S: SensorProvider> {
    pub endpoint: Endpoint,
    pub sensors: Arc<S>,
}
