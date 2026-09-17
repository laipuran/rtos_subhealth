# 现有代码库对比分析

## 已移除的问题

- 旧编排核心、设备 SDK、world model、safety 和 ROS 节点把
  设备分发、ROS transport 与业务逻辑混在一起；已从新的 Cargo workspace 移除。
- 旧 adapter 通过 `DEVICE_TYPE` 直接构造具体后端；新实现
  由 endpoint registry 配置工厂，不让核心服务了解设备名称。
- 旧接口同时存在 Rust model、ROS generated model、RFC 和脚本；新契约集中在
  `contracts/`，transport 负责显式映射。

## 新边界

- Sensor 可同时被 Orchestration 和 Execution 读取/订阅。
- Execution 只依赖 `Executor` 和 `SensorProvider`。
- Orchestration 只依赖 `ExecutionPort`、`SensorProvider` 和能力描述。
- Gateway 只产生 canonical task，不直接调用 endpoint。

## 验证方式

`cargo build --workspace` 和 `cargo clippy --workspace` 验证当前保留代码；后续
ROS adapter 只验证 transport mapper，不把未实现的设备逻辑放入控制平面。
