# Task Contract

平台拥有 canonical `Task`，仅包含：

- `id: TaskId`；
- `device_id: DeviceId`；
- `primitive: Primitive`；
- `target: Vec<i32>`；
- 可选 `deadline_ms`。

`Primitive` 仅包含 `go_to_tag`。`target` 是按执行顺序排列的 AprilTag ID 路线；
执行端必须保持该顺序。

平台同时拥有完整的 `TaskRecord`：`task`、`state`、`progress` 和 `phase`。
Gateway 与 Orchestration 通过共享的 `TaskRepository` 创建、读取和更新记录；
Gateway 不拥有任务存储或任务记录投影。Repository 负责生成任务 ID，并以原子操作
应用反馈和终态结果。终态记录仍可读取，但不再占用其设备。
