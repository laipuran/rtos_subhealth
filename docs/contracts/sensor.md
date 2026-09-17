# Sensor Contract

`SensorProvider` 提供：

- Sensor descriptors；
- 按 `SensorId` 查询最新样本；
- 按 `SensorFilter` 订阅样本流。

Orchestration 和 Execution 都可以访问同一 Sensor provider。Sensor 只负责采集、
缓存、查询和发布，不负责任务决策或动作执行。
