# Domain Contract

基础标识类型：`TaskId`、`DeviceId`、`SensorId`。

`DeviceDescriptor` 描述设备事实：标识、名称、能力、支持的 primitive 和可用
Sensor；`DeviceState` 描述健康状态、状态信息和更新时间。

该契约不得包含具体设备名称、厂商 SDK、ROS 类型或设备操作系统信息。
