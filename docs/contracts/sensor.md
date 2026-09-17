# Sensor Contract

当前 `services/platform/src/sensor.rs` 定义：

- `SensorDescriptor { id: SensorId, kind: String, unit: Option<String> }`；
- `SensorSample { sensor_id: SensorId, value: serde_json::Value, timestamp_ms:
  u64 }`；
- `SensorFilter { ids: Vec<SensorId> }`；
- `type SensorStream = Pin<Box<dyn Stream<Item = SensorSample> + Send>>`。

Provider 签名为：

```rust
pub trait SensorProvider: Send + Sync {
    fn descriptors(&self) -> Vec<SensorDescriptor>;
    fn latest(&self, id: &SensorId) -> Option<SensorSample>;
    fn subscribe(&self, filter: SensorFilter) -> SensorStream;
}
```

Orchestration 和 Execution 都可以访问同一 Sensor provider。Sensor 只负责采集、
缓存、查询和发布，不负责任务决策或动作执行。

[RFC 009 sensor mock](../guide/ros-mocks.md) 的 `PhysioSample` 不是 canonical
`SensorSample`；生产 transport 仍需显式 domain/ROS mapper。
