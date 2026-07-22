## RFC 005: Desc Layer 服务与对外接口

**状态：** 草案

**修订日期：** 2026-05-22

## 1. 摘要
desc_layer 负责把跨网段请求转成 ROS2 `task` action，并汇总执行状态对外发布。它部署在 `ros2_ws/src/orchestration/desc_layer`，通过 HTTP/WS 提供 WebUI 与其他上层调用入口。

---

## 2. 目标与非目标
**目标：**
1. 统一对外任务入口，所有任务经 desc_layer 下发。
2. 提供 HTTP/WS 接口，支持跨网段访问。
3. 汇总并推送任务状态，便于 WebUI 展示。

**非目标：**
1. 不让 WebUI 直接访问 ROS2 DDS。
2. 不实现执行层的硬件控制逻辑。
3. 不定义 `task` action 的字段（见 RFC 003）。

---

## 3. 现状与痛点
WebUI 与外部系统处在不同网段时，无法直接使用 ROS2 DDS。没有 desc_layer 会导致任务入口分散，调用方式不统一。

---

## 4. 方案概览
desc_layer 作为唯一入口接收 HTTP/WS 请求，转成 `task` action，并把反馈与结果聚合后推送给 WebUI。
数据流：WebUI/外部系统 -> desc_layer -> `task` action -> 执行层 -> desc_layer -> WebUI。

---

## 5. 关键接口
**HTTP API：**
- `POST /api/v1/tasks`：下发任务（请求体对应 RFC 003 Goal）。
- `GET /api/v1/tasks/{goal_id}`：查询状态与结果。
- `POST /api/v1/tasks/{goal_id}/cancel`：取消任务。
- `GET /api/v1/tasks`：列表查询（分页/时间范围）。

**WebSocket：**
- `WS /api/v1/events`：推送任务状态变更与结果。
- 消息体：`{ goal_id, feedback?, result?, timestamp }`

**状态字段约定：**
- 使用 RFC 003 的字段；反馈中包含 `finished_stage` 与 `total_stage`。

**访问控制：**
- API 访问鉴权采用 Token 或 API Key；缺省拒绝。

---

## 6. 数据/控制流
1. desc_layer 接收 HTTP/WS 请求并校验字段。
2. desc_layer 作为 action client 下发 `task`。
3. desc_layer 聚合 feedback/result 并推送 WebUI。
4. 任务结束后落库或保留记录（实现可选）。

---

## 7. 风险与替代
1. **替代方案：** WebUI 直接使用 ROS2 DDS。
   - **权衡：** 延迟低，但跨网段部署困难。
2. **替代方案：** 使用 gRPC 网关代替 HTTP/WS。
   - **权衡：** 接口更强，但实现与运维成本更高。
3. **风险：** desc_layer 单点不可用会影响任务入口。

---

## 8. 验证计划
1. WebUI 可通过 HTTP/WS 正常下发与取消任务。
2. desc_layer 能正确转发 `task` action 并接收反馈。
3. WebUI 能收到 `finished_stage/total_stage` 与终态结果。

---

## 9. 未决问题
1. Token 与 API Key 的具体格式与更新机制。
2. 任务记录保存多久、是否需要持久化。
