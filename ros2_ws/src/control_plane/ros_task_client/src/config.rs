use std::{collections::HashSet, env, fs, path::Path};

use crate::RosTaskError;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RosTaskClientConfig {
    pub version: u32,
    pub ros: RosRuntimeConfig,
    pub devices: Vec<DeviceConfig>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RosRuntimeConfig {
    pub node_name: String,
    pub feedback_buffer: usize,
    pub server_wait_timeout_ms: u64,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeviceConfig {
    pub id: String,
    pub action_name: String,
    pub enabled: bool,
}

impl RosTaskClientConfig {
    pub fn from_environment() -> Result<Self, RosTaskError> {
        let path =
            env::var_os("ROS_TASK_CLIENT_CONFIG").ok_or_else(|| RosTaskError::ConfigLoad {
                message: "ROS_TASK_CLIENT_CONFIG is not set".into(),
            })?;
        Self::from_path(Path::new(&path))
    }

    pub fn from_path(path: &Path) -> Result<Self, RosTaskError> {
        let contents = fs::read_to_string(path).map_err(|error| RosTaskError::ConfigLoad {
            message: format!("could not read {}: {error}", path.display()),
        })?;
        let config = serde_yaml::from_str(&contents).map_err(|error| RosTaskError::ConfigLoad {
            message: format!("could not parse {}: {error}", path.display()),
        })?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), RosTaskError> {
        if self.version != 1 {
            return Err(RosTaskError::InvalidConfig {
                field: "version",
                message: "must be 1".into(),
            });
        }
        validate_name("node_name", &self.ros.node_name)?;
        if self.ros.feedback_buffer == 0 {
            return Err(RosTaskError::InvalidConfig {
                field: "feedback_buffer",
                message: "must be greater than zero".into(),
            });
        }
        if self.ros.server_wait_timeout_ms == 0 {
            return Err(RosTaskError::InvalidConfig {
                field: "server_wait_timeout_ms",
                message: "must be greater than zero".into(),
            });
        }

        let mut device_ids = HashSet::new();
        let mut action_names = HashSet::new();
        for device in &self.devices {
            validate_name("device_id", &device.id)?;
            validate_name("action_name", &device.action_name)?;
            if !is_canonical_absolute_ros_name(&device.action_name) {
                return Err(RosTaskError::InvalidConfig {
                    field: "action_name",
                    message: "must be a canonical absolute ROS action name".into(),
                });
            }
            if !device_ids.insert(device.id.as_str()) {
                return Err(RosTaskError::InvalidConfig {
                    field: "device_id",
                    message: format!("duplicate device ID `{}`", device.id),
                });
            }
            if !action_names.insert(device.action_name.as_str()) {
                return Err(RosTaskError::InvalidConfig {
                    field: "action_name",
                    message: format!("duplicate action name `{}`", device.action_name),
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
