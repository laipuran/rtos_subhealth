# 系统架构总览

```text
WebUI → Gateway → Orchestration → Execution → Endpoint Adapter → Backend SDK → Device
                  ↘ Sensor Access ↗
```

Gateway、Orchestration、Execution 和 Sensor 是设备无关的控制平面。设备类型、
厂商协议、ROS 发行版和设备操作系统只允许出现在 endpoint adapter 与 backend
SDK 边界。

权威设计：[`../superpowers/specs/2026-09-17-device-agnostic-rewrite-design.md`](../superpowers/specs/2026-09-17-device-agnostic-rewrite-design.md)。
