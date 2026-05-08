## RFC 003-EX: 动作执行层规范（执行层 & 路径规划）

**状态：** 草案

**修订日期：** 2026-05-06

## 1 概述
本规范是对 `rfc-003-decision-action.md` 中动作任务语义的补充与细化，目标是为后续多人协作实现动作执行层（executor）与路径规划模块（planner）提供统一约定：消息字段、topic、状态机、路径规划输入输出、执行历史语义、失败与恢复策略，以及拆包建议（执行核心与低层操控分离）。

适用范围：`ros2_ws/src` 下的执行层实现与与之交互的决策层、感知层（Apriltag）、UI 层。

设计原则：最小化耦合、明确边界、优先可测与可追踪的行为。

## 2 目标与交付物
- 统一 `ActionTask` / `ActionStatus` 字段定义（与 `rfc-003` 对齐）。
- 明确定义执行层状态机（状态、转移条件、异常处理）。
- 定义 Dijkstra 输入/输出约定与执行历史语义。
- 提供最小可运行任务行为说明（`go_to_tag` / `hold` / `cancel`）。

交付物（供实现使用）：本 md、状态机图（附件/PNG）、消息定义样例、验收测试清单。

## 3 消息与 Topic 约定
消息本体已在 `apriltag_interfaces` 包中定义并同步到构建配置，本文档只保留字段语义与使用约定，避免重复维护。

Topic 与 QoS（建议精确参数）
| Topic | Publisher | Type | Reliability | Durability | History | Depth | 说明 |
| :--- | :--- | :---: | :---: | :---: | :---: | :---: | :--- |
| `/action_tasks` | 决策层 | `ActionTask` | RELIABLE | VOLATILE | KEEP_LAST | 10 | 决策主动下发任务，executor 通常已在线，保持短历史便于重试 |
| `/action_status` | 执行层 | `ActionStatus` | RELIABLE | TRANSIENT_LOCAL | KEEP_LAST | 1 | UI/后端可作为 late-joiner 看到最近状态 |

说明：以上为默认建议，可根据网络与部署调整；使用 RELIABLE 保证任务与状态不丢失，TRANSIENT_LOCAL 让新订阅者看到最近状态。

备注：`constraints` 使用 `rcl_interfaces/Parameter[]`（键为 name，值为 ParameterValue），用于避免 JSON 解析歧义并支持强类型约束。
建议团队在实现初期先约定一小组稳定键名（例如 `max_speed_m_s`, `min_distance_m`, `planner_timeout_ms`），不认识的键忽略但要记录日志。

## 4 状态机（执行层核心）
核心状态：`idle` -> `accepted` -> `running` -> `succeeded` | `failed` | `canceled` | `paused`

补充约定（用于多人协作一致性）：
- 单任务模型：executor 同一时刻只维护一个“主任务”（`go_to_tag`/`patrol_route`）。`hold`/`cancel` 视为控制命令，不会成为主任务。
- 事件优先级：`cancel` > `hold` > 新主任务。
- 新主任务到达时的处理（preempt）：
  - 若当前处于 `running/accepted/paused` 且收到新的主任务：
    - `new.priority > current.priority`：允许抢占。对当前主任务发布 `canceled`（`message=preempted_by:<new_task_id>`），然后开始新任务。
    - 否则：拒绝新任务，对新任务发布 `failed`（`error_code=rejected_busy`）。
状态字段语义补充：
- `message`：由执行层生成的可读说明，主要用于 UI、日志与测试断言，典型内容包括“任务被接受”“规划失败原因”“被 `cancel`/`hold` 打断”“抢占来源任务 ID”。
- `error_code`：机器可读错误码，供上层稳定分支处理；`message` 只做解释，不承担状态判定责任。

简要转移表：
- 接收新任务 -> 校验 -> 发布 `accepted` -> 进入 `planning`
- 规划成功 -> 发布 `running` -> 执行路径，持续发布 `running`（更新 `progress/current_tag/next_tag`）
- 到达目标 -> 发布 `succeeded` -> 回到 `idle`
- 发生可恢复错误 -> 进入 `paused` 或 `failed`（取决策略），发布对应状态
- 收到 `cancel` -> 立即停止运动 -> 发布 `canceled` -> 保留执行历史用于调试或后续显式恢复任务
- 收到 `hold` -> 立即停止运动 -> 发布 `paused`（或自定义 `hold` 子状态）

异常处理要点：
- Tag 丢失/不稳定：短时重试（配置 `executor.tag_loss_retry` 次、每次等待 `executor.tag_loss_backoff_ms`），超过阈值进入 `paused` 并上报 `error_code=tag_lost`。
- 规划失败：回报 `failed` 并包含 `message`（可触发重试：`planner.retry_count`，`planner.timeout_ms`）。

