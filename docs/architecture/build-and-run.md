# 构建与运行

```bash
make image humble
make image jazzy
make webui
make webui-dev
make check
make check
```

镜像由 Ubuntu 与 ROS 发行版 profile 决定：Humble 使用 Ubuntu 22.04，Jazzy
使用 Ubuntu 24.04。endpoint 类型和 backend SDK 由 endpoint 配置决定，而不是
编译控制平面。

## 运行

服务端和设备 endpoint 是两种互斥场景：

```bash
make run server
make run endpoint DEVICE_TYPE=<device-type>
```

服务端不需要设备类型；endpoint 必须指定已注册的 `DEVICE_TYPE`。Sensor 同时
向 Orchestration 和 Execution 提供数据访问。
