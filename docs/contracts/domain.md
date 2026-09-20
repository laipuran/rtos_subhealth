# Domain Contract

当前 `ros2_ws/src/services/platform/src/domain.rs` 定义的基础标识类型为：

- `TaskId(String)`；
- `DeviceId(String)`；
- `SensorId(String)`。

- `DeviceDescriptor { id: DeviceId, name: String, sensors: Vec<SensorId> }`；
- `DeviceState { device_id: DeviceId, healthy: bool, message: String,
  updated_at_ms: u64 }`。

该契约不得包含具体设备名称、厂商 SDK、ROS 类型或设备操作系统信息。
