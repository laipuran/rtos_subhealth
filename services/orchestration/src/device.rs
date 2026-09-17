use platform::Task;
use platform::{DeviceDescriptor, DeviceId};
use std::collections::HashMap;

#[derive(Default)]
pub struct DeviceRegistry {
    devices: HashMap<DeviceId, DeviceDescriptor>,
}

impl DeviceRegistry {
    pub fn register(&mut self, descriptor: DeviceDescriptor) {
        self.devices.insert(descriptor.id.clone(), descriptor);
    }

    pub fn get(&self, id: &DeviceId) -> Option<&DeviceDescriptor> {
        self.devices.get(id)
    }

    pub fn select(&self, task: &Task) -> Option<DeviceId> {
        if let Some(id) = &task.device_id {
            return self
                .devices
                .get(id)
                .and_then(|device| supports(device, task).then(|| device.id.clone()));
        }
        self.devices
            .values()
            .find(|device| supports(device, task))
            .map(|device| device.id.clone())
    }
}

fn supports(device: &DeviceDescriptor, task: &Task) -> bool {
    task.required_capabilities
        .iter()
        .all(|required| device.capabilities.contains(required))
        && device
            .primitives
            .iter()
            .any(|primitive| primitive == task.primitive.as_str())
}
