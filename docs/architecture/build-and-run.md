# 构建与运行

```bash
make humble
make jazzy
make webui
make webui-dev
make check
```

镜像由 Ubuntu 与 ROS 发行版 profile 决定：Humble 使用 Ubuntu 22.04，Jazzy
使用 Ubuntu 24.04。endpoint 类型和 backend SDK 由 endpoint 配置决定，而不是
编译控制平面。

ROS package 必须在镜像内构建。`make jazzy`（或 `make humble`）会先构建对应
镜像，然后直接进入交互容器：

```bash
make jazzy
# 进入容器后
make build
```

容器内 `make build` 以 `ros2_ws/src` 为唯一 colcon base path 构建 ROS
workspace（包括 `ros2_ws/src/services/` 下的五个 Rust service packages），并使用
`colcon --merge-install --symlink-install`；不会再用独立的 Cargo 命令重复构建
这些 packages。主机上的 `make build` 仍使用 repository Cargo workspace。
默认
`ROS_BUILD_ROOT=/ws/$ROS_DISTRO`，因此 log、build intermediate 和 merged
install 分别位于 `/ws/$ROS_DISTRO/log`、
`/ws/$ROS_DISTRO/build/merged-symlink` 和 `/ws/$ROS_DISTRO/install`。
Humble 与 Jazzy 共用 named volume 但不会共用这些构建状态；也可以设置
`ROS_BUILD_ROOT` 覆盖默认位置。

## 运行

服务端和设备 endpoint 是两种互斥场景：

```bash
make run server
make run endpoint DEVICE_TYPE=mock-exec \
  ENDPOINT_ARGS="-p step_delay_s:=0.2"
make run endpoint DEVICE_TYPE=mock-sensor \
  ENDPOINT_ARGS="-p scenario:=anomaly -p rate_hz:=5.0 -p random_seed:=0"
```

服务端不需要设备类型；endpoint 必须指定已注册的 `DEVICE_TYPE`。当前两个
profile 是开发验证 mock，不是正式 endpoint runtime，也没有接入 Orchestration。
完整 action、取消和 topic 命令见 [ROS 2 RFC mock 指南](../guide/ros-mocks.md)。
目标架构中 Sensor 将同时向 Orchestration 和 Execution 提供数据访问。
