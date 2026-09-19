use platform::TaskState;
use platform::{Primitive, Task, TaskId, TaskTarget};
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize)]
pub struct CreateTask {
    pub device_id: Option<platform::DeviceId>,

    #[serde(default)]
    pub required_capabilities: Vec<String>,

    pub primitive: Primitive,

    #[serde(default)]
    pub target: TaskTarget,

    #[serde(default)]
    pub parameters: serde_json::Value,

    pub deadline_ms: Option<u64>,
}

impl CreateTask {
    pub fn into_task(self, id: TaskId) -> Task {
        Task {
            id,
            device_id: self.device_id,
            required_capabilities: self.required_capabilities,
            primitive: self.primitive,
            target: self.target,
            parameters: self.parameters,
            deadline_ms: self.deadline_ms,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskView {
    pub task: Task,
    pub state: TaskState,
    pub progress: f32,
    pub phase: String,
}
