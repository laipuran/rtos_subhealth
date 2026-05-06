# RFC 003-EX: 动作执行层规范（执行层 & 路径规划）

**状态：** 草案

**修订日期：** 2026-05-06

## 1 概述
本规范是对 `rfc-003-decision-action.md` 中动作任务语义的补充与细化，目标是为后续多人协作实现动作执行层（executor）与路径规划模块（planner/stack）提供统一约定：消息字段、topic、状态机、路径规划输入输出、原子移动栈语义、失败与恢复策略，以及拆包建议（执行核心与低层操控分离）。

适用范围：`ros2_ws/src` 下的执行层实现与与之交互的决策层、感知层（Apriltag）、UI 层。

设计原则：最小化耦合、明确边界、优先可测与可回退的行为。

## 2 目标与交付物
- 统一 `ActionTask` / `ActionStatus` 字段定义（与 `rfc-003` 对齐）。
- 明确定义执行层状态机（状态、转移条件、异常处理）。
- 定义 Dijkstra 输入/输出约定与原子移动栈语义。
- 提供最小可运行任务行为说明（`go_to_tag` / `hold` / `cancel`）。

交付物（供实现使用）：本 md、状态机图（附件/PNG）、消息定义样例、验收测试清单。

## 3 消息与 Topic 约定
推荐在 `apriltag_interfaces` 包内添加两条消息：`ActionTask.msg` 与 `ActionStatus.msg`。

ActionTask.msg (建议)
```
string task_id
string type              # go_to_tag | patrol_route | hold | cancel | resume
int32 priority
string route_id
int32[] target_tags
int32 start_tag          # -1 表示自动推断起点；>=0 表示显式起点
string target_task_id    # cancel/resume 必填：被取消/恢复的任务 task_id
rcl_interfaces/Parameter[] constraints
int64 deadline_ms        # 相对 issue_time 的超时(ms)，0 表示不设截止
builtin_interfaces/Time issue_time
```

ActionStatus.msg (建议)
```
string task_id
string state            # accepted|running|succeeded|failed|canceled|paused
float32 progress
int32 current_tag
int32 next_tag
int32 goal_tag           # 本任务最终目标 tag（未知填 -1）
int32 total_steps        # 原子移动总步数（未知填 0）
int32 completed_steps    # 已完成原子移动步数
string last_move_id     # optional: 最近完成的原子移动 ID
string error_code       # 参考附件错误码枚举
string message
builtin_interfaces/Time timestamp
```

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
- 单任务模型：executor 同一时刻只维护一个“主任务”（`go_to_tag`/`patrol_route`）。`hold`/`cancel`/`resume` 视为控制命令，不会成为主任务。
- 事件优先级：`cancel` > `hold` > `resume` > 新主任务。
- 新主任务到达时的处理（preempt）：
  - 若当前处于 `running/accepted/paused` 且收到新的主任务：
    - `new.priority > current.priority`：允许抢占。对当前主任务发布 `canceled`（`message=preempted_by:<new_task_id>`），然后开始新任务。
    - 否则：拒绝新任务，对新任务发布 `failed`（`error_code=rejected_busy`）。
- `paused` 恢复：
  - `resume`（扩展）用于继续同一主任务；或下发新的主任务替换当前上下文。

简要转移表：
- 接收新任务 -> 校验 -> 发布 `accepted` -> 进入 `planning`
- 规划成功 -> 发布 `running` -> 执行路径，持续发布 `running`（更新 `progress/current_tag/next_tag`）
- 到达目标 -> 发布 `succeeded` -> 回到 `idle`
- 发生可恢复错误 -> 进入 `paused` 或 `failed`（取决策略），发布对应状态
- 收到 `cancel` -> 立即停止运动 -> 发布 `canceled` -> 保留移动栈用于回退或下一任务
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
  - 依次执行原子移动（见原子移动栈），订阅 Apriltag 感知调整对齐；持续发布 `running`，更新进度。
  - 成功到达后发布 `succeeded`。

2) `hold`
  - 收到 `hold` 时，执行层立即停止并发布 `paused`（`task_id=当前主任务`, `message=hold_by:<hold_task_id>`）。
  - 保留当前移动栈与位置，直到收到新任务或取消。
  - 若没有主任务：发布 `failed`（`task_id=<hold_task_id>`, `error_code=no_active_task`）。

3) `cancel`
  - 决策层发布 `ActionTask`（type=`cancel`, `target_task_id=<要取消的任务>`）。
  - 执行层立即停止目标任务并发布 `canceled`（`task_id=target_task_id`, `message=canceled_by:<cancel_task_id>`）。移动栈保留供用户选择回退或重试。
  - 兼容模式（对齐 `rfc-003` 旧写法）：若 `target_task_id` 为空，则将 `task_id` 视为目标任务 ID（仅用于取消命令；此时建议在日志中告警）。
  - 若目标任务不存在：发布 `failed`（`task_id=<cancel_task_id>`, `error_code=cancel_target_not_found`）。

