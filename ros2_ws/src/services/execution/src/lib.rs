//! Concrete ROS execution, owned by the runtime composition root.

use std::{future::Future, pin::Pin};

use orchestration::ExecutionPort;
use platform::{ExecutionError, ExecutionSession, Task};
use ros_task_client::{RosTaskClient, RosTaskRuntime};

pub struct Execution {
    client: RosTaskClient,
    runtime: RosTaskRuntime,
}

impl Execution {
    pub fn init() -> Result<Self, ExecutionError> {
        let (client, runtime) =
            RosTaskClient::init().map_err(|error| ExecutionError::Failed(error.to_string()))?;
        Ok(Self { client, runtime })
    }

    pub fn shutdown(&self) -> Result<(), ExecutionError> {
        self.runtime
            .shutdown()
            .map_err(|error| ExecutionError::Failed(error.to_string()))
    }
}

impl ExecutionPort for Execution {
    fn validate<'a>(
        &'a self,
        task: &'a Task,
    ) -> Pin<Box<dyn Future<Output = Result<(), ExecutionError>> + Send + 'a>> {
        Box::pin(async move {
            self.client
                .validate(task)
                .await
                .map_err(ExecutionError::from)
        })
    }

    fn execute(
        &self,
        task: Task,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionSession, ExecutionError>> + Send + '_>> {
        Box::pin(self.client.execute(task))
    }
}
