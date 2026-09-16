use domain_contract::{DeviceDescriptor, DeviceState, TaskId};
use endpoint_runtime::{BackendSdk, EndpointConfig, EndpointRegistry};
use execution_contract::{
    ExecutionCommand, ExecutionError, ExecutionHandle, ExecutionHandlePort, ExecutionResult,
    Executor,
};
use std::sync::Arc;

pub struct FakeBackend {
    descriptor: DeviceDescriptor,
}

impl FakeBackend {
    pub fn new(config: &EndpointConfig) -> Self {
        Self {
            descriptor: DeviceDescriptor {
                id: config.device_id.clone(),
                name: "fake endpoint".into(),
                capabilities: vec!["execution".into()],
                primitives: vec!["hold".into(), "stop".into(), "execute_primitive".into()],
                sensors: vec![],
            },
        }
    }
}

struct Handle;
impl ExecutionHandlePort for Handle {
    fn result(&self) -> Option<ExecutionResult> {
        None
    }
}

impl Executor for FakeBackend {
    fn descriptor(&self) -> DeviceDescriptor {
        self.descriptor.clone()
    }
    fn execute(&self, _command: ExecutionCommand) -> Result<ExecutionHandle, ExecutionError> {
        Ok(Arc::new(Handle))
    }
    fn cancel(&self, _task_id: &TaskId) -> Result<(), ExecutionError> {
        Ok(())
    }
    fn state(&self) -> DeviceState {
        DeviceState {
            device_id: self.descriptor.id.clone(),
            healthy: true,
            message: "ready".into(),
            updated_at_ms: 0,
        }
    }
}

impl BackendSdk for FakeBackend {
    fn stop(&self) -> Result<(), String> {
        Ok(())
    }
}

pub fn registry() -> EndpointRegistry {
    let mut registry = EndpointRegistry::default();
    registry.register(
        "fake",
        Box::new(|config| Arc::new(FakeBackend::new(config))),
    );
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain_contract::DeviceId;

    #[test]
    fn endpoint_is_selected_by_registered_configuration() {
        let registry = registry();
        let endpoint = registry
            .create(&EndpointConfig {
                device_type: "fake".into(),
                device_id: DeviceId("one".into()),
            })
            .unwrap();
        assert_eq!(endpoint.descriptor().id, DeviceId("one".into()));
        assert!(registry
            .create(&EndpointConfig {
                device_type: "unknown".into(),
                device_id: DeviceId("two".into())
            })
            .is_err());
    }
}