4) `resume`（扩展，可选）
  - 语义：恢复 `paused` 的主任务。`target_task_id` 必填。
  - 若恢复成功：对 `target_task_id` 发布 `running`（`message=resume_by:<resume_task_id>`）。
  - 若 executor 未实现 resume：发布 `failed`（`task_id=<resume_task_id>`, `error_code=unsupported_task_type`）。

验收标准（执行层核心）
- 能接收任务并在 `/action_status` 上按序发布 `accepted` -> `running` -> `succeeded|failed|canceled|paused`。
- `cancel`/`hold` 能中断运动并立即回传状态（响应时间小于 `executor.cancel_response_ms` 参数）。
- 对 `go_to_tag`，到达时 `progress==1.0`，`state==succeeded`。

## 6 路径规划与原子移动栈
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

Dijkstra 输出（约定）
- 输出为 `int32[] path_tags`（包含起点与终点）。

原子移动栈设计
- 概念：将完整路径分解为“原子移动”（move between adjacent tags）。每个原子移动记录：
  - `move_id` (string, 唯一)
  - `seq_idx` (int, 从0开始的序号)
  - `from_tag`, `to_tag`
  - `planned_heading`, `control_params`（JSON 字符串）
  - `timestamp_planned`
- 入栈时机：规划完成后按路径顺序将原子移动入队列（实施时按队列弹出）。
- 出栈/确认：每完成一个原子移动（到达 `to_tag` 并对齐）后发布确认并从队列弹出；确认消息可写入 `ActionStatus.last_move_id`。
- 撤回逻辑：在 `cancel` 或失败需要撤回时，使用已完成的历史移动逆序生成回退路径（由决策层授权），或直接停机并由决策层决定后续。

性能/一致性注意：在执行过程中如果图结构或感知发生变化（边权改变、tag 新增/丢失），应重新规划并重构原子移动栈。

## 7 执行层闭环控制接口假设
- 执行层订阅感知：建议以 `apriltag_interfaces/AprilTagDetections` 为输入（见 `apriltag_interfaces/msg/AprilTagDetections.msg`）。字段约定：
  - `timestamp`：本帧检测时间
  - `frame_id`：坐标系/相机标识
  - `detections[]`：每个元素为 `AprilTagDetection`（见 `apriltag_interfaces/msg/AprilTagDetection.msg`），至少用到：
    - `id`
    - `distance`
    - `center_offset_x`, `center_offset_y`
    - `roll`, `pitch`, `yaw`
    - `hamming`（可用于质量过滤，阈值由参数配置）
- 建议感知约束：频率 >= 5-10 Hz，最大延迟 `executor.max_detection_latency_ms`（例如 200 ms）。
- 控制接口（到移动控制包）建议：发布 `MoveCommand`（自定义 msg，包含 linear, angular, target_tag, move_id）或调用控制包 action；若拆包则通过本地 ROS topic 或 action 与低层控制器通信。

## 8 失败与恢复策略
- Tag 丢失：短时等待并重试（`executor.tag_loss_retry` 次），超过阈值进入 `paused` 并上报 `error_code=tag_lost`。
- 对齐失败（无法达到精度）：退回上一个已确认 tag（若可行），或上报 `error_code=align_failed` 并进入 `paused`/`failed`。
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
- 执行层核心（`action_executor`）：任务接收、状态机、路径规划调用、原子移动栈管理、与感知对接、发布状态。
- 低层操控包（`motion_controller`）：接收原子移动或 `MoveCommand`，负责底盘具体运动与低频闭环控制。两者通过明确 topic/service/action 交互。

## 10 参数、配置与测试
参数样例：
- `tag_graph.yaml`（图，增加 `directed: true|false` 字段，权重单位说明）
- 配置项：`planner.max_retries`, `planner.timeout_ms`, `executor.tag_loss_retry`, `executor.tag_loss_backoff_ms`, `executor.align_tolerance`, `executor.cancel_response_ms` 等

测试建议（示例用例，便于写自动化测试）：
- 用例 1: go_to_tag 成功
  - 前提：tag_graph 已加载，感知发布全部必要 tag
  - 操作：发布 `ActionTask`(type=`go_to_tag`, target_tags=[T])
  - 断言：在 `planner.timeout_ms` 内收到 `accepted`，规划成功并进入 `running`，最后 `succeeded`，`progress==1.0`。
- 用例 2: tag 丢失导致 paused
  - 前提：在执行过程中人为停止感知某个 tag
  - 操作：发布 `go_to_tag`，在中间阶段停止发布中间 tag
  - 断言：executor 在 `executor.tag_loss_retry` 次重试后发布 `paused` 且 `error_code==tag_lost`。
- 用例 3: cancel 响应
  - 操作：在 `running` 状态下发布类型为 `cancel` 的任务（`target_task_id=<当前主任务 task_id>`）
  - 断言：executor 立即停止并发布 `canceled`，响应时间 < `executor.cancel_response_ms`。

