# 系统架构总览

```text
WebUI → Gateway → Orchestration → Execution → Endpoint Adapter → Backend SDK → Device
                  ↘ Sensor Access ↗
```

Gateway、Orchestration、Execution 和 Sensor 是设备无关的控制平面。设备类型、
厂商协议、ROS 发行版和设备操作系统只允许出现在 endpoint adapter 与 backend
SDK 边界。

Repository 是任务状态的唯一真相源，保存所有已接受任务及其状态、进度和阶段。
运行时组合根只构造一个 Repository 实例，并将同一句柄注入 Gateway 和
Orchestration；Execution 只负责执行，不依赖任务持久化。Gateway 在 Repository
状态转换后发布事件，事件仅是瞬时通知，不是第二份任务状态。

权威设计：[`../superpowers/specs/2026-09-17-device-agnostic-rewrite-design.md`](../superpowers/specs/2026-09-17-device-agnostic-rewrite-design.md)。
