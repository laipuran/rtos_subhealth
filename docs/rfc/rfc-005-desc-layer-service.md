## RFC 005: Desc Layer 服务与对外接口

**状态：** 草案

**修订日期：** 2026-07-21

**修订历史：**
| 日期 | 变更 |
|---|---|
| 2026-05-22 | 初版 |
| 2026-07-21 | 补充 API 规范：错误格式、分页、ETag、trace_id、鉴权格式 |

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
4. 不定义 WebUI 前端实现细节（Toast、布局等）。

---

## 3. 现状与痛点
WebUI 与外部系统处在不同网段时，无法直接使用 ROS2 DDS。没有 desc_layer 会导致任务入口分散，调用方式不统一。

---

## 4. 方案概览
desc_layer 作为唯一入口接收 HTTP/WS 请求，转成 `task` action，并把反馈与结果聚合后推送给 WebUI。
数据流：WebUI/外部系统 -> desc_layer -> `task` action -> 执行层 -> desc_layer -> WebUI。

---

## 5. 关键接口

### 5.1 HTTP API

| 方法 | 路径 | 说明 |
|---|---|---|
| `POST` | `/api/v1/tasks` | 下发任务（请求体对应 RFC 003 Goal） |
| `GET` | `/api/v1/tasks` | 列表查询，支持分页 |
| `GET` | `/api/v1/tasks/{goal_id}` | 查询单任务状态与结果 |
| `POST` | `/api/v1/tasks/{goal_id}/cancel` | 取消任务 |

### 5.2 WebSocket

- `WS /api/v1/events`：推送任务状态变更与结果
- 消息体：`{ goal_id, event: "feedback"|"result", trace_id, state?, progress?, current_tag?, next_tag?, error_code?, message?, final_state? }`

### 5.3 统一规范

以下规范适用于所有 HTTP API。

#### 5.3.1 通用响应格式

成功响应：

```json
{
  "task_id": "...",
  "trace_id": "a1b2c3d4e5f6g7h8",
  ...
}
```

错误响应：

```json
{
  "error": {
    "code": "NOT_FOUND",
    "message": "human readable description",
    "details": {}
  },
  "trace_id": "a1b2c3d4e5f6g7h8"
}
```

错误码枚举：

| code | HTTP 状态 | 典型场景 |
|---|---|---|
| `INVALID_JSON` | 400 | 请求体不是合法 JSON |
| `INVALID_GOAL` | 400 | type 缺失或非法 |
| `INVALID_PARAM` | 400 | 查询参数格式错误 |
| `INVALID_STATE` | 400 | 任务已在终态无法取消 |
| `NOT_FOUND` | 404 | 任务或地图不存在 |
| `CONFLICT` | 409 | 地图编辑被活跃任务阻止 |
| `UNAUTHORIZED` | 401 | 缺少或错误的 API Token |

#### 5.3.2 分页

`GET /api/v1/tasks` 支持分页参数：

| 参数 | 默认值 | 说明 |
|---|---|---|
| `offset` | `0` | 起始偏移量 |
| `limit` | `50` | 最大返回条数（上限 200） |

响应包含 `total`、`offset`、`limit` 字段：

```json
{
  "tasks": [...],
  "total": 128,
  "offset": 0,
  "limit": 50,
  "trace_id": "a1b2c3d4e5f6g7h8"
}
```

#### 5.3.3 ETag / 缓存

`GET /api/v1/map` 支持 ETag：

| Header | 说明 |
|---|---|
| 响应 `ETag` | 文件内容的 MD5 值 |
| 请求 `If-None-Match` | 客户端缓存的值，匹配则返回 `304 Not Modified`，不返回 body |

ETag 仅在文件内容变更时变化，适用于地图数据的浏览器缓存。

#### 5.3.4 Request Tracing

每个请求分配一个 `trace_id`，贯穿 HTTP 请求的完整链路：

| 传递方式 | 说明 |
|---|---|
| 客户端指定 | 请求头 `X-Trace-Id: custom-id`，后端沿用此值 |
| 服务端生成 | 客户端未指定时，服务端生成 16 位 hex 值 |
| 响应返回 | `X-Trace-Id` 响应头 + 响应体 `trace_id` 字段 |

`trace_id` 用于日志关联与调试，后继可扩展为跨 ROS2 action 传递。

#### 5.3.5 鉴权

API 访问鉴权采用 `X-API-Key` Header：

```
X-API-Key: my-secret-token
```

- Token 为空字符串时，鉴权关闭（开发环境默认）。
- Token 非空时，所有 HTTP 请求必须携带匹配的 `X-API-Key`，否则返回 `401 UNAUTHORIZED`。
- WebSocket 连接时，首个消息应为 Token 字符串；否则服务端关闭连接。
- Token 通过 desc_layer 的 ROS2 参数 `api_token` 配置。

### 5.4 状态字段约定

- 使用 RFC 003 的字段；反馈中包含 `finished_stage` 与 `total_stage`。

---

## 6. 数据/控制流
1. desc_layer 接收 HTTP/WS 请求并校验字段，生成 `trace_id`。
2. 鉴权开启时校验 `X-API-Key`；不匹配则返回 401。
3. desc_layer 作为 action client 下发 `task`。
4. desc_layer 聚合 feedback/result 并推送 WebUI（WebSocket 消息携带 `trace_id`）。
5. 任务记录通过 SQLite 持久化，重启不丢失。

---

## 7. 风险与替代
1. **替代方案：** WebUI 直接使用 ROS2 DDS。
   - **权衡：** 延迟低，但跨网段部署困难。
2. **替代方案：** 使用 gRPC 网关代替 HTTP/WS。
   - **权衡：** 接口更强，但实现与运维成本更高。
3. **风险：** desc_layer 单点不可用会影响任务入口。

---

## 8. 验证计划

| 测试 | 通过标准 |
|---|---|
| 任务下发/取消 | WebUI 可通过 HTTP/WS 正常下发与取消任务 |
| Action 转发 | desc_layer 能正确转发 `task` action 并接收反馈 |
| 实时推送 | WebUI 能收到 `finished_stage/total_stage` 与终态结果 |
| 错误格式 | 不合法请求返回统一 `{error: {code, message}}` 格式 |
| 分页 | `?offset=&limit=` 参数生效，响应含 `total/offset/limit` |
| ETag | `GET /api/v1/map` 返回 `ETag` 头，`If-None-Match` 匹配时返回 304 |
| trace_id | 所有响应含 `X-Trace-Id` 头和 `trace_id` 字段 |
| 鉴权 | `api_token` 开启后，无 `X-API-Key` 的请求返回 401 |

---

## 9. 未决问题

1. Token 与 API Key 的轮换与更新机制（当前仅在启动时通过 ROS2 参数配置，不支持热更新）。
2. 任务记录保留时长与清理策略（当前 SQLite 持久化无自动清理）。
