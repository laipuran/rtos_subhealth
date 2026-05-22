## RFC 006: WebUI 任务操作界面

**状态：** 草案

**修订日期：** 2026-05-22

## 1. 摘要
WebUI 提供任务下发与任务状态展示，仅通过 desc_layer 的 HTTP/WS 接口访问系统，不直接访问 ROS2 DDS。

---

## 2. 目标与非目标
**目标：**
1. 支持 `patrol_route` / `go_to_tag` / `hold` 任务下发。
2. 展示任务状态与失败原因。
3. 支持任务取消与历史查询。

**非目标：**
1. 不直接访问 ROS2 DDS。
2. 不在 WebUI 中实现执行层控制逻辑。
3. 不替代 desc_layer 的任务校验与状态聚合。

---

## 3. 现状与痛点
WebUI 与机器人常处于不同网络，直接访问 ROS2 DDS 不可行，需要一个稳定的 HTTP/WS 入口。

---

## 4. 方案概览
WebUI 通过 desc_layer 的 HTTP/WS 接口完成任务下发与状态订阅。
数据流：WebUI -> desc_layer -> `task` action -> 执行层 -> desc_layer -> WebUI。

---

## 5. 关键接口
**HTTP：**
- `POST /api/v1/tasks`
- `GET /api/v1/tasks/{goal_id}`
- `POST /api/v1/tasks/{goal_id}/cancel`
- `GET /api/v1/tasks`

**WebSocket：**
- `WS /api/v1/events`

**字段映射：**
1. 任务状态：`feedback.state`
2. 阶段进度：`feedback.finished_stage` / `feedback.total_stage`
3. 当前/下一 tag：`feedback.current_tag` / `feedback.next_tag`
4. 终态：`result.final_state`
5. 失败原因：`error_code` / `message`

---

## 6. 数据/控制流
1. WebUI 发起 HTTP 请求下发任务。
2. WebUI 使用 WS 订阅任务状态变更。
3. WebUI 展示阶段进度与终态结果。

---

## 7. 风险与替代
1. **替代方案：** 直接用 ROS2 DDS 通信。
   - **权衡：** 延迟更低，但跨网段部署困难。
2. **替代方案：** 使用 gRPC 代替 HTTP/WS。
   - **权衡：** 接口更强，但前端接入成本更高。
3. **风险：** desc_layer 不可用会导致 WebUI 无法下发任务。

---

## 8. 验证计划
1. WebUI 可通过 HTTP/WS 完成任务下发与取消。
2. 任务状态与阶段进度可实时更新。
3. WebUI 展示字段与 RFC 003 一致。

---

## 9. 未决问题
1. WebUI 是否需要离线缓存与重连策略。
2. 历史查询默认条数与分页方式。

---

## 10. 参考
* [RFC 003: 决策层动作任务流](rfc-003-decision-action.md)
* [RFC 005: Desc Layer 服务与对外接口](rfc-005-desc-layer-service.md)
