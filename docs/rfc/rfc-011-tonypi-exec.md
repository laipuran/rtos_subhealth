# RFC 011：TonyPi 机器人端 exec

**状态：** 草案

## 1. 摘要

定义 TonyPi 在 RPi4B 上的设备端执行服务。TonyPi 使用 Ubuntu 22.04、
ROS 2 Humble 和厂商 Python SDK；该服务通过平台 ROS2 接口接收设备任务，
将通用 primitive 映射为 TonyPi 动作组或 SDK 调用，并向控制电脑发布状态和
任务结果。

## 2. 部署边界

```text
控制电脑（Ubuntu 24.04 / Jazzy）
  WebUI → gateway → orchestrator
                         │ DDS
机器人端（RPi4B / Ubuntu 22.04 / Humble）
  tonypi-exec → TonyPi Python SDK → hardware
```

TonyPi 端不运行 WebUI、gateway、orchestrator 或全局 planner。控制电脑和
RPi4B 之间只通过 `device_interfaces` / `task_interfaces` 的 ROS2 契约通信。
Jazzy/Humble 的 DDS 互操作性需要单独实机验证。

## 3. 语言和进程

- 使用 Python `rclpy` 编写 `tonypi-exec`。
- TonyPi Python SDK 仅在该进程内调用。
- 不让 Rust 控制平面嵌入 Python，也不使用跨语言进程内 FFI。
- 通过 systemd 启动、重启和记录日志。

## 4. 能力声明

TonyPi 默认声明：

```text
capabilities: supports_action_groups
primitives: execute_primitive, hold, stop
sensors: 仅声明实际可用的 battery / imu / camera 等
```

除非实现了真实的反馈和控制闭环，不声明 `set_velocity`、`move_to_pose`、
odometry 或 pose feedback。动作组通过 `params_json` 传递，例如：

```json
{"action": "wave", "repeat": 2}
```

## 5. 本地安全

`tonypi-exec` 必须在本地处理：

- stop/hold
- 任务 deadline 和动作超时
- 控制电脑断联后的 watchdog 行为
- SDK 异常和设备故障
- 任务取消后的安全停止

这些行为不能依赖控制电脑继续发送消息。

## 6. 验证范围

1. RPi4B 能发现并发布 `DeviceDescriptor`。
2. 控制电脑能通过 DDS 发送 `DeviceTask`。
3. 动作组能正确执行并返回 feedback/result。
4. cancel、deadline、SDK 异常和 DDS 断联都进入安全状态。
5. TonyPi 不会被错误地下发 `move_to_pose` 或 `set_velocity`。
