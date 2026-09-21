use platform::{DeviceId, Primitive, Task, TaskId};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CreateTask {
    pub device_id: DeviceId,
    pub primitive: Primitive,
    pub target: Vec<i32>,
    pub deadline_ms: Option<u64>,
}

impl CreateTask {
    pub fn into_task(self) -> Task {
        Task {
            id: TaskId(String::new()),
            device_id: self.device_id,
            primitive: self.primitive,
            target: self.target,
            deadline_ms: self.deadline_ms,
        }
    }
}
