use platform::{DeviceId, Primitive, Task, TaskId, TaskState};
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize)]
pub struct CreateTask {
    pub device_id: DeviceId,
    pub primitive: Primitive,
    pub target: Vec<i32>,
    pub deadline_ms: Option<u64>,
}

impl CreateTask {
    pub fn into_task(self, id: TaskId) -> Task {
        Task {
            id,
            device_id: self.device_id,
            primitive: self.primitive,
            target: self.target,
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
