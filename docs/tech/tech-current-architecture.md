# 当前架构（权威文档）

> 本文是最新描述。若 RFC 或 `docs/guide/` 中的旧文档与本文冲突，以本文和 ADR
> 为准。旧 Python/ROS 2 Foxy 实现保存在 `legacy/`，仅供参考。

## 平台

- ROS 2 **Jazzy** on Ubuntu 24.04 (external control PC), `rmw_cyclonedds_cpp`.
- Robot endpoints keep their vendor-supported OS/ROS and communicate through the
  shared rosidl contract over DDS. The first endpoint is TonyPi on RPi4B,
  Ubuntu 22.04 + ROS 2 Humble.
- Docker describes the control-PC development environment. It does not imply
  that ROS or the vendor SDK is absent from a robot endpoint.

## 语言边界

默认使用 Rust；实时控制或厂商 C++ SDK adapter 使用 C++；Python 仅用于边缘
adapter 和工具。详见 `docs/tech/adr-001-language-scope.md`。

## 组件

| 层 | crate / 包 | 说明 |
|---|---|---|
| HTTP/WS 网关 | `services/gateway` + `ros2_ws/src/robot/gateway_bridge` | axum；`gateway_bridge` 连接 ROS |
| 诊断核心 | `services/diagnosis` | 聚合、异常检测、提示词和 JSON schema |
| 诊断节点 | `ros2_ws/src/robot/diagnosis_node` | 订阅 `/physio/*`，发布 `/diagnosis/*` |
| 生理数据模拟器 | `ros2_ws/src/robot/physio_mock` | 6 个传感器，1 Hz |
| 编排核心 | `services/orchestrator-core` | 注册表、能力检查、任务生命周期 |
| 编排节点 | `ros2_ws/src/robot/orchestrator` | 发现设备并路由到 adapter |
| 设备 adapter SDK | `services/device-sdk` | Rust backend 契约以及 mock/差速底盘实现 |
| 通用 adapter 节点 | `ros2_ws/src/robot/adapter` | 适用于兼容 backend 的 Rust adapter |
| TonyPi exec | 机器人端包 | RPi4B/Humble 上的 Python `rclpy` adapter 和 TonyPi Python SDK |
| 世界模型/规划器 | `services/world-model` | graph + Dijkstra |
| 安全 | `services/safety` | 限制、watchdog、急停 |
| 感知 | `services/perception-sim` + `ros2_ws/src/robot/perception_sim`（仿真）/ `perception_camera`（真实 tag36h11） | 几何仿真和真实相机 |
| 编排目标解析 | orchestrator 内使用 `services/world-model` | tag/waypoint → pose |
| WebUI | `webui/` | 由 gateway 托管 |

## 接口（rosidl，不添加统一前缀）

`task_interfaces` (`DeviceTask`, `ExecTask`, `PlanPath`), `device_interfaces`
(`DeviceDescriptor`, `DeviceState`, `TaskTarget`, primitives),
`perception_interfaces`, `diagnosis_interfaces`.

## 数据流

```
WebUI --HTTP/WS--> gateway(gateway_bridge)
                        |  DeviceTask action
                        v
                    orchestrator ---- /<device_id>/device_task ----> endpoint exec
                        ^                                            |
                        +------------- feedback / result ------------+
physio_mock --/physio/*--> diagnosis_node --/diagnosis/*--> gateway
perception_sim / perception_camera --/perception/apriltag_detections--> (control)
orchestrator resolves tag/waypoint targets to poses via the world model (MAP_PATH)
```

## 构建与运行

所有日常任务通过根目录 `Makefile` 执行（`make help`）。

```bash
# pure Rust services + tests (host or container)
make test

# ROS interfaces + nodes (dev container; auto-enters it from the host)
make ros

# run orchestrator + adapter (dev container)
make run-stack
```

Full walkthrough: `docs/guide/getting-started.md`.

## 部署

systemd units under `deploy/systemd/`: `orchestrator.service`,
`adapter@.service` (generic per-device adapter), `tonypi-exec.service`, and
`gateway_bridge.service`. Config in
`deploy/config/`, secrets via systemd credentials. Per-package `.deb` + signed
apt + RAUC A/B are scaffolded in `deploy/`.
