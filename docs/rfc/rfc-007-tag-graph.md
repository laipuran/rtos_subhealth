## RFC 007: Tag Graph 数据格式与可视化编辑

**状态：** 草案

**修订日期：** 2026-07-21

**修订历史：**
| 日期 | 变更 |
|---|---|
| 2026-07-21 | 初版 |
| 2026-07-21 | 文件路径改为 `maps/` 目录约定，预留多场景扩展 |

---

## 1. 摘要

当前 Planner 不存在，`GraphMissing` 错误码无人触发；WebUI 只能看文本状态字段，无法直观展示机器人在地图中的位置。本 RFC 定义 Tag Graph 的 JSON 文件格式作为规划与可视化的单一数据源，描述 Planner 基于该图计算最短路径的接口，以及 WebUI 可视化编辑与实时高亮的方案。

---

## 2. 目标与非目标

**目标：**
1. 定义 Tag Graph 的 JSON 文件格式，作为规划与可视化的单一数据源。
2. 实现 Planner 模块，加载 Tag Graph 并响应 `PlanPath.srv` 返回 `segments`。
3. Desc Layer 新增 `GET/PUT /api/v1/map` 端点，供 WebUI 读取和持久化编辑结果。
4. WebUI 支持 Tag Graph 可视化编辑（节点/边/路线增删改）与实时位置高亮。
5. Planner 和 WebUI 共用同一份 JSON，消除数据不一致。

**非目标：**
1. 不定义机器人底层控制与 SportClient 接口（见 RFC 008）。
2. 不实现地图自动建图（Tag 图由人工编辑）。
3. 不替代 desc_layer 的 HTTP 网关职责（见 RFC 005）。
4. 不定义 AprilTag 感知协议（见 RFC 001）。
5. 暂不支持多场景管理（多文件 + `?scene=` 参数已预留目录结构，逻辑待后续实现，见 5.1）。

---

## 3. 现状与痛点

1. `PlanPath.srv` 接口已定义但无 Server 响应，Planner 不存在，`GraphMissing` 错误码无人触发。
2. Tag 之间的拓扑关系无结构化定义，Planner 无法计算路径，exec_layer 无法获取 segments。
3. WebUI 无地图可视化，操作员只能从文本字段推断机器人位置，不直观。

---

## 4. 方案概览

定义一份 **`maps/default.json`** 文件作为当前场景的图数据，同时被 Planner 和 WebUI 消费。Planner 启动时加载该文件建邻接表，收到 `PlanPath.srv` 请求后运行 Dijkstra 返回最短路径 segments。WebUI 通过 `GET /api/v1/map` 获取该图，使用 React Flow 渲染为可拖拽编辑的节点图，编辑后通过 `PUT /api/v1/map` 持久化。任务执行过程中，WebSocket 推送的 `current_tag` / `next_tag` 叠加在图上实时高亮。

```
     ros2_ws/config/maps/
     └── default.json   ← 当前场景图（未来可加 room_b.json 等）
            ↙           ↘
     Planner           Desc Layer
 (Dijkstra, segs)    GET/PUT /map
                           │
                           ▼
                      WebUI Editor
                 (@xyflow/react 渲染)
                 + 实时高亮 current_tag
```

---

## 5. 关键接口

### 5.1 Tag Graph 存储格式（`tag_graph.json`）

```json
{
  "tags": {
    "1":  { "name": "bed_1",   "x": 0.0,  "y": 0.0 },
    "2":  { "name": "door",    "x": 3.0,  "y": 0.0 },
    "3":  { "name": "desk",    "x": 5.0,  "y": 2.0 },
    "42": { "name": "target",  "x": 8.0,  "y": 5.0 }
  },
  "edges": [
    { "from": 1,   "to": 2,   "cost": 3.0 },
    { "from": 2,   "to": 3,   "cost": 2.5 },
    { "from": 3,   "to": 42,  "cost": 4.0 },
    { "from": 1,   "to": 42,  "cost": 10.0 }
  ],
  "routes": {
    "patrol_ward":   [1, 2, 3, 42, 3, 2, 1],
    "supply_point": [1, 42]
  }
}
```

