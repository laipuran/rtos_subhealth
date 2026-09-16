# 分层边界

| 层 | 职责 | 禁止依赖 |
|---|---|---|
| Gateway | HTTP/WS、持久化、外部事件 | 设备 SDK、设备类型 |
| Orchestration | 能力匹配、编排、生命周期 | ROS、设备实现 |
| Execution | 执行状态、取消、deadline、安全停止 | 具体机器 |
| Sensor | 采集、缓存、订阅、查询 | 任务决策 |
| Endpoint | 组合运行时、配置和本地安全 | 上层业务策略 |
| Backend SDK | 厂商协议和硬件调用 | 控制平面 |

依赖方向只能向下：`transport/adapter → service → contract`。
