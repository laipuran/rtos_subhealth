//! Simulated AprilTag perception.
//!
//! Geometry-based detector used in simulation and tests: projects known tag
//! positions into the camera frame, applies frustum culling and injects
//! configurable noise. It produces the same fields as the real perception node
//! (RFC-001), so downstream code does not need to know the difference.

pub mod detector;

pub use detector::{Camera, Detection, Pose2d, SimulatedAprilTagDetector, Tag};

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
