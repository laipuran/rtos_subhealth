# 设计规格：向 Rust-first / ROS 2 Jazzy 的工业级重构

**状态：** 已批准（用户口头授权，明确要求不再提问、持续推进至最佳实践）

**日期：** 2026-09-13

**分支：** `feat.duckran.migration`

**相关研究：** `docs/tech/tech-platform-migration-research.md`（全部结论均有一手来源）

---

## 1. 背景与目标

当前仓库是医疗巡检机器人（Unitree GO2）的多 ROS 包 monorepo：

- 100% Python（63 个 `.py`，0 个 Rust/C++），ROS 2 **Foxy** / Ubuntu 20.04。
- 后端为 Flask 开发服务器跑在 ROS 节点内的 daemon 线程。
- 无 CI、无 per-package 打包、无自动化部署、存在死代码与文档合并冲突。

**目标：** 建立一套可维护、可复现、可安全部署到机器人端点的工业级架构，并回答"是否迁移 Rust"与"如何把单个包部署到 Foxy/目标端点"。

**成功判据：**

1. 单一语言工具链（Rust workspace）覆盖全部节点与后端服务，除隔离的硬件边界外无 Python 生产代码。
2. 每个 ROS 包 / 服务可独立构建为 `.deb`，通过签名 apt 仓库安装到目标端点；节点由 systemd 托管。
3. 实时控制路径（`rt/lowcmd` 500 Hz）有明确的隔离边界与 spike 验证，不因语言迁移引入安全性回退。
4. CI 在固定 Jazzy 镜像内完成 build + test + 出包。

---

## 2. 决策驱动因素

| 驱动 | 结论 |
|---|---|
| GO2 兼容性 | 约束在 **CycloneDDS + Unitree IDL/topics + domain + 网卡**，与 ROS 发行版无关。Unitree 官方 `unitree_ros2` 同时支持 Foxy 与 Humble；`unitree_sdk2_python` 不依赖 rclpy。GO2 不绑定 Foxy。 |
| Foxy 生命周期 | Foxy 自 2023-06 EOL；继续使用是工业风险。 |
| 目标端点 | **外部控制 PC（x86_64）**，非车载 Jetson。 |
| 目标发行版 | **ROS 2 Jazzy / Ubuntu 24.04 LTS**（支持到 2029-05），`rmw_cyclonedds_cpp`。Humble（到 2027）仅作 Phase 0 回退。 |
| Rust 可行性 | `rclrs 0.7` 支持 Humble→Rolling，含 actions/services/params/timers/async；Unitree 的 `unitree_ros2` ROS 消息可由 `rosidl_rust` 生成 Rust 绑定；与 Unitree 同用 CycloneDDS，互操作风险最低。 |
| 最大风险 | 500 Hz `rt/lowcmd` 控制环与精确 DDS 类型匹配。必须隔离并 spike 验证。 |

**Go/No-Go Gate（Phase 0）：** 在 Jazzy 上让 Rust 控制节点与 GO2（或 MuJoCo 桥）完成 `rt/lowcmd` 发布 / `rt/sportmodestate` 订阅的端到端验证。失败则仅 `robot_driver` crate 回退 C++（`unitree_sdk2`）薄封装，其余架构不变。

---

## 3. 语言决策：Rust-first，硬件边界隔离

用户倾向全 Rust 并授权由工程判断决定。结论：**Rust-first**。

- 控制电脑上的 ROS 节点、网关、诊断、感知和模拟器默认使用 Rust
  （`rclrs` + `cargo`）。
- **设备端执行是独立边界。** 当厂商 SDK 只有 Python 时，允许在机器人端运行
  独立 Python adapter；它不属于 Rust 核心请求链路，也不使用进程内 FFI。
