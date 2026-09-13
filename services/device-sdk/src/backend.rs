//! Core adapter contract.

use orchestrator_core::DeviceDescriptor;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose2d {
    pub x: f64,
    pub y: f64,
    pub yaw: f64,
}

impl Pose2d {
    pub fn new(x: f64, y: f64, yaw: f64) -> Self {
        Self { x, y, yaw }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Velocity {
    pub vx: f64,
    pub vy: f64,
    pub vyaw: f64,
}

impl Velocity {
    pub const ZERO: Velocity = Velocity {
        vx: 0.0,
        vy: 0.0,
        vyaw: 0.0,
    };

    pub fn new(vx: f64, vy: f64, vyaw: f64) -> Self {
        Self { vx, vy, vyaw }
    }
}

/// A primitive invocation, mapped from the ROS device interface.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    MoveToPose {
        target: Pose2d,
        position_tolerance_m: f64,
        yaw_tolerance_rad: f64,
    },
    SetVelocity {
        velocity: Velocity,
        duration_s: f64,
    },
    Hold {
        duration_s: f64,
    },
    Stop,
    ExecutePrimitive {
        name: String,
        params: Value,
    },
}

impl Command {
    pub fn primitive_name(&self) -> &'static str {
        match self {
            Self::MoveToPose { .. } => "move_to_pose",
            Self::SetVelocity { .. } => "set_velocity",
            Self::Hold { .. } => "hold",
            Self::Stop => "stop",
            Self::ExecutePrimitive { .. } => "execute_primitive",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BackendStatus {
    Running { progress: f32 },
    Succeeded,
    Failed { code: String, message: String },
}

impl BackendStatus {
    pub fn is_terminal(&self) -> bool {
        !matches!(self, Self::Running { .. })
    }

    pub fn failed(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Failed {
            code: code.into(),
            message: message.into(),
        }
    }
}

/// A device adapter backend. Implementations may be simulated (this crate) or
/// real (an rclrs node driving vendor hardware).
pub trait DeviceBackend: Send {
    fn descriptor(&self) -> &DeviceDescriptor;

    /// Accept a command. Returns `Err` when the device cannot execute it, which
    /// maps to a rejected goal in the adapter node.
    fn command(&mut self, command: Command) -> Result<(), String>;

    /// Advance the backend by `dt_s`. Returns the current status.
    fn step(&mut self, dt_s: f64) -> BackendStatus;

    fn pose(&self) -> Option<Pose2d>;

    fn velocity(&self) -> Option<Velocity>;
}

/// Wrap an angle to `[-pi, pi]`.
pub(crate) fn normalize_angle(a: f64) -> f64 {
    let mut a = a;
    while a > std::f64::consts::PI {
        a -= 2.0 * std::f64::consts::PI;
    }
    while a < -std::f64::consts::PI {
        a += 2.0 * std::f64::consts::PI;
    }
    a
}
