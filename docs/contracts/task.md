# Task Contract

Canonical `Task` 包含：

- `id: TaskId`；
- `device_id: DeviceId`；
- `primitive: Primitive`；
- `target: Vec<i32>`；
- 可选 `deadline_ms`。

`Primitive` 仅包含 `go_to_tag`。`target` 是按执行顺序排列的 AprilTag ID 路线；
执行端必须保持该顺序。
