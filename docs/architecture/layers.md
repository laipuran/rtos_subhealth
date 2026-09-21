# 分层边界

| 层 | 职责 | 禁止依赖 |
|---|---|---|
| Gateway | HTTP/WS、Repository 查询、瞬时事件通知 | 任务状态存储、设备 SDK、设备类型 |
| Orchestration | 能力匹配、编排、生命周期 | ROS、设备实现 |
| Execution | 执行状态、取消、deadline、安全停止 | 具体机器 |
| Repository | 任务状态真相源、任务记录读写和状态转换 | 外部传输、执行实现 |
| Sensor | 采集、缓存、订阅、查询 | 任务决策 |
| Endpoint | 组合运行时、配置和本地安全 | 上层业务策略 |
| Backend SDK | 厂商协议和硬件调用 | 控制平面 |

依赖方向只能向下：`transport/adapter → service → contract`。

Repository 是已接受任务及其状态、进度和阶段的唯一真相源。Gateway 和
Orchestration 共享同一个 Repository 实例；Gateway 不拥有任务状态，只查询
Repository 并在状态转换后发布瞬时事件通知。事件不是任务状态存储，也不是
Repository 的替代品。
