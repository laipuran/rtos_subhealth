# 接口契约

Rust 契约位于 `contracts/`：

- `domain-contract`：设备、任务、传感器标识和状态；
- `task-contract`：canonical task、target、primitive、生命周期；
- `execution-contract`：command、feedback、result、Executor；
- `sensor-contract`：descriptor、sample、filter、SensorProvider；
- `event-contract`：系统事件和事件序号。

ROS、HTTP 和 WebSocket 只负责映射这些类型。生成的 ROS 类型不得进入 service
核心。
