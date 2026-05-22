## RFC 003: 决策层动作任务流

**状态：** 草案

**修订日期：** 2026-05-04

## 1. 摘要
本 RFC 规定决策层到执行层的单一任务入口，使用 ROS2 Action 统一任务下发、反馈与结果回传。目标是让 WebUI、决策层与执行层使用同一套任务接口，避免多个入口造成混乱。

---

## 2. 目标与非目标
**目标：**
1. 统一任务入口为一个 action，所有任务走同一路径。
2. 固定任务字段与状态字段，便于 WebUI 展示与日志记录。
3. 支持巡航、点到点与等待三类任务。

**非目标：**
1. 不定义 Apriltag 检测发布协议。
2. 不定义姿态调整与硬件控制方式。
3. 不实现具体 UI 页面与样式。

---

## 3. 现状与痛点
目前任务入口不统一，任务字段与状态字段容易出现不同版本，导致下游显示与处理不一致。

---

## 4. 方案概览
使用一个 action 作为唯一任务入口，任务类型用 `type` 区分，执行层统一回传 feedback/result。
数据流：WebUI -> 决策层 -> `task` action -> 执行层 -> feedback/result -> WebUI。

---

## 5. 关键接口
**Action 名称：** `task`

**消息位置：** `ros2_ws/src/orchestration/task_flow_interfaces/action/Task.action`

**Goal 字段：**
| 字段 | 类型 | 说明 |
| :--- | :--- | :--- |
| `type` | string | 任务类型：`patrol_route` / `go_to_tag` / `hold` |
| `priority` | int32 | 优先级，值越大越高 |
| `route_id` | string | 预定义路线 ID（可选） |
| `target_tags` | int32[] | 目标 tag 序列（可选） |
| `constraints` | map | 约束，如速度上限、最小安全距离 |
| `deadline_ms` | int64 | 截止时间（可选） |
| `issue_time` | Time | 任务下发时间 |

**Feedback 字段：**
| 字段 | 类型 | 说明 |
| :--- | :--- | :--- |
| `state` | string | `accepted`, `running`, `paused` |
| `finished_stage` | int32 | 已完成阶段数 |
| `total_stage` | int32 | 总阶段数 |
| `current_tag` | int32 | 当前识别/对齐的 tag |
| `next_tag` | int32 | 计划到达的下一个 tag |
| `error_code` | string | 失败或异常码（可选） |
| `message` | string | 补充说明（可选） |
| `timestamp` | Time | 状态时间戳 |

**Result 字段：**
| 字段 | 类型 | 说明 |
| :--- | :--- | :--- |
| `final_state` | string | `succeeded`, `failed`, `canceled` |
| `error_code` | string | 失败或异常码（可选） |
| `message` | string | 补充说明（可选） |
| `finished_time` | Time | 结束时间戳 |

---

## 6. 数据/控制流
1. WebUI 发起任务请求到决策层。
2. 决策层作为 action client 下发 `task`。
3. 执行层作为 action server 执行并反馈。
4. WebUI 通过反馈与结果展示任务状态。

---

## 7. 风险与替代
1. **替代方案：** 多入口 action（按任务类型拆分）。
   - **权衡：** 入口清晰，但 WebUI 与决策层需要处理多套接口。
2. **替代方案：** 使用 service/topic 下发任务。
   - **权衡：** 实现简单，但缺少任务生命周期与反馈。
3. **风险：** 单一 action 变更会影响所有任务类型。

---

## 8. 验证计划
1. 所有任务类型均通过 `task` action 下发。
2. cancel 后结果回传为 `canceled`。
3. goal_id 全程可追踪，状态变化完整。
4. `hold` 任务可使机器人停机等待。

---

## 9. 未决问题
1. `constraints` 字段的具体结构需要单独定义还是保持自由格式。
2. 是否需要创建 `ros2_ws/src/orchestration/task_flow_interfaces` 包与 `Task.action` 文件骨架。
