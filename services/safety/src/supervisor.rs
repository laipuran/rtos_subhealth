//! Safety supervisor implementation.

use device_sdk::{Command, Velocity};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SafetyLimits {
    pub max_speed_mps: f64,
    pub max_yaw_rate_rps: f64,
}

impl Default for SafetyLimits {
    fn default() -> Self {
        Self {
            max_speed_mps: 2.0,
            max_yaw_rate_rps: 3.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum SafetyViolation {
    #[error("emergency stop engaged")]
    EStop,
    #[error("device state is stale by {0:.3}s")]
    Stale(f64),
}

pub struct SafetySupervisor {
    limits: SafetyLimits,
    estop: bool,
    last_heartbeat: Option<f64>,
    watchdog_timeout_s: f64,
}

impl SafetySupervisor {
    pub fn new(limits: SafetyLimits, watchdog_timeout_s: f64) -> Self {
        Self {
            limits,
            estop: false,
            last_heartbeat: None,
            watchdog_timeout_s,
        }
    }

    pub fn limits(&self) -> SafetyLimits {
        self.limits
    }

    pub fn engage_estop(&mut self) {
        self.estop = true;
    }

    pub fn release_estop(&mut self) {
        self.estop = false;
    }

    pub fn is_estopped(&self) -> bool {
        self.estop
    }

    pub fn heartbeat(&mut self, now_s: f64) {
        self.last_heartbeat = Some(now_s);
    }

    pub fn is_stale(&self, now_s: f64) -> bool {
        match (self.last_heartbeat, self.watchdog_timeout_s) {
            (Some(last), timeout) if timeout > 0.0 => now_s - last > timeout,
            _ => false,
        }
    }

    /// Scale a linear velocity vector so its magnitude does not exceed the
    /// limit, and clamp yaw rate independently.
    pub fn clamp_velocity(&self, v: Velocity) -> Velocity {
        let magnitude = (v.vx * v.vx + v.vy * v.vy).sqrt();
        let (vx, vy) = if magnitude > self.limits.max_speed_mps && magnitude > f64::EPSILON {
            let scale = self.limits.max_speed_mps / magnitude;
            (v.vx * scale, v.vy * scale)
        } else {
            (v.vx, v.vy)
        };
        Velocity::new(
            vx,
            vy,
            v.vyaw
                .clamp(-self.limits.max_yaw_rate_rps, self.limits.max_yaw_rate_rps),
        )
    }

    /// Validate a command against the current safety state.
    pub fn check(&self, now_s: f64) -> Result<(), SafetyViolation> {
        if self.estop {
            return Err(SafetyViolation::EStop);
        }
        if self.is_stale(now_s) {
            return Err(SafetyViolation::Stale(
                now_s - self.last_heartbeat.unwrap_or(now_s),
            ));
        }
        Ok(())
    }

    /// Return a command that is safe to execute, clamping as needed.
    pub fn sanitize(&self, command: Command, now_s: f64) -> Result<Command, SafetyViolation> {
        self.check(now_s)?;
        Ok(match command {
            Command::SetVelocity {
                velocity,
                duration_s,
            } => Command::SetVelocity {
                velocity: self.clamp_velocity(velocity),
                duration_s,
            },
            other => other,
        })
    }
}
