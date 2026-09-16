use domain_contract::SensorId;
use sensor_contract::{SensorDescriptor, SensorFilter, SensorProvider, SensorSample, SensorStream};
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

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::stream;
    use serde_json::json;

    struct Provider;

    impl SensorProvider for Provider {
        fn descriptors(&self) -> Vec<SensorDescriptor> {
            vec![SensorDescriptor {
                id: SensorId("battery".into()),
                kind: "battery".into(),
                unit: Some("percent".into()),
            }]
        }

        fn latest(&self, id: &SensorId) -> Option<SensorSample> {
            (id.0 == "battery").then(|| SensorSample {
                sensor_id: id.clone(),
                value: json!(90),
                timestamp_ms: 1,
            })
        }

        fn subscribe(&self, _filter: SensorFilter) -> SensorStream {
            Box::pin(stream::empty())
        }
    }

    #[test]
    fn registry_exposes_one_provider_to_all_consumers() {
        let mut registry = SensorRegistry::default();
        registry.register(Arc::new(Provider));
        assert_eq!(registry.descriptors().len(), 1);
        assert_eq!(
            registry.latest(&SensorId("battery".into())).unwrap().value,
            json!(90)
        );
    }
}
