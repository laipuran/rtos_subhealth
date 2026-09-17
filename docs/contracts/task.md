# Task Contract

Canonical `Task` 包含：

- `id`；
- 可选 `device_id`；
- `required_capabilities`；
- `primitive`；
- `target`；
- `parameters`；
- 可选 `deadline_ms`。

通用 primitive 为 `hold`、`stop`、`execute_primitive`、`move_to_pose` 和
`set_velocity`。设备是否支持某个 primitive 由 `DeviceDescriptor` 声明，不能由
Orchestration 按设备名称判断。

[ROS 2 RFC mock](../guide/ros-mocks.md) 的 `ExecTask` 保留 RFC workflow 字段，
并不是 canonical `Task` 的 ROS 表示；生产 transport 需要后续实现显式 mapper。