## 5 最小可运行任务行为说明
1) `go_to_tag` (target_tags 长度=1)
  - 决策层发布 `ActionTask`（type=`go_to_tag`）。
  - 校验规则：`target_tags` 长度必须为 1；`route_id` 为空；`target_task_id` 必须为空。
  - 执行层接受并发布 `accepted`。
  - 起点确定：
    - 若 `start_tag >= 0`：使用显式起点。
    - 若 `start_tag < 0`：executor 从 `AprilTagDetections` 推断当前起点（推荐：过滤 `hamming<=threshold` 后取 `distance` 最小的 detection）。
    - 若无法推断：发布 `failed`（`error_code=start_tag_unknown`）。
  - 调用路径规划（Dijkstra）得到 tag 序列路径；若规划失败发布 `failed`。
  - 依次执行规划步骤，订阅 Apriltag 感知调整对齐；持续发布 `running`，更新进度。
  - 成功到达后发布 `succeeded`。

2) `hold`
  - 收到 `hold` 时，执行层立即停止并发布 `paused`（`task_id=当前主任务`, `message=hold_by:<hold_task_id>`）。
  - 保留当前移动栈与位置，直到收到新任务或取消。
  - 若没有主任务：发布 `failed`（`task_id=<hold_task_id>`, `error_code=no_active_task`）。

3) `cancel`
  - 决策层发布 `ActionTask`（type=`cancel`, `target_task_id=<要取消的任务>`）。
  - 执行层立即停止目标任务并发布 `canceled`（`task_id=target_task_id`, `message=canceled_by:<cancel_task_id>`）。执行历史保留供调试、可视化或后续显式恢复任务使用。
  - 兼容模式（对齐 `rfc-003` 旧写法）：若 `target_task_id` 为空，则将 `task_id` 视为目标任务 ID（仅用于取消命令；此时建议在日志中告警）。
  - 若目标任务不存在：发布 `failed`（`task_id=<cancel_task_id>`, `error_code=cancel_target_not_found`）。

验收标准（执行层核心）
- 能接收任务并在 `/action_status` 上按序发布 `accepted` -> `running` -> `succeeded|failed|canceled|paused`。
- `cancel`/`hold` 能中断运动并立即回传状态（响应时间小于 `executor.cancel_response_ms` 参数）。
- 对 `go_to_tag`，到达时 `progress==1.0`，`state==succeeded`。

执行轨迹可观测性建议：
- `current_tag`、`next_tag`、`completed_steps`、`total_steps` 已足以表达大部分 UI 进度。
- `progress` 不建议仅由轨迹 index 推导，因为真实执行可能在某个原子移动上停留更久；推荐用 `completed_steps / total_steps` 作为基础，必要时结合里程计/对齐状态做修正。
- 若调试和可视化需要完整路径，建议由执行层额外发布一个只读调试话题，例如 `planned_path_tags` 或 `planned_path_debug`，避免把大数组塞进主状态消息。

## 6 路径规划与执行历史
路径图定义（输入）
- Tag 图文件：`tag_graph.yaml`（参数形式），格式示例：
```
directed: false
weight_unit: distance_m
nodes: [1,2,3,4]
edges:
  - [1,2,1.0]
  - [2,3,1.2]
  - [3,4,0.8]
```
- `directed=false` 时边视为无向；`directed=true` 时仅表示单向边。
- 边格式为 `[from,to,weight]`，权重含义由 `weight_unit` 约定（距离/时间/风险成本等）。
- 图结构建议优先用邻接表在内存中表示，以适配稀疏图与 Dijkstra；输入格式可以保留 YAML 作为默认配置，同时允许 JSON / Protobuf / 其他序列化格式。

Dijkstra 输出（约定）
- 输出为 `int32[] path_tags`（包含起点与终点）。
可选调试输出：`path_edges` / `path_debug`，用于可视化路径。

执行历史设计
- 路径按相邻 tag 分解为执行步骤，executor 只维护当前步骤索引与已完成步骤数。
- 每步记录 `step_id`, `step_idx`, `from_tag`, `to_tag`, `planned_heading`, `control_params`。
- 规划完成后按顺序执行；每完成一步，递增 `completed_steps` 并更新 `last_move_id`。
- 取消/失败后默认只停止当前动作，不自动恢复；是否继续由决策层下发新任务决定。
- 图变化时应重新规划；旧执行历史仅用于调试和可视化。

