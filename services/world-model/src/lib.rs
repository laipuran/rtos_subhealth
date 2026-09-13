//! Device-agnostic world model and path planner.
//!
//! A `WorldModel` is a graph of named nodes with positions, weighted directed
//! edges and predefined routes. Tags are one kind of node; waypoints and poses
//! are others. The planner is pure logic, testable without ROS.

pub mod model;
pub mod planner;

pub use model::{Edge, Node, WorldModel, WorldModelError};
pub use planner::{PlanError, Planner, Segment};

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
