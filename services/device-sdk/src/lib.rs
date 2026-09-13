//! Device adapter SDK: the in-process contract every device adapter (or
//! simulation) implements, independent of the transport. The rclrs adapter
//! nodes translate ROS actions/services into [`Command`]s and drive a backend.

pub mod backend;
pub mod diff_drive;
pub mod mock;
pub mod tonypi;

pub use backend::{BackendStatus, Command, DeviceBackend, Pose2d, Velocity};
pub use diff_drive::DiffDriveSim;
pub use mock::MockDevice;
pub use tonypi::{ActionRunner, TonyPiBackend};

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
