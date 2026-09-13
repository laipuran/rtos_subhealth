//! Outbound bridge to the ROS graph.
//!
//! The gateway never calls rclrs directly from the HTTP layer. Instead it emits
//! [`BridgeCommand`]s through a [`RosBridge`], which keeps the whole HTTP/WS
//! layer testable without a ROS installation. The production implementation
//! (feature `ros`) wraps an `rclrs` action client; [`mock::MockBridge`] records
//! commands for tests.

pub mod mock;

use crate::model::task::Goal;

#[derive(Debug, Clone, PartialEq)]
pub enum BridgeCommand {
    SendGoal { goal_id: String, goal: Goal },
    Cancel { goal_id: String },
    TriggerDiagnosis { diagnosis_id: String },
}

pub trait RosBridge: Send + Sync {
    fn send_goal(&self, goal_id: &str, goal: Goal);
    fn cancel(&self, goal_id: &str);
    fn trigger_diagnosis(&self, diagnosis_id: &str);
}
