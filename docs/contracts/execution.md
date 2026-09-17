# Execution Contract

Execution 接口包含：

- `ExecutionCommand`：任务 ID、primitive、参数和 deadline；
- `ExecutionFeedback`：任务 ID、进度和阶段；
- `ExecutionResult`：任务 ID、最终状态、错误码和消息；
- `Executor`：descriptor、execute、cancel、state。

具体设备只能通过 Endpoint Adapter 实现 `Executor`，控制平面不得导入其 SDK。
