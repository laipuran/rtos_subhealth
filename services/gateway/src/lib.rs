//! ROS Subhealth gateway.
//!
//! HTTP/WS gateway that replaces the legacy Python `desc_layer`. It preserves
//! the RFC-005/006/009 external contract and talks to the ROS graph through a
//! [`bridge::RosBridge`] seam so that the bulk of the logic is testable without
//! a ROS installation.

pub mod api;
pub mod app;
pub mod bridge;
pub mod config;
pub mod error;
pub mod model;
pub mod store;

pub use config::Config;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
