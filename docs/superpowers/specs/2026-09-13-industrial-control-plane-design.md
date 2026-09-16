# 工业控制平面设计

## 目的

将 Rust/ROS 2 迁移落地为可靠的操作员控制平面，用于移动机器人任务执行、地图
管理和生理监测。主要用户是现场操作员和场地工程师。他们需要准确了解设备可用性、
任务状态、地图安全性和诊断告警，不应被要求理解 ROS 传输细节。

## 架构

gateway 是生产环境唯一的 Web 入口和 Web 域边界。它负责校验命令、持久化操作员
可见状态，并提供稳定的 REST/WebSocket 协议。ROS bridge 是边缘 adapter，负责将
gateway 的规范命令和事件转换为 ROS action 与 topic，不包含 Web 兼容策略。

```text
WebUI -> Gateway API/state projection -> Bridge adapter -> ROS actions/topics
                 ^                         |
                 +---- persisted events ---+
```

独立 mock gateway 保留用于开发和契约测试；生产部署只运行 `gateway_bridge`。

## 命令模型

规范任务命令采用设备导向结构，已由
`DeviceTask`: `device_id`, `primitive`, `target`, `params_json`, constraints,
and deadline. The gateway rejects incomplete canonical commands.

旧 WebUI 结构（`goal.type`、`target_tags`）暂时只保留在
the HTTP edge only. Its conversion is explicit and total: a `go_to_tag` becomes
`move_to_pose` with a tag target, `hold` becomes `hold`, and patrol requires an
explicit route conversion. Unsupported or ambiguous legacy input returns an
API error; no missing field is silently converted to `hold`.

## 状态与事件

gateway 持久化 accepted、running、终态和取消状态转换。
Each WebSocket event carries a monotonic per-process sequence number. Clients
use one multiplexed socket and resync REST resources after reconnect or a
sequence gap. Task results and lifecycle state are durable; telemetry remains
best-effort.

## 安全与运维

Map scene identifiers use a restricted identifier grammar and maps are written
with a temporary file plus rename. Browser authentication uses the existing API
token through an explicit REST header and WebSocket authentication message.
Hardware RPC has bounded connect, read, and write operations. Safety health is
fed by device/backend communication rather than a self-refreshing timer.

## 操作员体验

The WebUI submits canonical commands, presents device availability and task
failure reasons, maintains a single event connection, and preserves named map
routes. The current task, map, and diagnosis workflows remain available while
their data source becomes the canonical gateway model.

## 交付阶段

1. 加固并规范 gateway 的命令、地图和事件边界。
2. 在 ROS bridge 中完成任务生命周期转发和持久化状态投影。
3. 将诊断与体征 topic 接入 gateway 持久化和事件系统。
4. 将 WebUI 升级为规范命令、单一认证事件流和无损地图编辑。
5. 为关键生产路径增加 Rust 契约测试、ROS 集成测试和 CI 执行。

## 验收标准

- 导航请求以带 tag 目标的 `move_to_pose` 到达 ROS。
- 取消任务会请求 ROS 取消，并进入已持久化的终态。
- 场景穿越会被拒绝，地图路线定义在编辑后保持正确。
- 配置 token 后，经过认证的 WebUI REST 和 WebSocket 流量正常工作。
- 诊断和体征事件通过 gateway 持久化或投递。
- CI 中的 gateway、WebUI 和 ROS 集成测试覆盖上述路径。
