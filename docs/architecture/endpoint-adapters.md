# Endpoint Adapter

Endpoint adapter 是设备相关代码的唯一入口。它实现 `BackendSdk`，向
`endpoint-runtime` 提供 descriptor、state 和 execute。

新增设备时只允许新增：

1. backend SDK wrapper；
2. endpoint adapter 注册；
3. endpoint 配置和部署 profile；
4. adapter 自己的构建与运行验证。

不得修改 Gateway、Orchestration 或 Execution 来加入设备分支。