额外验收点（接口兼容性）
- `task_id` 必须全链路唯一且可追踪（UI/日志/重试）。
- `cancel`/`hold` 的结果必须通过 `/action_status` 体现（状态与错误码一致）。

验收清单（与交付物对应）
- 状态机文档 + 流程图
- `ActionTask.msg` 与 `ActionStatus.msg`（实现并放入 `apriltag_interfaces/msg/`）
- `executor_node` 能在仿真/测试下通过 `/action_tasks` 接收任务并在 `/action_status` 上发布状态
- 给定 `tag_graph` 与 `target_tags` 能输出合理路径且记录原子移动（含 `move_id`/`seq_idx`）

## 11 示例：最小流程示意
- 决策层发布 `ActionTask{task_id: "t1", type: "go_to_tag", target_tags: [5]}` 到 `/action_tasks`。
- `executor` 接收 -> 发布 `ActionStatus{task_id:"t1", state:"accepted"}` -> 规划 -> `ActionStatus{state:"running", current_tag:1, next_tag:2}` -> ... -> `ActionStatus{state:"succeeded"}`。

## 12 A/B 协作接口契约（建议）
本节用于两位成员（A/B）在不互相侵入实现细节的情况下协作：A 负责任务驱动与状态机；B 负责规划与原子移动栈。

成员 A（执行层核心）职责
- 订阅 `/action_tasks`，做任务校验与主流程驱动
- 发布 `/action_status`（`accepted/running/paused/failed/canceled/succeeded`）
- 管理主任务生命周期与抢占规则
- 调用 B 的规划接口得到 `path_tags` 与 `atomic_moves`
- 调用低层控制接口执行原子移动，并在完成后确认（更新 `last_move_id/completed_steps`）

成员 B（路径规划与原子移动栈）职责
- Dijkstra 规划：`ComputePath(start_tag, goal_tag, tag_graph) -> path_tags`
- 路径分解：`BuildAtomicMoveStack(path_tags, constraints) -> atomic_moves`
- 撤回/恢复策略：`Rollback(history, current_tag) -> recovery_moves`（可选）
- 图变化重规划：`ReplanIfGraphChanged(current_path, graph_update) -> new_path`（可选）

建议 A/B 之间的“数据结构”最小约定
- `path_tags`: `int32[]`，包含起点与终点
- `atomic_moves[]`: 每个元素至少包含 `move_id, seq_idx, from_tag, to_tag, control_params`

## 13 示例：契约样例与状态序列
go_to_tag 示例（伪 YAML，重点展示字段含义）：
```
task_id: t1
type: go_to_tag
priority: 10
target_tags: [5]
start_tag: -1
constraints:
  - name: max_speed_m_s
    value: { double_value: 0.5 }
deadline_ms: 0
issue_time: { sec: 1680000000, nanosec: 0 }
```

cancel 示例：
```
task_id: c1
type: cancel
target_task_id: t1
issue_time: { sec: 1680000001, nanosec: 0 }
```

对应的一系列 `ActionStatus`（示例）：
```
{ "task_id":"t1","state":"accepted","timestamp":... }
{ "task_id":"t1","state":"running","progress":0.2,"current_tag":1,"next_tag":2,"goal_tag":5,"total_steps":4,"completed_steps":1 }
{ "task_id":"t1","state":"running","progress":0.6,"current_tag":2,"next_tag":3,"last_move_id":"m2","completed_steps":2 }
{ "task_id":"t1","state":"succeeded","progress":1.0,"completed_steps":4 }
```

## 14 开发与协作建议
- 先实现消息与状态机（主要契约），并用 mock 感知数据做端到端测试。
- 成员分工：A 实现执行层状态机与主流程；B 实现 Dijkstra/planner 与原子移动栈；接口用小例子（YAML 图 + 单轮任务）验证。
- 代码审查点：状态边界（何时发布 accepted/running 等）、重试与超时参数、栈一致性（原子移动的确认要幂等）。

## 15 设计权衡（topic vs ROS2 Action）
- Topic + Status: 轻量、实现简单，便于广播状态给多个订阅者，但需要重复实现 goal/cancel/feedback 语义。
- ROS2 Action: 原生支持 goal/cancel/feedback/result，适合复杂的单一目标交互；若需要 preempt 与丰富的交互建议优先使用 Action。
- 建议：在初期使用 topic/Status 快速迭代契约；若后续交互复杂或需要更强的可靠语义，考虑迁移到 ROS2 Action（本文档字段可映射到 Action 的 goal/feedback/result）。

---
下一步（可选）：
- scaffold 一个最小 `action_executor` 包的骨架代码与 launch 用例用于本地迭代测试。
- 增加 `planner` 的独立包骨架（供成员 B 实现 Dijkstra 与原子移动栈），并定义 A/B 间的 Python 接口或 ROS service/action。
