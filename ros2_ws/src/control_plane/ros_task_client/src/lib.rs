//! Typed Rust client for the ExecuteTask ROS action.

mod client;
mod config;
mod error;
mod mapper;
mod runtime;
mod types;

pub use client::RosTaskClient;
pub use config::{ExecEndpointConfig, RosConnectionConfig};
pub use error::RosTaskError;
pub use runtime::RosTaskRuntime;
pub use types::{
    ExecuteCommand, FinalState, PrimitiveCommand, TaskFeedback, TaskResult, TaskSession,
};
