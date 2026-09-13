//! Device capability model.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    WheeledDiff,
    WheeledOmni,
    Legged,
    Aerial,
    Arm,
    Mock,
    Other(String),
}

impl DeviceKind {
    pub fn parse(s: &str) -> Self {
        match s {
            "wheeled_diff" => Self::WheeledDiff,
            "wheeled_omni" => Self::WheeledOmni,
            "legged" => Self::Legged,
            "aerial" => Self::Aerial,
            "arm" => Self::Arm,
            "mock" => Self::Mock,
            other => Self::Other(other.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::WheeledDiff => "wheeled_diff",
            Self::WheeledOmni => "wheeled_omni",
            Self::Legged => "legged",
            Self::Aerial => "aerial",
            Self::Arm => "arm",
            Self::Mock => "mock",
            Self::Other(s) => s,
        }
    }
}

/// A device primitive the orchestrator can route.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Primitive {
    MoveToPose,
    SetVelocity,
    Hold,
    Stop,
    ExecutePrimitive,
}

impl Primitive {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "move_to_pose" => Some(Self::MoveToPose),
            "set_velocity" => Some(Self::SetVelocity),
            "hold" => Some(Self::Hold),
            "stop" => Some(Self::Stop),
            "execute_primitive" => Some(Self::ExecutePrimitive),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::MoveToPose => "move_to_pose",
            Self::SetVelocity => "set_velocity",
            Self::Hold => "hold",
            Self::Stop => "stop",
            Self::ExecutePrimitive => "execute_primitive",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceLimits {
    pub max_speed_mps: f32,
    pub max_yaw_rate_rps: f32,
    pub max_slope_rad: f32,
    pub position_tolerance_m: f32,
    pub yaw_tolerance_rad: f32,
}

impl Default for DeviceLimits {
    fn default() -> Self {
        Self {
            max_speed_mps: 0.0,
            max_yaw_rate_rps: 0.0,
            max_slope_rad: 0.0,
            position_tolerance_m: 0.05,
            yaw_tolerance_rad: 0.1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceDescriptor {
    pub device_id: String,
    pub name: String,
    pub kind: DeviceKind,
    pub capabilities: Vec<String>,
    pub primitives: Vec<String>,
    pub limits: DeviceLimits,
    pub frames: Vec<String>,
    pub sensors: Vec<String>,
}

impl DeviceDescriptor {
    pub fn supports_primitive(&self, primitive: Primitive) -> bool {
        self.primitives.iter().any(|p| p == primitive.as_str())
    }

    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities.iter().any(|c| c == capability)
    }
}
