# Event Contract

系统事件使用单调 `EventSequence`，事件类型包括：

- 任务状态变化；
- 设备状态变化；
- Sensor 更新。

WebSocket 客户端按序号恢复状态；事件契约不包含具体设备实现细节。
