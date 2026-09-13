//! In-memory device registry.

use std::collections::HashMap;

use crate::device::{DeviceDescriptor, Primitive};

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum RegistryError {
    #[error("device already registered: {0}")]
    Duplicate(String),
    #[error("device not found: {0}")]
    NotFound(String),
}

#[derive(Debug, Default)]
pub struct DeviceRegistry {
    devices: HashMap<String, DeviceDescriptor>,
}

impl DeviceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, descriptor: DeviceDescriptor) -> Result<(), RegistryError> {
        if self.devices.contains_key(&descriptor.device_id) {
            return Err(RegistryError::Duplicate(descriptor.device_id));
        }
        self.devices
            .insert(descriptor.device_id.clone(), descriptor);
        Ok(())
    }

    pub fn unregister(&mut self, device_id: &str) -> Result<DeviceDescriptor, RegistryError> {
        self.devices
            .remove(device_id)
            .ok_or_else(|| RegistryError::NotFound(device_id.to_string()))
    }

    pub fn get(&self, device_id: &str) -> Option<&DeviceDescriptor> {
        self.devices.get(device_id)
    }

    pub fn list(&self) -> Vec<&DeviceDescriptor> {
        let mut all: Vec<&DeviceDescriptor> = self.devices.values().collect();
        all.sort_by(|a, b| a.device_id.cmp(&b.device_id));
        all
    }

    pub fn len(&self) -> usize {
        self.devices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.devices.is_empty()
    }

    /// Devices that support every required primitive and, if given, a capability.
    pub fn find(&self, required: &[Primitive], capability: Option<&str>) -> Vec<&DeviceDescriptor> {
        let mut found: Vec<&DeviceDescriptor> = self
            .devices
            .values()
            .filter(|d| required.iter().all(|p| d.supports_primitive(*p)))
            .filter(|d| capability.is_none_or(|c| d.has_capability(c)))
            .collect();
        found.sort_by(|a, b| a.device_id.cmp(&b.device_id));
        found
    }
}
