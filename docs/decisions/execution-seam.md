# 执行 seam 与设备注册表

## 决策

Orchestration 只依赖 `ExecutionPort`。具体 ROS action client 通过 `RosTaskClient` 和 YAML device registry 解析设备 ID 到 action name 的关系。

## 动机

任务编排不应知道 ROS action 名称、设备类型或厂商 SDK。设备配置变化应局部发生在 endpoint/client 配置中，而不是传播到上层业务模块。

## 当前范围

当前 registry 是静态 YAML 配置，Execution backend 只有 ROS task client。仓库没有正式的厂商 backend SDK 或通用 endpoint adapter 抽象。
