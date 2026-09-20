# Execution Contract

当前 `ros2_ws/src/services/platform/src/execution.rs` 定义：

- `ExecutionFeedback { task_id: TaskId, progress: f32, phase: String }`；
- `ExecutionResult { task_id: TaskId, state: String }`；
- `ExecutionError::Busy`、`ExecutionError::Failed(String)`。

执行边界直接消费 canonical `Task`，不创建第二套 command/payload 参数契约：

```rust
pub trait Executor: Send + Sync {
    fn descriptor(&self) -> DeviceDescriptor;
    fn execute(&self, task: Task) -> Result<(), ExecutionError>;
    fn state(&self) -> DeviceState;
}
```

首版执行契约不支持取消。反馈用于更新任务进度和阶段，结果仅暴露任务的终态。
