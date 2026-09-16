use domain_contract::{DeviceId, TaskId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Primitive {
    Hold,
    Stop,
    ExecutePrimitive,
    MoveToPose,
    SetVelocity,
}

impl Primitive {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Hold => "hold",
            Self::Stop => "stop",
            Self::ExecutePrimitive => "execute_primitive",
            Self::MoveToPose => "move_to_pose",
            Self::SetVelocity => "set_velocity",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TaskTarget {
    None,
    Pose { x: f64, y: f64, yaw: f64 },
    Sensor { sensor_id: String },
    Named { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    Accepted,
    Running,
    Succeeded,
    Canceled,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub device_id: Option<DeviceId>,
    pub required_capabilities: Vec<String>,
    pub primitive: Primitive,
    pub target: TaskTarget,
    pub parameters: serde_json::Value,
    pub deadline_ms: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitive_names_are_transport_stable() {
        assert_eq!(Primitive::ExecutePrimitive.as_str(), "execute_primitive");
    }
}