- Unitree 等设备的硬件边界仍可定义 `RobotBackend` trait，实现放在对应 adapter
  crate 或独立进程中。
  - 默认实现：Rust 通过 `rclrs` 发布/订阅 `unitree_ros2` 提供的 ROS 消息（`LowCmd`/`SportModeState`/`unitree_api`）。
  - 回退实现（仅当 Phase 0 spike 失败）：一个极薄的 C++ `unitree_sdk2` shim，通过进程内 FFI 或独立 DDS 进程暴露同样接口。
- 这样既满足"全 Rust 的可维护性收益"，又不把 500 Hz 安全关键路径押在未验证的绑定上。

**Rust 相对 Python 在本项目的收益：**

1. 无 GIL，明确线程/异步模型，可满足确定性控制周期。
2. 编译期类型与接口校验，消除 `exec_layer.robot` 这类 import 副作用。
3. 单一 `cargo` workspace 依赖解析，可复现构建；二进制部署，无虚拟环境漂移。
4. 内存安全 + 所有权，降低医疗机器人并发/生命周期类缺陷。

---

## 4. 目标仓库结构

```
rtos_subhealth/
├── Cargo.toml                     # [workspace]，统一 edition/lints/deps
├── rust-toolchain.toml            # 固定工具链
├── ros2_ws/                       # colcon 工作区（rosidl 接口 + rclrs 节点）
│   └── src/
│       └── ros/                  # 新版 ROS 包统一根目录（legacy 同级保留待退役）
│           ├── interfaces/        # rosidl 接口，唯一契约源
│           │   ├── task_interfaces/      # ExecTask.action / PlanPath.srv / Segment / Constraints
│           │   ├── perception_interfaces/# AprilTagDetection(s)
│           │   └── diagnosis_interfaces/    # PhysioSample / VitalsStream / DiagnosisResult
│           ├── robot_driver/      # rclrs 节点：RobotBackend trait + Unitree/Mock/Sim 实现
│           ├── control/           # rclrs 节点：ExecTask action server + planner + FSM
│           ├── perception/        # rclrs 节点：AprilTag (opencv) + camera
│           └── sim/               # MuJoCo 桥 + mock publisher
├── services/
│   ├── gateway/                   # Rust/axum：HTTP+WS+SQLite+托管 WebUI
│   └── diagnosis/                 # Rust：聚合 + RAG + LLM
├── webui/                         # React/Vite（产物由 gateway 托管）
├── deploy/
│   ├── debian/                    # cargo-deb / bloom 配置
│   ├── systemd/                   # 各节点 unit
│   ├── apt/                       # aptly 仓库与发布脚本
│   ├── rauc/                      # A/B OTA 配置
│   └── config/                    # /etc 默认配置
├── docker/                        # dev + CI（Jazzy 固定镜像）
├── .github/workflows/             # CI
└── docs/
```

命名原则：包名不加全局前缀；接口按**业务域**拆分，不再按实现层（`orchestration/`）拆分。

---

## 5. 组件设计

### 5.1 interfaces（rosidl）

沿用现有消息语义，重命名并归位：

- `task_interfaces`：`ExecTask.action`（由现有 `Task`/`ExecTask` 合并）、`PlanPath.srv`、`Segment.msg`、`Constraints.msg`。
- `perception_interfaces`：`AprilTagDetection.msg`、`AprilTagDetections.msg`。
- `diagnosis_interfaces`：`PhysioSample.msg`、`VitalsStream.msg`、`DiagnosisResult.msg`、`DiagnosisMetric.msg`。

**契约稳定规则：** 接口包独立版本号；破坏性变更必须新增字段/新建 action，并由 CI 的接口兼容性检查（字段增删检测）拦截。

### 5.2 robot_driver（crate）

职责：把"任务级速度/姿态指令"翻译为机器人控制，屏蔽仿真/真机差异。

```
trait RobotBackend: Send + Sync {
    fn stand_up(&self) -> Result<()>;
    fn move_velocity(&self, vx: f64, vy: f64, vyaw: f64) -> Result<()>;
    fn damp(&self) -> Result<()>;
    fn recovery_stand(&self) -> Result<()>;
    fn state(&self) -> RobotState;   // 位姿 / IMU / 错误码
}
```

