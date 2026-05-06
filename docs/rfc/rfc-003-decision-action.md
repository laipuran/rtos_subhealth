## RFC 003: 决策层动作任务流

**状态：** 草案

**修订日期：** 2026-05-06

**摘要：** 本 RFC 固定“单一入口任务流”的决策-执行链路，定义动作任务的最小语义结构与执行反馈约定，用于 tag chaining 场景。

---

### 1. 决策与范围
* **适用范围：** `ros2_ws/src` 内决策层、动作执行层、UI 适配层。
* **通信模型：** 决策层发布任务；执行层订阅任务并反馈执行状态；决策层订阅状态用于 UI 展示。
* **导航方式：** 不使用 Nav2；路径由 Dijkstra 在执行层计算。
* **定位方式：** Apriltag chaining，执行层保存原子移动栈用于撤回。
* **任务入口：** 单一入口任务流（统一 topic/接口，任务类型区分）。

---

### 2. 动作任务语义（最小结构）

#### 2.1 任务字段（建议）
| 字段 | 类型 | 说明 |
| :--- | :--- | :--- |
| `task_id` | string | 全局唯一任务 ID（UI/日志/重试使用） |
| `type` | string | 任务类型，如 `patrol_route`, `go_to_tag`, `hold`, `cancel` |
| `priority` | int32 | 优先级，值越大越高 |
| `route_id` | string | 预定义路线 ID（可选） |
| `target_tags` | int32[] | 目标 tag 序列（可选） |
| `target_task_id` | string | 控制类任务指向的目标任务（如 `cancel`），可选 |
| `constraints` | map | 约束，如速度上限、最小安全距离 |
| `deadline_ms` | int64 | 相对 `issue_time` 的超时(ms)，0 表示不设截止（可选） |
| `issue_time` | Time | 任务下发时间 |

#### 2.2 类型约定
* **巡航任务：** `type=patrol_route`，使用 `route_id` 或 `target_tags`。
* **点到点：** `type=go_to_tag`，使用 `target_tags`（长度=1）。
* **等待：** `type=hold`，执行层进入停机等待状态。
* **取消：** `type=cancel`，使用 `target_task_id` 指向被取消任务；`task_id` 仍为取消命令自身的全局唯一 ID。

---

### 3. 执行状态反馈（最小结构）

| 字段 | 类型 | 说明 |
| :--- | :--- | :--- |
| `task_id` | string | 对应任务 ID |
| `state` | string | `accepted`, `running`, `succeeded`, `failed`, `canceled`, `paused` |
| `progress` | float32 | 0.0-1.0 任务进度 |
| `current_tag` | int32 | 当前识别/对齐的 tag |
| `next_tag` | int32 | 计划到达的下一个 tag |
| `error_code` | string | 失败或异常码（可选） |
| `message` | string | 补充说明（可选） |
| `timestamp` | Time | 状态时间戳 |

---

### 4. 执行层行为要求
1. **路径规划：** 对 `target_tags` 使用 Dijkstra 计算 tag 路径。
2. **原子移动栈：** 按 tag 间原子移动入栈，用于撤回或恢复。
3. **取消处理：** 收到 `cancel` 后停止运动，发布 `canceled` 状态，并保留栈用于回退或下一任务。
4. **直接停机：** 收到 `hold` 立即停止并进入等待，直到新任务。
5. **感知依赖：** 执行层订阅 Apriltag 感知数据，闭环控制。

---

### 5. UI 对接原则
1. **只订阅执行结果：** UI 不直接依赖感知 topic。
2. **任务可追踪：** UI 以 `task_id` 展示进度、状态与失败原因。
3. **一致性：** UI 与日志以本 RFC 字段为准，避免私有字段。

---

### 6. 验收标准
1. **入口统一：** 所有动作任务通过单一入口流发布。
2. **可取消性：** 取消后不继续运动，状态回传为 `canceled`。
3. **可追踪性：** `task_id` 全程可追踪，状态变化完整。
4. **可等待性：** `hold` 任务可使机器人停机等待。

---
