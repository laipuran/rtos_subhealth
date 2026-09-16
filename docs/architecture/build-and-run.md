# 构建与运行

```bash
make image humble
make image jazzy
make webui
make webui-dev
make test
make check
```

镜像由 Ubuntu 与 ROS 发行版 profile 决定：Humble 使用 Ubuntu 22.04，Jazzy
使用 Ubuntu 24.04。endpoint 类型和 backend SDK 由 endpoint 配置决定，而不是
编译控制平面。