实现：`MockBackend`、`SimBackend`（MuJoCo `mujoco-rs`）、`UnitreeBackend`（rclrs + `unitree_ros2` 消息）。
`UnitreeBackend` 内部把 500 Hz 控制环放在**独立 OS 线程**，以固定周期写 `rt/lowcmd`；不与其他 rcl 回调共享执行器。

### 5.3 control（crate）

- `ExecTask` action server：解析 goal → 调 planner → 驱动 `RobotBackend` → 发 feedback/result。
- 用**显式状态机枚举**替代 Python `transitions` 的动态元类；状态与转移在编译期穷尽匹配。
- 规划请求改为异步：`Planner` 作为 crate 内的纯函数式模块（可脱离 ROS 单测），ROS service 仅做适配层。
- 消除嵌套 spin：action 执行使用 rclrs async / 独立执行器。

**状态机（显式枚举）：**

```
Idle -> Planning -> Moving -> Approaching -> Aligning -> Stabilizing -> Done
                \-> Holding -> Done            (hold)
所有状态 -> Canceled / Failed
```

### 5.4 perception（crate）

- AprilTag：`opencv` crate（AprilTag/ArUco 检测）+ `solvePnP`，或 `apriltag` crate（注意其 2023 后无发布）。
- 相机：`nokhwa`（V4L2）。
- 检测在**独立线程/执行器**，10 Hz 定时发布 `AprilTagDetections`，无目标也发空数组（RFC-001）。

### 5.5 gateway（service，取代 desc_layer Flask）

- `axum` + `tokio`，`tokio-tungstenite` WebSocket，`rusqlite` 持久化。
- 同时是 rclrs 节点（`ROS` build）或通过 `ros_bridge` trait 与 ROS 交互：
  - 生产实现：rclrs action client + 订阅诊断/体征 topic。
  - 测试实现：内存 mock，使 `cargo test` 无需 ROS。
- **HTTP/WS 契约保持 RFC-005 不变**（路径、错误格式、分页、ETag、trace_id、`X-API-Key`），确保 WebUI 零改动。
- 新增：托管 `webui/dist`（`tower-http::ServeDir`），去掉"WebUI 无服务器"的空白；WS 合并为单连接多路复用或后端统一 fan-out（前端后续跟进，不阻塞后端）。
- 单进程内 axum（tokio）与 rclrs 执行器分线程 + `crossbeam`/`tokio::sync` channel 通信，取代 Flask daemon 线程。

### 5.6 diagnosis（service）

- 窗口聚合 → 规则异常检测 → RAG 检索 → LLM（`async-openai`，可配 `base_url`）→ JSON schema 校验 → 发布 `DiagnosisResult`。
- 纯 Rust 核心逻辑（聚合/阈值/检索打分/JSON 校验）可离线单测；LLM/embedding 客户端有 trait + mock。
- 保留 RFC-009 语义：每传感器独立 topic、多源 `source_ids`、confidence 阈值、异常优先。

### 5.7 webui

- 保持 React 19 + Vite + TS。**仅**做必要适配：由 gateway 同源托管后，去掉 Vite dev proxy 依赖；WS 连接由三个合并为一个（优化项）。
- `pnpm build` 产物进入 gateway 的静态目录；不再需要单独的 preview 服务器。

---

## 6. 接口与契约

### 6.1 ROS 话题 / QoS（显式声明，不再默认）

