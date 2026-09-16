# ADR-002：设备契约与单任务优先编排

- **状态：** 已接受
- **日期：** 2026-09-13

## 背景

旧执行层假定设备是 Unitree GO2，并固定使用 `stand_up`/`damp`/
`recovery_stand`、`moving → approaching → aligning → stabilizing` 状态机和
tag graph Dijkstra。这套假设无法推广到轮式底盘、舵机人形机器人或未来设备。
首个目标设备 TonyPi 只提供命名动作组和原始舵机脉冲，不提供速度控制、里程计或
`cmd_vel`。

## 决策

1. **基于能力的设备契约。** 每个 adapter 发布
   `DeviceDescriptor` (kind, capabilities, supported primitives, limits, frames,
   sensors) and periodic `DeviceState`. The orchestrator routes to devices by
   advertised capability, never by assumed hardware.
2. **Primitive 可选。** 通用 primitive 包括 `move_to_pose`、
   `set_velocity`, `hold`, `stop`. Device-specific behavior uses the generic
   `execute_primitive` (name + `params_json`) so new devices do not need new ROS
   messages. A servo humanoid implements only `execute_primitive`/`hold`/`stop`.
3. **目标与设备无关。** `TaskTarget` 支持 `tag`、`pose`、
   `waypoint`, and `action`. Tag chaining becomes one localization provider, not
   the core model.
4. **单任务优先。** 对外任务契约是单个 `DeviceTask`
   (one goal, one device, one primitive). The workflow/step superset is
   deliberately deferred.
5. **为未来工作流引擎保留扩展点：**
   - `TaskTarget` and primitives are reusable as workflow step payloads.
   - Persistence stores the task spec as JSON, so a step list can be added
     without a schema change.
    - `Orchestrator` validation/lifecycle logic is independent of the transport
      and can be driven by a future multi-step scheduler.
    - The HTTP entry point (`POST /api/v1/tasks`) is unchanged.

6. **执行位于机器人端。** endpoint adapter 可以是 Rust、C++ 或
   Python process. The orchestrator only depends on the ROS device contract; it
   never depends on a vendor SDK or on the adapter implementation language.

7. **TonyPi 配置。** TonyPi 发布 `execute_primitive`、`hold` 和
   `stop` (plus configured named action-group primitives). It does not advertise
   `set_velocity`, `move_to_pose`, odometry, or pose feedback unless a concrete
   implementation provides those capabilities.

## 后果

- 轮式底盘和舵机人形机器人可以由同一个 orchestrator 通过不同能力集合处理。
- `MoveToPose` 并非普遍可用；规划器必须在生成位姿任务前检查能力。
- 延后工作流引擎可以避免过早引入 DAG 复杂度，同时保留后续接入点。
- 旧 `ExecTask` 接口暂时保留以兼容现有调用方，待新节点栈替代旧 Python 包后再退役。
