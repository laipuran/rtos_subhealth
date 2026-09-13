//! Device-agnostic safety supervisor.
//!
//! Pure logic: clamps velocity commands to the device's advertised limits,
//! enforces an emergency stop, and provides a watchdog for stale device state.
//! Kept free of ROS so it can be unit-tested off-robot and reused by any node.

pub mod supervisor;

pub use supervisor::{SafetyLimits, SafetySupervisor, SafetyViolation};

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
