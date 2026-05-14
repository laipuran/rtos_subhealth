## RFC 004: 执行层与规划层接口说明

**状态：** 草案

**修订日期：** 2026-05-14

**摘要：** 本文定义执行层与路径规划层的最小接口、字段语义、错误码以及 `progress` / `current_tag` / `next_tag` 更新规则。该说明用于配套 RFC 003 的单一入口动作任务流，确保执行、日志与 UI 对状态理解一致。

---

### 1. 接口边界
1. **规划层职责：** 基于 tag 图与约束生成路径，必要时重规划。
2. **执行层职责：** 调用规划、驱动动作与感知闭环控制、维护任务状态与反馈。
3. **关注点隔离：** 规划层不处理动作控制、感知订阅与动作生命周期。

---

### 2. 接口定义

#### 2.1 PlanRequest（执行层 -> 规划层）
| 字段 | 类型 | 说明 |
| :--- | :--- | :--- |
| `goal_id` | string | 任务唯一标识。 |
| `task_type` | string | 任务类型：`patrol_route` / `go_to_tag`。 |
| `route_id` | string | 预定义路线 ID（可选）。 |
| `target_tags` | int32[] | 目标序列；`go_to_tag` 长度必须为 1。 |
| `start_tag` | int32 | 当前已对齐的 tag；未知时传 `-1`。 |
| `constraints.max_speed_mps` | float32 | 速度上限（可选）。 |
| `constraints.min_clearance_m` | float32 | 最小安全距离（可选）。 |
| `constraints.avoid_tags` | int32[] | 需避让的 tag（可选）。 |
| `deadline_ms` | int64 | 任务截止时间（epoch ms，可选）。 |
| `allow_partial` | bool | 不可达时是否返回可执行前缀。 |
| `replan_reason` | string | 重规划原因（可选）：`lost_tag` / `blocked` / `manual` 等。 |

#### 2.2 PlanResponse（规划层 -> 执行层）
| 字段 | 类型 | 说明 |
| :--- | :--- | :--- |
| `plan_id` | string | 规划实例 ID。 |
| `plan_state` | string | `ok` / `partial` / `failed`。 |
| `path_tags` | int32[] | 完整路径（含起点与终点）。 |
| `segments` | object[] | 路径段列表，见 2.3。 |
| `total_cost` | float32 | 返回路径的总成本（权重之和）。 |
| `estimated_time_ms` | int64 | 预计耗时（可选）。 |
| `next_tag` | int32 | 下一目标 tag；无则 `-1`。 |
| `error_code` | string | 失败或部分成功时必填。 |
| `message` | string | 补充说明（可选）。 |

#### 2.3 Segment 结构
| 字段 | 类型 | 说明 |
| :--- | :--- | :--- |
| `from_tag` | int32 | 起点 tag。 |
| `to_tag` | int32 | 终点 tag。 |
| `edge_cost` | float32 | 边权重。 |
| `edge_id` | string | 可选的边 ID。 |

---

### 3. 错误码（规划层）
| 错误码 | 说明 |
| :--- | :--- |
| `OK` | 成功。 |
| `INVALID_GOAL` | 目标字段非法或缺失。 |
| `GRAPH_MISSING` | 图未加载或版本不匹配。 |
| `START_UNKNOWN` | 起点未知且无法推断。 |
| `TARGET_UNKNOWN` | 目标 tag 不存在。 |
| `NO_ROUTE` | 不可达。 |
| `CONSTRAINT_VIOLATION` | 约束不可满足。 |
| `TIMEOUT` | 规划超时。 |
| `CANCELED` | 调用方取消。 |
| `INTERNAL` | 内部异常。 |

---

### 4. 状态字段更新规则

#### 4.1 `progress` 规则
1. **范围：** 取值范围为 $[0, 1]$，单调不回退。
2. **基准算法：** 设总段数 `N = len(path_tags) - 1`，已完成段数 `K`，则候选进度为 $candidate\_progress = K / N$。
3. **零段处理：** 若 `N <= 0`（起点即终点），候选进度为 `1.0`。
4. **重规划规则：** 若发生重规划（如存在 `replan_reason`）或 `path_tags` 变化导致 `N` 改变，执行层可按新的有效路径计算候选进度，但对外发布值必须满足 `progress = max(last_reported_progress, candidate_progress)`，不得因切换到新路径而回退。
5. **段内细化：** 若可获得段内里程或时间进度，可在当前段内线性插值，但对外发布值仍不得小于历史已发布值。
6. **部分路径：** `plan_state=partial` 时可按返回的路径长度计算候选进度，但发布时仍需应用单调约束，避免因部分路径或后续重规划导致进度回退。
7. **取消任务：** `canceled` 结果保持最后已发布值，不回退。

#### 4.2 `current_tag` 规则
1. **定义：** 最近一次完成对齐的 tag。
2. **初始化：** 规划可用且 `start_tag` 合法时，执行层应将 `current_tag` 设为 `start_tag`；否则为 `-1`。
3. **更新：** 到达并确认新 tag 后立即更新。
4. **停机/取消：** `hold` / `cancel` 时保持最后确认值。

#### 4.3 `next_tag` 规则
1. **定义：** 执行中下一目标 tag。
2. **更新：** 若 `path_tags` 长度 > 1，`next_tag = path_tags[1]`；否则为 `-1`。
3. **完成/暂停/取消：** 置为 `-1`。

---

### 5. 约束与一致性
1. **类型约束：** `go_to_tag` 必须 `target_tags` 长度为 1；否则返回 `INVALID_GOAL`。
2. **冲突处理：** 同时提供 `route_id` 与 `target_tags` 时，以 `route_id` 为准；若 `route_id` 无效，返回 `INVALID_GOAL`。
3. **起点语义：** `start_tag=-1` 时规划层需尝试推断；无法推断则返回 `START_UNKNOWN`。
4. **返回语义：** `plan_state=ok` 时 `error_code=OK`；`partial`/`failed` 时 `error_code` 不得为 `OK`。
5. **不可达策略：** `allow_partial=true` 时可返回 `partial` 并携带可执行前缀；无前缀仍返回 `failed`。
6. **路径完整性：** `path_tags` 不得包含 `-1`；`plan_state=failed` 时必须为空数组。
7. **任务覆盖：** `hold` 不触发规划调用，由执行层直接处理。
8. **一致性要求：** 执行层、日志与 UI 均以本说明字段语义为准。

---

### 6. 参考
* [RFC 003: 决策层动作任务流](rfc-003-decision-action.md)
