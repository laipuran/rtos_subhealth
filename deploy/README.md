# 部署

部署由两个独立场景组成：

```bash
make run server
make run endpoint DEVICE_TYPE=<device-type>
```

server 只部署控制平面；endpoint 只部署 endpoint runtime、对应 adapter 和
backend SDK。endpoint 的 Ubuntu、ROS 和厂商 SDK 版本由其 profile 决定。

设备相关配置不得进入 Gateway、Orchestration 或 Execution 的构建产物。
