//! Task model. Intentionally single-step: a future workflow engine can build on
//! the same [`Task`]/[`TaskTarget`] types without changing them.

use crate::device::Primitive;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetKind {
    Tag,
    Pose,
    Waypoint,
    Action,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskTarget {
    pub kind: TargetKind,
    pub tag_id: Option<i32>,
    pub waypoint_id: Option<String>,
    pub action_id: Option<String>,
    pub position_tolerance_m: Option<f32>,
    pub yaw_tolerance_rad: Option<f32>,
}

impl TaskTarget {
    pub fn tag(tag_id: i32) -> Self {
        Self {
            kind: TargetKind::Tag,
            tag_id: Some(tag_id),
            waypoint_id: None,
            action_id: None,
            position_tolerance_m: None,
            yaw_tolerance_rad: None,
        }
    }

    pub fn waypoint(id: impl Into<String>) -> Self {
        Self {
            kind: TargetKind::Waypoint,
            tag_id: None,
            waypoint_id: Some(id.into()),
            action_id: None,
            position_tolerance_m: None,
            yaw_tolerance_rad: None,
        }
    }

    pub fn action(id: impl Into<String>) -> Self {
        Self {
            kind: TargetKind::Action,
            tag_id: None,
            waypoint_id: None,
            action_id: Some(id.into()),
            position_tolerance_m: None,
            yaw_tolerance_rad: None,
        }
    }

    pub fn pose(tolerance_m: f32, yaw_tolerance_rad: f32) -> Self {
        Self {
            kind: TargetKind::Pose,
            tag_id: None,
            waypoint_id: None,
            action_id: None,
            position_tolerance_m: Some(tolerance_m),
            yaw_tolerance_rad: Some(yaw_tolerance_rad),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Task {
    pub goal_id: String,
    pub device_id: String,
    pub primitive: Primitive,
    /// Optional: primitives like `set_velocity`, `hold` and `stop` need no target.
    pub target: Option<TaskTarget>,
    /// Primitive-specific extras (mirrors the ROS `params_json` field).
    pub params_json: Option<String>,
    pub deadline_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Dispatched,
    Running,
    Succeeded,
    Failed,
    Canceled,
}

impl TaskState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Canceled)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskOutcome {
    pub state: TaskState,
    pub error_code: String,
    pub message: String,
}

impl TaskOutcome {
    pub fn succeeded() -> Self {
        Self {
            state: TaskState::Succeeded,
            error_code: String::new(),
            message: String::new(),
        }
    }

    pub fn failed(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            state: TaskState::Failed,
            error_code: code.into(),
            message: message.into(),
        }
    }

    pub fn canceled() -> Self {
        Self {
            state: TaskState::Canceled,
            error_code: String::new(),
            message: String::new(),
        }
    }
}
