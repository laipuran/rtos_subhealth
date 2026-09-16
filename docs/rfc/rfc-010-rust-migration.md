# RFC 010：Rust-first 与 ROS 2 Jazzy 迁移

**状态：** 已实施，持续收敛

**范围：** 本 RFC 记录当前分支相对主分支的 Rust 迁移结果、保留的兼容边界
以及尚未完成的实机端工作。它不重新定义 ROS2 消息；接口字段以
`ros2_ws/src/robot/interfaces/` 为唯一来源。

## 1. 背景

主分支是以 Python/ROS 2 Foxy 为主的单体 ROS 工作区。当前分支将控制平面
迁移为 Rust，并将 WebUI、网关、任务编排、规划、诊断、感知、模拟和设备契约
拆分为可测试、可部署的 crate 或 ROS2 节点。

迁移的目标不是把每个厂商 SDK 都改写成 Rust，而是将语言差异隔离在设备端
exec/adapter 边界之后。

## 2. 架构决策

### 2.1 控制电脑与机器人端分离

控制电脑运行 Ubuntu 24.04 + ROS 2 Jazzy，承担：

- Rust gateway 和 WebUI 托管；
- orchestrator 和任务生命周期管理；
- world model 与路径规划；
- 诊断、持久化和全局安全策略。

机器人端只承担设备相关执行。TonyPi 的首个目标环境为 RPi4B、Ubuntu 22.04
+ ROS 2 Humble，运行独立的 Python `tonypi-exec`，调用 TonyPi Python SDK。

### 2.2 语言边界

- 控制平面默认使用 Rust；
- ROS2 核心节点默认使用 `rclrs`；
- C++ 用于实时控制或厂商 C++ SDK；
- Python 仅用于厂商 Python SDK 的设备端 adapter、工具和测试；
- 不使用跨语言进程内 FFI；不同语言通过 ROS2/DDS 或明确的进程协议通信。

### 2.3 设备能力契约

设备通过 `DeviceDescriptor` 声明能力、primitive、限制、坐标系和传感器，
通过 `DeviceState` 发布运行状态。orchestrator 根据能力路由任务，而不是根据
设备类型写条件分支。

通用 primitive 包括 `move_to_pose`、`set_velocity`、`hold` 和 `stop`；设备
专有动作使用 `execute_primitive` 与 `params_json`。TonyPi 默认只声明动作组、
`hold` 和 `stop`，不虚构速度、位姿或里程计能力。

## 3. 当前分支的主要变更

### 3.1 服务与核心库

新增或迁移了以下 Rust 组件：

| 组件 | 作用 |
|---|---|
| `services/gateway` | axum HTTP/WS、任务 API、地图 API、SQLite、WebUI 托管 |
| `services/orchestrator-core` | 设备注册、能力检查、任务生命周期和路由 |
| `services/device-sdk` | Rust 设备 backend 契约、mock 和差速底盘实现 |
| `services/world-model` | 地图加载、tag graph 和 Dijkstra 规划 |
| `services/safety` | 速度限制、watchdog 和急停策略 |
| `services/diagnosis` | 生理数据聚合、异常检测、RAG/LLM 结构化结果 |
| `services/perception-sim` | 几何 AprilTag 仿真与检测模型 |

### 3.2 ROS2 节点与接口

新增 `robot/interfaces` 接口包：

- `task_interfaces`：`DeviceTask`、兼容入口 `ExecTask`、`PlanPath`、约束和路径段；
- `device_interfaces`：设备描述、设备状态、目标和设备 primitive；
- `perception_interfaces`：AprilTag 检测；
- `diagnosis_interfaces`：体征、诊断指标和诊断结果。

新增或迁移的 `rclrs` 节点包括 gateway bridge、orchestrator、generic adapter、
diagnosis、physio mock、perception camera 和 perception simulation。

### 3.3 外部 API

保留 WebUI 使用的 HTTP/WS 契约：

```text
POST /api/v1/tasks
GET  /api/v1/tasks
GET  /api/v1/tasks/{goal_id}
POST /api/v1/tasks/{goal_id}/cancel
WS   /api/v1/events
```

同时加入统一错误格式、分页、ETag、`trace_id` 和可选 `X-API-Key` 鉴权。

### 3.4 部署和工程化

- 使用固定 Docker 开发环境；
- 使用 Cargo workspace 和锁定的 Rust toolchain；
- ROS 节点和接口支持 `.deb` 打包；
- 使用 systemd 管理服务；
- 使用 journald 记录日志；
- 预留签名 apt 仓库和 RAUC A/B 更新；
- CI 执行 Rust 测试、ROS 构建、接口检查和 WebUI 构建。

## 4. 与主分支的兼容处理

### 保留

- WebUI 的任务、诊断和地图 API 语义；
- 任务状态、feedback/result 和错误码语义；
- AprilTag、tag graph 和生理诊断的 ROS payload 语义；
- 旧接口的兼容性入口 `ExecTask`，直到所有调用方迁移完成。

### 替代

- `desc_layer` → Rust `gateway`；
- Python diagnosis layer → Rust diagnosis service；
- Python exec layer/FSM → orchestrator + endpoint exec；
- 按硬件写死的 `RobotInterface` → capability-based device contract；
- 旧的统一 GO2 控制实现 → 设备端独立 adapter。

### 明确不兼容的旧假设

- 所有机器人都有速度控制；
- 所有机器人都有 odometry 或 pose；
- 所有设备端服务都使用 Rust；
- 所有节点都运行在同一台机器或同一个 ROS 发行版；
- TonyPi 可以复用 GO2 的 `stand_up`、`damp`、`SportClient` 或 `LowCmd` 模型。

## 5. 未完成事项

1. 在 RPi4B/Humble 上实现并安装 Python `tonypi-exec`；
2. 验证 Jazzy 控制电脑与 Humble TonyPi 端的 DDS/rosidl 互操作；
3. 为 TonyPi 建立本地 stop、watchdog、deadline 和 SDK 故障测试；
4. 将当前控制电脑上的 TonyPi JSON-RPC backend 与真实 Python SDK 路径区分；
5. 根据实机反馈决定是否扩展状态字段，但优先不修改现有 ROS payload。

## 6. 关联文档

- `docs/tech/adr-001-language-scope.md`
- `docs/tech/adr-002-device-contract.md`
- `docs/tech/tech-current-architecture.md`
- `docs/rfc/rfc-011-tonypi-exec.md`
