//! A scripted mock device for tests and WebUI demos.

use orchestrator_core::{DeviceDescriptor, DeviceKind, DeviceLimits, Primitive};

use crate::backend::{BackendStatus, Command, DeviceBackend, Pose2d, Velocity};

pub struct MockDevice {
    descriptor: DeviceDescriptor,
    steps_to_terminal: u32,
    steps_remaining: u32,
    fail: bool,
    active: bool,
    pose: Pose2d,
}

impl MockDevice {
    pub fn new(device_id: impl Into<String>, primitives: &[Primitive], steps: u32) -> Self {
        let descriptor = DeviceDescriptor {
            device_id: device_id.into(),
            name: "mock device".into(),
            kind: DeviceKind::Mock,
            capabilities: vec!["supports_action_groups".into()],
            primitives: primitives.iter().map(|p| p.as_str().to_string()).collect(),
            limits: DeviceLimits::default(),
            frames: vec!["map".into()],
            sensors: vec![],
        };
        Self {
            descriptor,
            steps_to_terminal: steps.max(1),
            steps_remaining: 0,
            fail: false,
            active: false,
            pose: Pose2d::new(0.0, 0.0, 0.0),
        }
    }

    pub fn failing(mut self) -> Self {
        self.fail = true;
        self
    }
}

impl DeviceBackend for MockDevice {
    fn descriptor(&self) -> &DeviceDescriptor {
        &self.descriptor
    }

    fn command(&mut self, command: Command) -> Result<(), String> {
        let Some(primitive) = Primitive::parse(command.primitive_name()) else {
            return Err(format!("unknown primitive: {}", command.primitive_name()));
        };
        if !self.descriptor.supports_primitive(primitive) {
            return Err(format!("{} not supported", command.primitive_name()));
        }
        self.steps_remaining = self.steps_to_terminal;
        self.active = true;
        Ok(())
    }

    fn step(&mut self, _dt_s: f64) -> BackendStatus {
        if !self.active {
            return BackendStatus::Succeeded;
        }
        self.steps_remaining -= 1;
        if self.steps_remaining > 0 {
            let total = self.steps_to_terminal as f32;
            let done = (total - self.steps_remaining as f32) / total;
            return BackendStatus::Running { progress: done };
        }
        self.active = false;
        if self.fail {
            BackendStatus::failed("INTERNAL", "mock device configured to fail")
        } else {
            BackendStatus::Succeeded
        }
    }

    fn pose(&self) -> Option<Pose2d> {
        Some(self.pose)
    }

    fn velocity(&self) -> Option<Velocity> {
        Some(Velocity::ZERO)
    }
}
