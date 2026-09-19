use crate::domain::{DeviceId, TaskId};
use crate::task::TaskState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EventSequence(pub u64);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SystemEvent {
    TaskStateChanged { task_id: TaskId, state: TaskState },
    DeviceStateChanged { device_id: DeviceId, healthy: bool },
    SensorUpdated { sensor_id: String },
}
