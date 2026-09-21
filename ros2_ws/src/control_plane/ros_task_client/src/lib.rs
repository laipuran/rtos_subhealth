//! Typed Rust client for the ExecuteTask ROS action.

mod client;
mod config;
mod error;
mod mapper;
mod runtime;

pub use client::RosTaskClient;
pub use config::{DeviceConfig, RosRuntimeConfig, RosTaskClientConfig};
pub use error::RosTaskError;
pub use runtime::RosTaskRuntime;