- **tags**: 节点集合，key 为 tag ID；`x`/`y` 为物理坐标或可视化布局坐标（未决问题 3）。
- **edges**: 有向边列表，`cost` 用于规划权重。
- **routes**: 预定义路线，key 为 `route_id`，value 为 tag ID 有序数组。
- 文件位置：`ros2_ws/config/maps/default.json`（可 ROS2 参数化）。
- 目录预留多场景扩展：未来可添加 `maps/room_b.json`、`maps/corridor.json` 等文件，通过 `?scene=` 参数切换。
- API 现阶段读写 `default.json`，未来加 `?scene=room_b` 后无需改现有逻辑。

### 5.2 ROS2 接口（已有，不新增）

| 类型 | 名称 | 消息 | 说明 |
|---|---|---|---|
| Service | `/plan_path` | `PlanPath.srv` | Planner 读取 JSON 后响应 segments |

### 5.3 Desc Layer 新增 HTTP 端点

| 方法 | 路径 | 说明 |
|---|---|---|
| `GET` | `/api/v1/map` | 返回当前 `tag_graph.json` |
| `PUT` | `/api/v1/map` | 接收 WebUI 编辑后的完整 JSON，写入文件（覆盖写入） |

**`PUT /api/v1/map` 冲突检测：**

后端收到 PUT 后，对比新旧图 diff，查 `task_store` 中所有 `state ∈ {accepted, running}` 的任务，检查其 `target_tags` 和 `route_id` 是否引用了被删除的 tag/edge/route：

- 无冲突 → 写入文件，返回 `200 { "status": "saved" }`
- 有冲突 → 返回 `409`，体含冲突详情：

```json
{
  "error": "cannot delete tag(s) [42, 43] used by active task T-001",
  "deleted_tags": [42, 43],
  "deleted_edges": [],
  "deleted_routes": ["patrol_ward"],
  "blocking_tasks": [
    {
      "goal_id": "T-001",
      "type": "go_to_tag",
      "target_tags": [42],
      "state": "running"
    }
  ]
}
```

不锁定全图——仅拒绝真实有冲突的删除操作。新增节点、修改位置/cost、删除未被引用的节点/边/路线均正常通过。

---

## 6. 数据/控制流

### 6.1 路径规划

```
WebUI 或决策层下发 go_to_tag(42)
  → Desc Layer -> /exec_task action
    → Exec Layer -> /plan_path service
      → Planner 加载 tag_graph.json
      → 建邻接表，Dijkstra 算 [start_tag → ... → 42]
      → 返回 segments[]
    → Exec Layer 消费 segments 并执行
```

### 6.2 可视化编辑

```
WebUI /editor 页面:
  → GET /api/v1/map              → 读取 tag_graph.json
  → GET /api/v1/tasks?filter=active → 获取 state=running|accepted 的任务
  → 合并两份数据:
      被活跃 task 引用的 tag → 灰色填充 + 移除删除按钮 + hover 提示 "占用中: T-001"
      被活跃 task 引用的边   → 灰色虚线
      未被引用               → 正常颜色 + 可选中编辑
  → 用户编辑（增删节点/边/路线、拖拽位置、编辑属性）
  → 点击 Save → PUT /api/v1/map → 若 409 则弹窗显示冲突详情与阻塞任务列表
```

### 6.3 图数据刷新机制

| 时机 | 行为 |
|---|---|
| Planner 启动时 | 读取 `maps/` 下 `map_scene` 参数指定的 JSON 文件建图 |
| `PUT /api/v1/map` 后 | Planner 定时或收到 invalidate 信号后重新加载 |
| WebUI 每次打开 editor | `GET /api/v1/map` 拉取最新数据 |
| WebUI 任务详情页 | `GET /api/v1/map` 渲染底图 + WS `current_tag` / `next_tag` 高亮 |