## 7 执行层闭环控制接口假设
- 执行层订阅感知：直接使用 `apriltag_interfaces/AprilTagDetections` 作为输入，字段约定沿用现有 `apriltag_interfaces` 消息定义；本 RFC 不重复展开字段明细。
- 建议感知约束：频率 >= 5-10 Hz，最大延迟 `executor.max_detection_latency_ms`（例如 200 ms）。
- 控制接口（到移动控制包）建议：发布 `MoveCommand`（自定义 msg，包含 linear, angular, target_tag, move_id）或调用控制包 action；若拆包则通过本地 ROS topic 或 action 与低层控制器通信。

## 8 失败与恢复策略
- Tag 丢失：短时等待并重试（`executor.tag_loss_retry` 次），超过阈值进入 `paused` 并上报 `error_code=tag_lost`。
- 对齐失败（无法达到精度）：上报 `error_code=align_failed` 并进入 `paused`/`failed`。
- 规划超时/不可达：返回 `error_code=planner_unreachable` 并 `failed`，包含 `message`（不可达/超时）。

错误码（建议枚举）示例：
- `planner_unreachable`
- `tag_lost`
- `align_failed`
- `control_error`
- `timeout`
- `invalid_task`
- `rejected_busy`
- `start_tag_unknown`
- `cancel_target_not_found`
- `no_active_task`
- `unsupported_task_type`

建议在实现中使用这些标准字符串，便于上层统一处理与监控。

## 9 拆包建议（职责边界）
- 执行层核心（`action_executor`）：任务接收、状态机、路径规划调用、执行历史管理、与感知对接、发布状态。
- 低层操控包（`motion_controller`）：接收原子移动或 `MoveCommand`，负责底盘具体运动与低频闭环控制。两者通过明确 topic/service/action 交互。

## 10 参数与测试
- 参数：`tag_graph.yaml`、`planner.max_retries`、`planner.timeout_ms`、`executor.tag_loss_retry`、`executor.tag_loss_backoff_ms`、`executor.align_tolerance`、`executor.cancel_response_ms`
- 用例 1: `go_to_tag` -> `accepted` -> `running` -> `succeeded`，`progress==1.0`
- 用例 2: tag 丢失 -> `paused`，`error_code==tag_lost`
- 用例 3: `cancel` -> `canceled`
- 兼容性：`task_id` 全链路唯一；`cancel`/`hold` 必须通过 `/action_status` 体现
- 验收：状态机文档、消息定义、`executor_node` 状态发布、`path_tags` 与步骤记录

## 11 示例：最小流程示意
- 决策层发布 `ActionTask{task_id: "t1", type: "go_to_tag", target_tags: [5]}` 到 `/action_tasks`。
- `executor` 接收 -> 发布 `ActionStatus{task_id:"t1", state:"accepted"}` -> 规划 -> `ActionStatus{state:"running", current_tag:1, next_tag:2}` -> ... -> `ActionStatus{state:"succeeded"}`。

## 12 A/B 接口契约
- A：订阅 `/action_tasks`，校验任务，驱动状态机，发布 `/action_status`，调用规划接口，执行步骤
- B：实现 `ComputePath(start_tag, goal_tag, tag_graph) -> path_tags`，`BuildExecutionSteps(path_tags, constraints) -> execution_steps`，`ReplanIfGraphChanged(...) -> new_path`
- 数据结构：`path_tags: int32[]`；`execution_steps[]` 至少包含 `step_id`、`step_idx`、`from_tag`、`to_tag`、`control_params`

## 13 示例（精简）
```
ActionTask: go_to_tag(t1, target_tags=[5])
ActionStatus: accepted -> running -> succeeded
ActionTask: cancel(target_task_id=t1)
ActionStatus: canceled
```

## 14 实现顺序
- 1. 消息与状态机
- 2. Dijkstra 与执行步骤分解
- 3. 状态发布与控制接入
- 4. 集成测试

## 15 设计权衡（topic vs ROS2 Action）
- Topic + Status: 轻量、实现简单，便于广播状态给多个订阅者，但需要重复实现 goal/cancel/feedback 语义。
- ROS2 Action: 原生支持 goal/cancel/feedback/result，适合复杂的单一目标交互；若需要 preempt 与丰富的交互建议优先使用 Action。
- 建议：在初期使用 topic/Status 快速迭代契约；若后续交互复杂或需要更强的可靠语义，考虑迁移到 ROS2 Action（本文档字段可映射到 Action 的 goal/feedback/result）。

---
下一步（可选）：
- scaffold 一个最小 `action_executor` 包的骨架代码与 launch 用例用于本地迭代测试。
- 增加 `planner` 的独立包骨架（供成员 B 实现 Dijkstra 与执行步骤分解），并定义 A/B 间的 Python 接口或 ROS service/action。
