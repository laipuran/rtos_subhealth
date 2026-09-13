//! A kinematic differential-drive simulator. Validates the `move_to_pose` /
//! `set_velocity` contract without hardware and is the reference for wheeled
//! adapters.

use orchestrator_core::{DeviceDescriptor, DeviceKind, DeviceLimits, Primitive};

use crate::backend::{normalize_angle, BackendStatus, Command, DeviceBackend, Pose2d, Velocity};

struct Goal {
    pose: Pose2d,
    position_tolerance_m: f64,
    yaw_tolerance_rad: f64,
    initial_distance: f64,
}

pub struct DiffDriveSim {
    descriptor: DeviceDescriptor,
    pose: Pose2d,
    velocity: Velocity,
    goal: Option<Goal>,
    active: bool,
    timer_remaining: f64,
    primitive_pending: bool,
}

impl DiffDriveSim {
    pub fn new(device_id: impl Into<String>) -> Self {
        Self::with_limits(device_id, 0.5, 1.5)
    }

    pub fn with_limits(device_id: impl Into<String>, max_speed: f32, max_yaw_rate: f32) -> Self {
        let descriptor = DeviceDescriptor {
            device_id: device_id.into(),
            name: "differential drive simulator".into(),
            kind: DeviceKind::WheeledDiff,
            capabilities: vec!["supports_velocity".into(), "supports_pose".into()],
            primitives: vec![
                Primitive::MoveToPose.as_str().into(),
                Primitive::SetVelocity.as_str().into(),
                Primitive::Hold.as_str().into(),
                Primitive::Stop.as_str().into(),
            ],
            limits: DeviceLimits {
                max_speed_mps: max_speed,
                max_yaw_rate_rps: max_yaw_rate,
                ..DeviceLimits::default()
            },
            frames: vec!["map".into(), "base_link".into()],
            sensors: vec!["odometry".into()],
        };
        Self {
            descriptor,
            pose: Pose2d::new(0.0, 0.0, 0.0),
            velocity: Velocity::ZERO,
            goal: None,
            active: false,
            timer_remaining: 0.0,
            primitive_pending: false,
        }
    }

    pub fn set_pose(&mut self, pose: Pose2d) {
        self.pose = pose;
    }
}

impl DeviceBackend for DiffDriveSim {
    fn descriptor(&self) -> &DeviceDescriptor {
        &self.descriptor
    }

    fn command(&mut self, command: Command) -> Result<(), String> {
        match command {
            Command::MoveToPose {
                target,
                position_tolerance_m,
                yaw_tolerance_rad,
            } => {
                if !self.descriptor.supports_primitive(Primitive::MoveToPose) {
                    return Err("move_to_pose not supported".into());
                }
                let dx = target.x - self.pose.x;
                let dy = target.y - self.pose.y;
                self.goal = Some(Goal {
                    pose: target,
                    position_tolerance_m,
                    yaw_tolerance_rad,
                    initial_distance: (dx * dx + dy * dy).sqrt(),
                });
                self.velocity = Velocity::ZERO;
                self.active = true;
            }
            Command::SetVelocity {
                velocity,
                duration_s,
            } => {
                if !self.descriptor.supports_primitive(Primitive::SetVelocity) {
                    return Err("set_velocity not supported".into());
                }
                self.velocity = velocity;
                self.timer_remaining = duration_s;
                self.goal = None;
                self.active = true;
            }
            Command::Hold { duration_s } => {
                self.velocity = Velocity::ZERO;
                self.timer_remaining = duration_s;
                self.goal = None;
                self.active = true;
            }
            Command::Stop => {
                self.velocity = Velocity::ZERO;
                self.goal = None;
                self.timer_remaining = 0.0;
                self.active = true;
            }
            Command::ExecutePrimitive { .. } => {
                if !self
                    .descriptor
                    .supports_primitive(Primitive::ExecutePrimitive)
                {
                    return Err("execute_primitive not supported".into());
                }
                self.primitive_pending = true;
                self.active = true;
            }
        }
        Ok(())
    }

    fn step(&mut self, dt_s: f64) -> BackendStatus {
        if !self.active {
            return BackendStatus::Succeeded;
        }

        if self.primitive_pending {
            self.primitive_pending = false;
            self.active = false;
            return BackendStatus::Succeeded;
        }

        if let Some(goal) = &self.goal {
            let max_v = self.descriptor.limits.max_speed_mps as f64;
            let max_w = self.descriptor.limits.max_yaw_rate_rps as f64;
            let ex = goal.pose.x - self.pose.x;
            let ey = goal.pose.y - self.pose.y;
            let dist = (ex * ex + ey * ey).sqrt();
            let desired_yaw = ey.atan2(ex);
            let yaw_err = normalize_angle(desired_yaw - self.pose.yaw);

            let vx = if yaw_err.abs() > 0.5 {
                0.0
            } else {
                (dist * 0.8).min(max_v)
            };
            let vyaw = (yaw_err * 2.0).clamp(-max_w, max_w);
            self.velocity = Velocity::new(vx, 0.0, vyaw);
            self.pose.x += vx * dt_s * self.pose.yaw.cos();
            self.pose.y += vx * dt_s * self.pose.yaw.sin();
            self.pose.yaw = normalize_angle(self.pose.yaw + vyaw * dt_s);

            let arrived =
                dist <= goal.position_tolerance_m && yaw_err.abs() <= goal.yaw_tolerance_rad;
            if arrived {
                self.goal = None;
                self.velocity = Velocity::ZERO;
                self.active = false;
                return BackendStatus::Succeeded;
            }
            let initial = goal.initial_distance;
            let progress = if initial > f64::EPSILON {
                ((initial - dist) / initial).clamp(0.0, 1.0) as f32
            } else {
                1.0
            };
            return BackendStatus::Running { progress };
        }

        // Velocity / hold with a finite duration.
        self.pose.x += self.velocity.vx * dt_s * self.pose.yaw.cos();
        self.pose.y += self.velocity.vx * dt_s * self.pose.yaw.sin();
        self.pose.yaw = normalize_angle(self.pose.yaw + self.velocity.vyaw * dt_s);
        if self.timer_remaining > 0.0 {
            self.timer_remaining -= dt_s;
            if self.timer_remaining <= 0.0 {
                self.timer_remaining = 0.0;
                self.velocity = Velocity::ZERO;
                self.active = false;
                return BackendStatus::Succeeded;
            }
            return BackendStatus::Running { progress: 0.0 };
        }

        // No duration: velocity persists until replaced or stopped.
        BackendStatus::Running { progress: 0.0 }
    }

    fn pose(&self) -> Option<Pose2d> {
        Some(self.pose)
    }

    fn velocity(&self) -> Option<Velocity> {
        Some(self.velocity)
    }
}
