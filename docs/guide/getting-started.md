# 快速开始

## 依赖

日常 Rust、ROS 和 WebUI 工具通过 Docker profile 提供。控制平面不要求主机
安装某一种设备 SDK。

## 格式、编译与构建

```bash
make check
make webui
```

## 运行服务端

```bash
make run server
```

服务端包含 Gateway、Orchestration、Execution 和 Sensor 控制平面，不需要传入
设备类型。

## 运行 endpoint

```bash
make run endpoint DEVICE_TYPE=<device-type>
```

endpoint 类型必须由已注册 adapter 提供。真实设备的 SDK、ROS 发行版和操作系统
属于 endpoint 自己的部署 profile，不进入控制平面。

## 选择镜像

```bash
make humble  # 构建 Ubuntu 22.04 + ROS Humble 镜像并进入容器
make jazzy   # 构建 Ubuntu 24.04 + ROS Jazzy 镜像并进入容器
```

## 文档入口

详见 [`docs/architecture/`](../architecture/)。旧 RFC 和迁移计划仅供历史参考，
不再作为当前实现依据。
