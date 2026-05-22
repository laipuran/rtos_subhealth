## RFC 004: 决策层到执行层的规划字段说明

**状态：** 草案

**修订日期：** 2026-05-14

## 1. 摘要
本 RFC 说明决策层发送到执行层的规划字段、错误码和 `current_tag` / `next_tag` 更新规则。该内容作为 RFC 003 的 `task` action 约定使用，不引入新的 ROS2 接口。目标是让执行层上报给决策层与 UI 的状态清晰一致。

---

## 2. 目标与非目标
**目标：**
1. 明确决策层发送到执行层的字段，避免解释不一致。
2. 固定错误码集合，便于上层处理失败与部分可达。
3. 统一 `progress` / `current_tag` / `next_tag` 的更新规则。

**非目标：**
1. 不定义决策层的任务调度策略。
2. 不定义底层硬件控制方式。
3. 不定义 Apriltag 感知与相机输入。

---

## 3. 现状与痛点
执行层接收规划相关字段后再内部执行，如果字段或更新规则不清楚，上报状态容易出现理解不一致，影响决策层与 UI 判断。

---

## 4. 方案概览
用一套简单字段说明“决策层发给执行层的规划内容”，并作为 `task` action 的约定，保证上报状态稳定可读。
规划与控制都在执行层内部完成；失败后由执行层上报给决策层，是否回退由决策层决定。

---

## 5. 关键接口
以下结构为 `task` action 的一部分，不是独立 ROS2 service/action。

**PlanRequest（决策层 -> 执行层）：**
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

**PlanResponse（执行层内部使用，用于生成反馈）：**
| 字段 | 类型 | 说明 |
| :--- | :--- | :--- |
| `plan_id` | string | 规划实例 ID。 |
| `segments` | object[] | 路径段列表，见下表；可为空数组表示起点即终点。 |
| `next_tag` | int32 | 下一目标 tag；无则 `-1`。 |
| `error_code` | string | `OK` / `PARTIAL` / 失败码。 |
| `message` | string | 补充说明（可选）。 |

**Segment：**
| 字段 | 类型 | 说明 |
| :--- | :--- | :--- |
| `from_tag` | int32 | 起点 tag。 |
| `to_tag` | int32 | 终点 tag。 |
| `edge_cost` | float32 | 边权重。 |
| `edge_id` | string | 可选的边 ID。 |

**错误码：** `OK` / `PARTIAL` / `INVALID_GOAL` / `GRAPH_MISSING` / `START_UNKNOWN` / `TARGET_UNKNOWN` / `NO_ROUTE` / `CONSTRAINT_VIOLATION` / `TIMEOUT` / `CANCELED` / `INTERNAL`。

**更新规则：**
1. `current_tag` 为最近完成对齐的 tag；`start_tag` 合法时可初始化为 `start_tag`。
2. `next_tag` 为 `segments[0].to_tag`；完成/暂停/取消时置 `-1`。

---

## 6. 数据/控制流
1. 决策层下发 `task` action 到执行层。
2. 执行层内部根据请求生成 `segments`。
3. 控制功能按 `segments` 驱动硬件控制动作。
4. 执行层通过 action feedback/result 上报状态给决策层与 UI。

---

## 7. 风险与替代
1. **替代方案：** 将规划做成独立 ROS2 service。
   - **权衡：** 接口清晰，但引入额外通信与维护成本。
2. **替代方案：** 让决策层直接规划路径。
   - **权衡：** 决策层更早做出路径，但难以掌握执行时的实时约束。
3. **风险：** 内部约定被多处引用后难以变更，需要严格维护本 RFC。

---

## 8. 验证计划
1. `task_type` / `route_id` / `target_tags` 非法时返回 `INVALID_GOAL`。
2. `allow_partial` 在可达/不可达场景下行为符合约定。
3. `progress` / `current_tag` / `next_tag` 更新符合规则。

---

## 9. 未决问题
1. 控制底层硬件的具体方式（直接驱动或动作组调用）。
2. 重规划信号如何传递到规划功能（由谁触发、如何触发）。
