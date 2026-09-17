# Domain Contract

当前 `services/platform/src/domain.rs` 定义的基础标识类型为：

- `TaskId(String)`；
- `DeviceId(String)`；
- `SensorId(String)`。

- `DeviceDescriptor { id: DeviceId, name: String, capabilities: Vec<String>,
  primitives: Vec<String>, sensors: Vec<SensorId> }`；
- `DeviceState { device_id: DeviceId, healthy: bool, message: String,
  updated_at_ms: u64 }`。

该契约不得包含具体设备名称、厂商 SDK、ROS 类型或设备操作系统信息。

[ROS 2 RFC mocks](../guide/ros-mocks.md) 使用的字段不是这些 canonical 字段的
直接表示；生产 transport 需要后续实现显式 domain/ROS mapper。
