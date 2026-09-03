# 操作手册

## 1. 环境要求

| 组件 | 版本 |
| --- | --- |
| Ubuntu | 20.04 |
| ROS2 | Foxy |
| Python | 3.8+ |
| Node.js | 18+ |
| pnpm | 9+ |
| gcc/g++ | 9.4+（编译 ROS2 接口包需要） |

### Python 依赖

```bash
# desc_layer 依赖
pip install flask flask-sock

# exec_layer 依赖（FSM 状态机）
pip install transitions
```

## 2. ROS2 环境配置

### 2.1 安装 ROS2 Foxy

参考 [ROS2 Foxy 官方安装指南](https://docs.ros.org/en/foxy/Installation/Ubuntu-Install-Debians.html)。

### 2.2 一键 source（推荐）

项目提供 `setup.sh`，自动设置 ROS2 环境与 DDS：

```bash
cd ros2_ws
source setup.sh
```

脚本内容包含：
- `source /opt/ros/foxy/setup.bash`
- `source install/setup.bash`（workspace 编译产物）
- `source ~/unitree_ros2/cyclonedds_ws/install/setup.bash`（CycloneDDS RMW，可选）
- `export RMW_IMPLEMENTATION=rmw_cyclonedds_cpp`
- `export ROS_DOMAIN_ID=1`
- `export CYCLONEDDS_URI=...<NetworkInterfaceAddress>lo</NetworkInterfaceAddress>...`（绑定 loopback 网卡）
- `export RCUTILS_CONSOLE_OUTPUT_FORMAT=...`

### 2.3 手动配置（不依赖 setup.sh）

```bash
source /opt/ros/foxy/setup.bash
source ~/unitree_ros2/cyclonedds_ws/install/setup.bash
export RMW_IMPLEMENTATION=rmw_cyclonedds_cpp
export ROS_DOMAIN_ID=1
export CYCLONEDDS_URI='<CycloneDDS><Domain><General><NetworkInterfaceAddress>lo</NetworkInterfaceAddress></General></Domain></CycloneDDS>'
unset ROS_LOCALHOST_ONLY
source ros2_ws/install/setup.bash
```

### 2.4 验证安装

```bash
ros2 --version
ros2 topic list   # 应无报错
```

## 3. ROS2 工作空间编译

### 3.1 首次编译

```bash
cd ros2_ws
colcon build --symlink-install
source install/setup.bash
```

`--symlink-install` 使 Python 节点可热重载，修改源码后无需重新编译。

**注意：** 编译后运行前务必 `source setup.sh`（见 2.2 节）配置 DDS 环境。

### 3.2 只编译部分包

```bash
colcon build --packages-select exec_layer desc_layer
```

### 3.3 每次编译后

必须 source 才能发现新包或更新：

```bash
source install/setup.bash
```

### 3.4 ROS2 接口包说明

接口定义均在 `ros_interfaces` 和 `apriltag_interfaces` 中（.action / .srv / .msg）。修改接口后需重新编译这两个包：

```bash
colcon build --packages-select ros_interfaces apriltag_interfaces
source install/setup.bash
```

## 4. ROS2 包启动

### 4.1 一键启动（推荐）

所有后端统一通过 `run.sh` 启动：

```bash
cd ros2_ws

# Mock 模式：纯软件模拟，测 WebUI 链路
./run.sh mock

# Sim 模式：MuJoCo 仿真，看 GO2 走路
./run.sh sim

# Real 模式：控制真机 GO2
./run.sh real
```

内部自动执行 `source setup.sh` + `ros2 launch desc_layer run.launch.py`。

### 4.2 逐节点启动（调试用）

```bash
source ros2_ws/setup.sh

# Planner
ros2 run exec_layer planner_node --ros-args -p maps_dir:=config/maps

# Exec Layer
ros2 run exec_layer exec_layer_node --ros-args -p robot_backend:=mock

# Desc Layer
ros2 run desc_layer desc_layer_node

# Mock Exec Layer（mock 模式专用）
ros2 run mock_exec_layer mock_exec_layer_node
```

### 4.4 相机测试发布器

```bash
ros2 launch camera_test_publisher camera_test_publisher.launch.py
```

### 4.5 Mock 执行层（mock_exec_layer，无硬件也能全链路测试）

```bash
source ros2_ws/setup.sh

# 启动 mock 执行层（默认 action 名 mock_exec_task）
ros2 run mock_exec_layer mock_exec_layer_node
```

日志预期：`[INFO] [mock_exec_layer_node]: Mock Exec Layer ready on action: mock_exec_task`

**模拟行为：**

| 任务类型 | 模拟表现 |
| --- | --- |
| `go_to_tag` | 3 秒完成，每秒发一次 feedback（progress 0.3→0.6→1.0） |
| `patrol_route` | 每个 tag 耗时 1 秒，逐一发 feedback |
| `hold` | 立即返回 succeeded |
| 失败模拟 | `constraints.max_speed_mps < 0` 时返回 failed |
| 取消 | 收到 cancel 请求后立即返回 canceled |

### 4.6 Planner（路径规划）

Planner 集成在 `exec_layer` 包内部，通过 launch 文件自动启动：

```bash
source ros2_ws/setup.sh
ros2 launch desc_layer mock.launch.py  # 自动包含 planner
```

或单独启动：

```bash
source ros2_ws/setup.sh
ros2 run exec_layer planner_node --ros-args -p maps_dir:=config/maps
```

日志预期：`[INFO] [planner_node]: Loaded graph: 4 tags, 4 edges, 2 routes`

### 4.7 查看所有可用节点

```bash
ros2 node list
ros2 topic list
ros2 action list
```

## 5. WebUI 启动

### 5.1 安装依赖

```bash
cd webui
pnpm install
```

### 5.2 开发模式启动

```bash
pnpm dev
```

默认监听 `http://localhost:5173`，Vite 自动热更新。

### 5.3 生产构建

```bash
pnpm build      # 输出到 webui/dist/
pnpm preview    # 本地预览生产构建
```

## 6. commitlint 提交规范

项目使用 husky + commitlint 自动拦截不合规的 commit message。

### 6.1 格式

```
type(scope): subject
```

**type 取值（@commitlint/config-conventional）：**

| type | 说明 |
| --- | --- |
| feat | 新功能 |
| fix | 修复 |
| docs | 文档 |
| style | 代码格式 |
| refactor | 重构 |
| test | 测试 |
| chore | 杂项（构建/CI） |
| rfc | RFC 文档专用 |

**项目内 scope 示例：** `docs`, `exec`, `desc`, `perception`, `visual`, `rfc`

### 6.2 提交示例

```
feat(docs): 更新RFC 003和RFC 006文档
fix(exec): 修复planner超时未处理的问题
rfc(docs): 添加决策层动作任务流协议
docs(guide): 添加操作手册
```

### 6.3 规则

- header 最大 120 字符
- subject 大小写不限制，允许中文
- scope 必须小写

### 6.4 绕过 commitlint（紧急情况）

```bash
git commit --no-verify -m "wip: 临时提交"
```

## 7. 全链路启动

所有模式统一入口：

### 7.1 Mock 模式（纯软件，测 WebUI）

```bash
cd ros2_ws
./run.sh mock
```

另开一个终端启动 WebUI：

```bash
cd webui && pnpm dev
```

浏览器打开 `http://localhost:5173` → 新建 go_to_tag 任务 → 实时看到 accepted → running → succeeded 状态变化。

### 7.2 Sim 模式（MuJoCo 仿真）

```bash
cd ros2_ws
./run.sh sim
```

MuJoCo 窗口显示 GO2 在 tag 地图上行走。

### 7.3 Real 模式（真机 GO2）

```bash
cd ros2_ws
./run.sh real
```

## 8. 常见问题

### Q: 启动后任务一直卡在 `accepted` 状态

检查 exec_layer 或 mock_exec_layer 是否已启动。desc_layer 需要连接到对应的 action server。

```bash
ros2 action list  # 查看可用 action server
```

若 action server 不存在，desc_layer 的 `wait_for_server` 会在 5 秒后超时，任务变为 `failed`。

### Q: `colcon build` 报错找不到 package

确保先 `source /opt/ros/foxy/setup.bash`，并且 `ros2_ws/src/` 下存在对应的 package.xml。

### Q: `ros2 run` 提示找不到 package

```bash
source ros2_ws/install/setup.bash
# 验证包是否存在
ros2 pkg list | grep <包名>
```

### Q: WebUI 连接不上 desc_layer

确认 desc_layer 已启动（日志无报错），且 WebUI 的 Vite proxy 配置正确。默认 Vite proxy 将 `/api` 转发到 `http://localhost:5000`。

### Q: desc_layer 返回 `401 UNAUTHORIZED`

desc_layer 启动了 `api_token` 参数，所有请求需要在 HTTP header 中添加：

```bash
curl -H "X-API-Key: my-secret-token" http://localhost:5000/api/v1/tasks
```

WebUI 开发环境下，可在浏览器控制台设置 `localStorage.setItem("api_token", "my-secret-token")` 后刷新（需配套修改前端 `api/tasks.ts` 中的 header 注入逻辑）。

### Q: API 返回 `{"error": {"code": "...", "message": "..."}}` 格式

这是统一错误格式。常见的 `code`：
- `INVALID_JSON` — 请求体不是合法 JSON
- `INVALID_GOAL` — task goal 字段缺失或 type 非法
- `NOT_FOUND` — 资源不存在
- `INVALID_STATE` — 任务已在终态无法取消
- `CONFLICT` — 编辑地图时被活跃任务引用
- `UNAUTHORIZED` — 缺少或错误的 API Token

### Q: 任务重启后还在吗？

在。`desc_layer` 使用 SQLite 持久化任务记录，重启后任务历史不丢失。数据库文件默认在 `ros2_ws/config/tasks.db`。

### Q: 如何追踪某个请求的全链路？

每个 HTTP 响应包含 `X-Trace-Id` header 和响应体中的 `trace_id` 字段。可在请求时通过 `X-Trace-Id: my-custom-id` 指定，便于日志关联。

### Q: commitlint 报错

```bash
# 查看 husky hook 是否激活
ls -la .husky/commit-msg
# 手动触发校验
npx --no -- commitlint --edit .git/COMMIT_EDITMSG
```