### 6.4 编辑冲突检测规则

后端 `PUT /api/v1/map` 的冲突判定按以下规则逐项检查：

| 编辑操作 | 检查范围 | 结果 |
|---|---|---|
| 删除 tag X | 所有活跃 task 的 `target_tags` 是否包含 X | 包含则 409 |
| 删除 edge from→to | 所有活跃 task 的 `target_tags` 构成的路径中是否包含该 edge | 包含则 409 |
| 删除 route R | 所有活跃 task 的 `route_id` 是否等于 R | 等于则 409 |
| 修改 tag 坐标/name | 不检查 | 200 |
| 修改 edge cost | 不检查 | 200 |
| 新增 tag/edge/route | 不检查 | 200 |

活跃 task 定义为 `state ∈ {accepted, running}` 的任务。

### 6.5 WebUI 高亮规则

| WS 数据 | 可视化效果 |
|---|---|
| `current_tag = 3, next_tag = 42` | tag 3 显示"完成"标记（绿），边 3→42 高亮（蓝），tag 42 闪烁/跳动 |
| `state = succeeded` | 终态 tag 全绿，所有已走过边变灰 |
| `error_code` 非空 | 当前 segment 边标红 |

---

## 7. 风险与替代

1. **替代方案：** 用 ROS2 `tf` + `nav_msgs/OccupancyGrid` 做全局地图。
   - **权衡：** 功能通用但引入沉重导航栈，Tag graph 的轻量拓扑更适合已知点位场景。
2. **替代方案：** 图数据存数据库（SQLite/PostgreSQL）而非 JSON 文件。
   - **权衡：** 并发安全、支持事务回滚，但引入运维依赖，单机场景 JSON 更简单。
3. **风险：** 多人同时编辑时 `PUT /api/v1/map` 覆盖写入会丢失冲突修改。初始版本按单人编辑设计。
4. **风险：** 多场景扩展后，同一活跃 task 的 `target_tags` 应限定在同一场景内，否则 Planner 需跨文件查图。当前版本不校验，由用户保证。

---

## 8. 验证计划

| 测试 | 方法 | 通过标准 |
|---|---|---|
| Graph 格式解析 | Planner 加载合法的 JSON | 建图成功，节点/边数量匹配 |
| 边界条件 | 空图 / 缺失起始点 / 目标点不存在 | 返回对应错误码 |
| 规划正确性 | 请求 go_to_tag(42) | 返回 segments 路径最短、无环 |
| Graph 编辑持久化 | WebUI 编辑 → PUT → GET 验证 | 修改内容持久化，格式仍合法 |
| 冲突拒绝 | 编辑正处于 running 的 task 的目标 tag → PUT | 返回 409 + 阻塞任务详情 |
| 正常编辑 | 编辑未被引用的 tag → PUT | 返回 200，文件持久化 |
| 前端灰色锁定 | 加载 editor，活跃 task 引用 tag 42 | tag 42 灰色显示，删除按钮不可用 |
| 冲突后释放 | 活跃 task 结束后，被引用的 tag 恢复可编辑 | 删除按钮恢复可用 |
| 实时高亮 | WebSocket 推送 `current_tag=3` | WebUI 图上对应节点高亮 |
| 多场景扩展（预留） | 在 `maps/` 目录新增 JSON 文件 | 不破坏现有功能，需 `?scene=` 参数才能访问 |

---

## 9. 未决问题

1. Planner 读取 `maps/default.json` 的路径如何传递给节点（ROS2 参数 `map_scene` / env / 固定路径）。
2. `PUT /api/v1/map` 的鉴权策略：是否与任务接口共用 Token。
3. WebUI 编辑器中的 tag 坐标 (`x`, `y`) 是否直接对应物理坐标系，还是仅用作可视化布局。若仅用作布局，规划时需另一份边权数据（目前由 `edge.cost` 承载）。
4. 图修改后 Planner 是否需要热重载，还是每次规划请求都重新读取文件。
