//! Pure-logic orchestration core.
//!
//! Device-agnostic task routing, capability matching and lifecycle tracking.
//! Contains no ROS dependency so the decision logic is unit-tested off-robot.
//! The rclrs node in `ros2_ws` wraps this crate and translates [`DispatchPlan`]
//! values into ROS action/service calls.

pub mod device;
pub mod orchestrator;
pub mod registry;
pub mod task;

pub use device::{DeviceDescriptor, DeviceKind, DeviceLimits, Primitive};
pub use orchestrator::{ActiveTask, DispatchPlan, Orchestrator, RejectReason};
pub use registry::{DeviceRegistry, RegistryError};
pub use task::{TargetKind, Task, TaskOutcome, TaskState, TaskTarget};

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
