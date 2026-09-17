# ROS Subhealth

设备无关的机器人任务控制系统。WebUI 通过 Gateway 提交任务，由
Orchestration 按能力选择 endpoint，Execution 使用 Sensor 数据支持行动，
endpoint adapter 再连接具体设备或仿真 backend。

## 架构

```text
WebUI → Gateway → Orchestration → Execution → Endpoint Adapter → Backend SDK
                       ↘ Sensor Access ↗
```

控制平面不依赖任何机器类型。设备操作系统、ROS 发行版、厂商 SDK 和设备类型
只存在于 endpoint 配置及 adapter 边界。

## 目录

```text
docs/contracts/                    业务契约文档
ros2_ws/src/                       RFC 接口与开发验证 mock
services/gateway/                  HTTP/WS 网关
services/orchestration/            能力匹配与任务生命周期
services/execution/                设备无关执行运行时
services/sensor/                   Sensor registry 与 provider 访问
adapters/                          endpoint adapters（待实现，见 TODO.md）
webui/                             React/Vite 前端
docs/architecture/                 权威架构与接口文档
```

## 使用

```bash
make check
make webui
make run server
make image humble
make image jazzy
docker compose -f docker/dev/compose.yaml run --rm dev make build
docker compose -f docker/dev/compose.yaml run --rm dev \
  make run endpoint DEVICE_TYPE=mock-exec
docker compose -f docker/dev/compose.yaml run --rm dev \
  make run endpoint DEVICE_TYPE=mock-sensor
```

`server` 和 `endpoint` 是 `run` 的两种互斥模式。服务端不需要设备类型；运行
endpoint 时必须指定 `DEVICE_TYPE`。当前 `mock-exec` 和 `mock-sensor` 是 RFC
工作流开发 mock，并非 canonical `platform` ROS mapper。参数可通过
`ENDPOINT_ARGS` 传给 ROS 节点。

## 文档

- [架构总览](docs/architecture/overview.md)
- [分层边界](docs/architecture/layers.md)
- [接口契约](docs/architecture/contracts.md)
- [构建与运行](docs/architecture/build-and-run.md)
- [ROS 2 RFC mock 指南](docs/guide/ros-mocks.md)
- [Endpoint Adapter](docs/architecture/endpoint-adapters.md)
- [代码库对比分析](docs/architecture/migration-analysis.md)
- [完整重写设计](docs/superpowers/specs/2026-09-17-device-agnostic-rewrite-design.md)
