use platform::SensorId;
use platform::{SensorDescriptor, SensorFilter, SensorProvider, SensorSample, SensorStream};
use std::sync::Arc;

#[derive(Default)]
pub struct SensorRegistry {
    providers: Vec<Arc<dyn SensorProvider>>,
}

impl SensorRegistry {
    pub fn register(&mut self, provider: Arc<dyn SensorProvider>) {
        self.providers.push(provider);
    }

    pub fn descriptors(&self) -> Vec<SensorDescriptor> {
        self.providers
            .iter()
            .flat_map(|provider| provider.descriptors())
            .collect()
    }

    pub fn latest(&self, id: &SensorId) -> Option<SensorSample> {
        self.providers
            .iter()
            .find_map(|provider| provider.latest(id))
    }

    pub fn subscribe(&self, filter: SensorFilter) -> Option<SensorStream> {
        self.providers
            .iter()
            .find(|provider| {
                provider
                    .descriptors()
                    .iter()
                    .any(|descriptor| filter.ids.contains(&descriptor.id))
            })
            .map(|provider| provider.subscribe(filter))
    }
}