| Topic | 类型 | QoS | 说明 |
|---|---|---|---|
| `/physio/{data_src}` | `PhysioSample` | KeepLast(10) Reliable Volatile | 1 Hz |
| `/diagnosis/results` | `DiagnosisResult` | Reliable | 事件驱动 |
| `/diagnosis/monitor` | `VitalsStream` | BestEffort(1) | 1 Hz 可视化 |
| `/perception/apriltag_detections` | `AprilTagDetections` | BestEffort(5) | ≥10 Hz |
| `rt/lowcmd` | `unitree_go/LowCmd` | Reliable, 独立线程 | 固定周期 |
| `rt/sportmodestate` | `unitree_go/SportModeState` | BestEffort | 状态反馈 |
| `/exec_task` | `ExecTask.action` | 默认 | 单一任务入口 |

**DDS：** 固定 `rmw_cyclonedds_cpp`、`ROS_DOMAIN_ID=1`（控制子网）、`CYCLONEDDS_URI` 绑定指定网卡；机器人链路与业务链路按需分域。

### 6.2 HTTP/WS（不变，见 RFC-005/006/009）

路径与语义保持，作为 WebUI 与外部系统的稳定契约。新增 `GET /` 与静态资源由 gateway 提供。

---

## 7. 部署模型（工业最佳实践）

| 维度 | 方案 |
|---|---|
| 构建 | CI 在固定 `ros:jazzy-ros-base` 镜像内 `colcon build`（rosidl + rclrs）+ `cargo build --release` |
| 打包 | ROS/接口包用 `bloom-generate rosdebian` + `fakeroot debian/rules binary`；Rust 服务用 `cargo-deb` |
| 分发 | 签名 apt 仓库（aptly 不可变快照 + pin），每 release 一个快照，可确定性安装与回滚 |
| 配置 | 默认 `/usr/lib/<pkg>` + 覆盖 `/etc/ros/*.yaml` |
| 密钥 | systemd credentials（可 TPM2 加密），不再用 `.env` 落在仓库旁 |
| 运行 | 每节点一个 systemd unit，`Restart=on-failure`；DDS/RMW 在 unit 环境固定 |
| 日志 | journald（`journalctl -u <svc>`），不写裸日志文件 |
| OTA | RAUC A/B 镜像更新（签名、原子、可回滚）；可变状态（地图、SQLite、日志）放独立数据分区 |
| 安全 | 控制网启用 SROS2 Enforce（需验证 Unitree 节点兼容性）；传感器 BestEffort，控制 Reliable+Deadline |

**目标端点安装（最终形态）：**

```
# 一次性加入仓库
curl -fsSL https://apt.example.com/ros-key.gpg | sudo tee /etc/apt/keyrings/ros.gpg
echo "deb [signed-by=...] https://apt.example.com/ros jazzy main" | sudo tee /etc/apt/sources.list.d/ros.list
sudo apt update && sudo apt install ros-control gateway diagnosis ros-webui
sudo systemctl enable --now ros-control gateway
```

---

## 8. CI

- `lint`：`cargo fmt --check`、`cargo clippy -D warnings`、`colcon test`、`pnpm lint/build`。
- `test`：Rust unit + integration（无 ROS 的部分纯 cargo；ROS 部分在 Jazzy 容器内 `cargo test`）。
- `build`：Jazzy 容器内 `colcon build` + `cargo build --release`。
- `package`：出 `.deb`，签名并发布到测试 apt 快照。
- `interface-compat`：接口字段增删检测。
- 所有 job 用固定 digest 的基础镜像保证可复现。

---

## 9. 测试策略

| 层 | 方式 |
|---|---|
| 纯逻辑（planner/FSM/聚合/RAG 打分/JSON 校验/HTTP handlers） | `cargo test`，无 ROS 依赖；这是大部分代码 |
| ROS 适配层 | 在 Jazzy 容器内跑，使用 `rclrs` 的 in-process 测试或 `ros2 topic` 断言 |
| 契约（HTTP/WS） | Rust 集成测试打真实 axum app，校验 RFC-005 响应/错误/分页/ETag/trace_id/鉴权 |
| 端到端（mock） | `docker compose` 启 gateway + mock 节点，跑脚本下发任务并断言 WS 事件 |
| 硬件（GO2） | Phase 0 spike 脚本 + 现场回归清单 |

