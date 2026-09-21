//! Concrete ROS execution, owned by the runtime composition root.

use std::{future::Future, pin::Pin};

use orchestration::ExecutionPort;
use platform::{DeviceId, ExecutionError, ExecutionSession, Task};
use ros_task_client::{
    DeviceConfig, RosRuntimeConfig, RosTaskClient, RosTaskClientConfig, RosTaskRuntime,
};

pub struct Execution {
    client: RosTaskClient,
    runtime: RosTaskRuntime,
}

impl Execution {
    pub fn start(device_id: DeviceId, action_name: String) -> Result<Self, ExecutionError> {
        let (client, runtime) = RosTaskClient::start(RosTaskClientConfig {
            version: 1,
            ros: RosRuntimeConfig {
                node_name: "execution".into(),
                feedback_buffer: 64,
                server_wait_timeout_ms: 5_000,
            },
            devices: vec![DeviceConfig {
                id: device_id.0,
                action_name,
                enabled: true,
            }],
        })
        .map_err(|error| ExecutionError::Failed(error.to_string()))?;
        Ok(Self { client, runtime })
    }

    pub fn shutdown(&self) -> Result<(), ExecutionError> {
        self.runtime
            .shutdown()
            .map_err(|error| ExecutionError::Failed(error.to_string()))
    }
}

impl ExecutionPort for Execution {
    fn execute(
        &self,
        task: Task,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionSession, ExecutionError>> + Send + '_>> {
        Box::pin(self.client.execute(task))
    }
}
