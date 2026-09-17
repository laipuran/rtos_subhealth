use crate::domain::SensorId;
use futures_core::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SensorDescriptor {
    pub id: SensorId,
    pub kind: String,
    pub unit: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SensorSample {
    pub sensor_id: SensorId,
    pub value: serde_json::Value,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SensorFilter {
    pub ids: Vec<SensorId>,
}

pub type SensorStream = Pin<Box<dyn Stream<Item = SensorSample> + Send>>;

pub trait SensorProvider: Send + Sync {
    fn descriptors(&self) -> Vec<SensorDescriptor>;
    fn latest(&self, id: &SensorId) -> Option<SensorSample>;
    fn subscribe(&self, filter: SensorFilter) -> SensorStream;
}
