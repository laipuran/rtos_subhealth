//! TonyPi (Hiwonder) backend.
//!
//! TonyPi is not a ROS robot: it exposes a Python SDK and a JSON-RPC server on
//! `:9030` with discrete action groups and raw servo control. It has no velocity,
//! no odometry and no `cmd_vel`, so only `execute_primitive` (action groups),
//! `hold` (stand) and `stop` are supported.
//!
//! The HTTP transport is injected via [`ActionRunner`] so this logic is testable
//! without hardware.

use orchestrator_core::{DeviceDescriptor, DeviceKind, DeviceLimits, Primitive};

use crate::backend::{BackendStatus, Command, DeviceBackend, Pose2d, Velocity};

/// Sends a named action group to the robot. `"0"` stops.
pub trait ActionRunner: Send {
    fn run_action(&mut self, action: &str) -> Result<(), String>;
}

pub struct TonyPiBackend<R: ActionRunner> {
    descriptor: DeviceDescriptor,
    runner: R,
    pending: Option<String>,
    active: bool,
}

impl<R: ActionRunner> TonyPiBackend<R> {
    pub fn new(device_id: impl Into<String>, runner: R) -> Self {
        let device_id = device_id.into();
        let descriptor = DeviceDescriptor {
            device_id,
            name: "TonyPi humanoid".into(),
            kind: DeviceKind::Legged,
            capabilities: vec!["supports_action_groups".into()],
            primitives: vec![
                Primitive::ExecutePrimitive.as_str().into(),
                Primitive::Hold.as_str().into(),
                Primitive::Stop.as_str().into(),
            ],
            limits: DeviceLimits::default(),
            frames: vec![],
            sensors: vec!["imu".into()],
        };
        Self {
            descriptor,
            runner,
            pending: None,
            active: false,
        }
    }
}

impl<R: ActionRunner> DeviceBackend for TonyPiBackend<R> {
    fn descriptor(&self) -> &DeviceDescriptor {
        &self.descriptor
    }

    fn command(&mut self, command: Command) -> Result<(), String> {
        match command {
            Command::ExecutePrimitive { name, .. } => {
                self.pending = Some(name);
                self.active = true;
                Ok(())
            }
            Command::Hold { .. } => {
                self.pending = Some("stand".into());
                self.active = true;
                Ok(())
            }
            Command::Stop => {
                self.pending = Some("0".into());
                self.active = true;
                Ok(())
            }
            other => Err(format!(
                "{} not supported by TonyPi",
                other.primitive_name()
            )),
        }
    }

    fn step(&mut self, _dt_s: f64) -> BackendStatus {
        if !self.active {
            return BackendStatus::Succeeded;
        }
        self.active = false;
        match self.pending.take() {
            Some(action) => match self.runner.run_action(&action) {
                Ok(()) => BackendStatus::Succeeded,
                Err(err) => BackendStatus::failed("INTERNAL", err),
            },
            None => BackendStatus::Succeeded,
        }
    }

    fn pose(&self) -> Option<Pose2d> {
        None
    }

    fn velocity(&self) -> Option<Velocity> {
        None
    }
}