---

## 10. 分阶段计划与 Gate

| 阶段 | 内容 | Gate |
|---|---|---|
| **P0** | GO2 DDS 兼容性 spike（Jazzy + rclrs 发布 `rt/lowcmd` / 订阅 `rt/sportmodestate`；MuJoCo 桥验证） | 端到端收发成功，或触发 `robot_driver` C++ 回退 |
| **P1** | 地基清理：修文档冲突、删死代码/跟踪的 db、依赖收敛、`.gitignore`、CI 骨架、repo 重组、接口重命名 | CI lint/test 绿 |
| **P2** | gateway + diagnosis Rust 化，WebUI 同源托管；HTTP 契约测试全绿 | `docker compose` mock 全链路 |
| **P3** | control/perception Rust 化，打通真实 `_drive_segment` + AprilTag 闭环 | 仿真中移动到 tag 前停止 |
| **P4** | 部署：deb、apt、systemd、配置/密钥、RAUC | 目标端点 `apt install` 后服务自启 |

**P0 未通过前，P3 的 `robot_driver` 硬件实现不得合并。**

---

## 11. 旧 → 新 映射

| 旧 | 新 |
|---|---|
| `orchestration/desc_layer`（Flask） | `services/gateway`（Rust/axum） |
| `orchestration/diagnosis_layer`（Python） | `services/diagnosis`（Rust） |
| `orchestration/exec_layer`（Python FSM + planner） | `ros2_ws/src/control`（Rust） |
| `exec_layer/robot/*`（死代码 + unitree import） | `ros2_ws/src/robot_driver`（trait + 实现） |
| `orchestration/mock_exec_layer` | `ros2_ws/src/sim` 内的 mock backend |
| `orchestration/physio_mock_publisher` | `ros2_ws/src/sim` 内的 mock publisher |
| `perception/apriltag_perception` | `ros2_ws/src/perception` |
| `perception/camera_test_publisher` | `ros2_ws/src/perception` 的 test 工具 |
| `ros_interfaces` / `apriltag_interfaces` / `physio_interfaces` | `robot/interfaces/*` |
| `run.sh` / `setup.sh` / `Makefile` | `deploy/` + systemd + docker compose |

---

## 12. 风险与缓解

| 风险 | 缓解 |
|---|---|
| rclrs 构建/API 不稳定（pre-1.0） | 固定 `rclrs` 版本与 rust-toolchain；CI 固定镜像；接口局限在适配层 |
| GO2 精确 DDS 类型匹配失败 | Phase 0 spike 前置；失败仅回退 `robot_driver` |
| Rust 控制环实时性不足 | 独立线程 + 固定周期 + `SCHED_FIFO`（如允许）；热点路径无分配 |
| 全量重写周期长 | 分阶段 tracer bullet，每阶段独立可验证；P2 gateway 可先行替换 |
| WebUI 受影响 | 保持 HTTP/WS 契约不变，前端改动最小化 |
| 目标端点无 24.04 | P0 同时验证 Humble 回退路径 |

---

## 13. 非目标

- 不实现真实医疗设备协议、不做医疗决策/报警。
- 不做多机器人/多用户权限体系。
- 不在本设计内实现地图自动建图。
- 不追求硬实时（ROS/内核补丁）；目标是确定性软实时 + 可观测。

---

## 14. 待验证假设（P0 解决）

1. `rclrs` + `unitree_ros2` 的 ROS 消息在 Jazzy 上可 codegen 并驱动 GO2。
2. CycloneDDS 版本/域/网卡在 Jazzy 侧与 GO2 0.10.2 对齐无冲突。
3. MuJoCo 仿真桥（`mujoco-rs`）能替代现有 Python bridge。
4. RAUC 在目标控制 PC 的引导链（GRUB/UEFI）上可用。
