use std::{collections::HashSet, time::Duration};

use crate::RosTaskError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RosConnectionConfig {
    pub node_name: String,
    pub endpoints: Vec<ExecEndpointConfig>,
    pub feedback_buffer: usize,
    pub server_wait_timeout: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecEndpointConfig {
    pub device_id: String,
    /// Canonical absolute action name. Endpoint remapping is unsupported with
    /// rclrs 0.7 because it does not expose the ActionClient's resolved name.
    pub action_name: String,
}

impl RosConnectionConfig {
    pub fn validate(&self) -> Result<(), RosTaskError> {
        validate_name("node_name", &self.node_name)?;
        if self.feedback_buffer == 0 {
            return Err(RosTaskError::InvalidConfig {
                field: "feedback_buffer",
                message: "must be greater than zero".into(),
            });
        }
        if self.server_wait_timeout.is_zero() {
            return Err(RosTaskError::InvalidConfig {
                field: "server_wait_timeout",
                message: "must be greater than zero".into(),
            });
        }

        let mut device_ids = HashSet::new();
        let mut action_names = HashSet::new();
        for endpoint in &self.endpoints {
            validate_name("device_id", &endpoint.device_id)?;
            validate_name("action_name", &endpoint.action_name)?;
            if !is_canonical_absolute_ros_name(&endpoint.action_name) {
                return Err(RosTaskError::InvalidConfig {
                    field: "action_name",
                    message: "must be a canonical absolute ROS action name".into(),
                });
            }
            if !device_ids.insert(endpoint.device_id.as_str()) {
                return Err(RosTaskError::InvalidConfig {
                    field: "device_id",
                    message: format!("duplicate device ID `{}`", endpoint.device_id),
                });
            }
            if !action_names.insert(endpoint.action_name.as_str()) {
                return Err(RosTaskError::InvalidConfig {
                    field: "action_name",
                    message: format!("duplicate action name `{}`", endpoint.action_name),
                });
            }
        }
        Ok(())
    }
}

fn is_canonical_absolute_ros_name(value: &str) -> bool {
    value.starts_with('/')
        && value != "/"
        && value
            .strip_prefix('/')
            .is_some_and(|name| name.split('/').all(is_valid_ros_name_token))
}

fn is_valid_ros_name_token(token: &str) -> bool {
    let mut characters = token.chars();
    matches!(
        characters.next(),
        Some(character) if character.is_ascii_alphabetic() || character == '_'
    ) && characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn validate_name(field: &'static str, value: &str) -> Result<(), RosTaskError> {
    if value.is_empty() {
        return Err(RosTaskError::InvalidConfig {
            field,
            message: "must not be empty".into(),
        });
    }
    if value.trim() != value {
        return Err(RosTaskError::InvalidConfig {
            field,
            message: "must not have leading or trailing whitespace".into(),
        });
    }
    Ok(())
}
