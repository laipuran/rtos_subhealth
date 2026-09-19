# 系统契约

这里是当前系统契约的唯一文档入口。契约代码位于 `ros2_ws/src/services/platform`，但不再
以独立 `contracts/` workspace 暴露；所有 transport 和 service 都必须遵守本文档
记录的语义。

## 契约目录

- `domain.md`：设备、任务和传感器标识及状态。
- `task.md`：canonical task、目标和 primitive。
- `execution.md`：执行命令、反馈、结果和 Executor 边界。
- `sensor.md`：Sensor descriptor、sample、filter 和 provider。
- `event.md`：系统事件和序列号。

ROS、HTTP、WebSocket 和 endpoint 协议只能映射这些契约，不得重新定义业务语义。
