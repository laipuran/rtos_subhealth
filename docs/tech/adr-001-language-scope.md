# ADR-001：语言边界（Rust / C++ / Python）

- **状态：** 已接受
- **日期：** 2026-09-13

## 背景

系统正从纯 Python 的 ROS 2 Foxy 技术栈迁移到支持多种设备的机器人控制平台。
设备可能是没有 ROS 图的舵机机器人（例如 TonyPi 提供 Python SDK、HTTP
JSON-RPC 和离散动作组），也可能提供 C++/Python 厂商 SDK 或原生 `rclcpp` 驱动。

需要一套规则，使平台能够维护使用不同语言 SDK 的设备。

## 决策

**语言是语言无关 ROS 2 接口之后的实现细节，不使用跨语言进程内 FFI。**
控制平面核心与设备端执行边界分别决策：前者以 Rust 为主，后者可以使用厂商
SDK 所要求的语言。

| 层 | 语言 |
|---|---|
| WebUI | TypeScript |
| 网关、诊断 | Rust |
| 编排、规划、控制平面执行核心、安全策略 | Rust |
| ROS 节点（`rclrs`） | Rust |
| 设备端 exec/adapter | 默认 Rust；厂商 SDK 或硬实时要求时使用 C++/Python |
| 实时控制环、`ros2_control`、Nav2 组件 | C++ |
| 测试、仿真、数据和 CI 工具 | 优先 Rust；允许 Python |

强制规则：

1. 设备契约由 ROS 2 消息、action 和 service 构成，任何语言编写的 adapter 均可实现。
2. Adapter 作为独立进程运行，通过 DDS 通信；不共享头文件，不使用 FFI。
3. 一个可部署制品只使用一种语言。
4. 新的 ROS 核心代码默认使用 Rust。C++/Python adapter 必须在 ADR 中记录理由。
5. Python 不得实现请求主链路的核心逻辑（编排、规划、执行、安全、持久化）。
   Python 可以实现 adapter 和工具，且替换它时不能改变契约。
6. 语言和进程假设不得泄漏到消息字段中。
7. 控制电脑和机器人端可以运行不同的受支持 ROS 2 发行版，但每种部署组合都必须
   验证共享 rosidl 契约和 DDS 兼容性。
8. 机器人端负责本地停止、watchdog、超时和厂商 SDK 故障处理，不能完全依赖控制电脑。

## 后果

- 只有 Python 的厂商 SDK 会被限制在可替换的边缘 adapter 内。
- TonyPi 在 RPi4B（Ubuntu 22.04 + ROS 2 Humble）上以 Python `tonypi-exec`
  进程部署，不嵌入 Rust 控制平面。
- 核心逻辑集中在 Cargo workspace，可脱离机器人进行单元测试。
- 由于进程/ROS 边界只传递消息，因此可以支持跨发行版部署：控制电脑运行 Jazzy，
  设备保留厂商支持的 ROS。
- 必要时同时维护两个 ROS 客户端库（核心使用 `rclrs`，C++ adapter 使用
  `rclcpp`），二者通过 DDS 互操作。
- Rust ROS（`rclrs`）尚未达到 1.0，因此固定版本并纳入 CI 验证。
