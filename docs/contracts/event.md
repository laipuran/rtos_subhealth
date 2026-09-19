# Event Contract

当前 `ros2_ws/src/services/platform/src/event.rs` 使用 `EventSequence(u64)`。`SystemEvent`
payload 精确为：

- `TaskStateChanged { task_id: TaskId, state: TaskState }`；
- `DeviceStateChanged { device_id: DeviceId, healthy: bool }`；
- `SensorUpdated { sensor_id: String }`。

WebSocket 客户端按序号恢复状态；事件契约不包含具体设备实现细节。
[ROS 2 RFC mocks](../guide/ros-mocks.md) 不发布 canonical `SystemEvent`，后续
mapper 必须显式定义 ROS 数据如何产生这些事件。
