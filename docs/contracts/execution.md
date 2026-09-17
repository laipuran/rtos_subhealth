# Execution Contract

当前 `services/platform/src/execution.rs` 定义：

- `ExecutionCommand { task_id: TaskId, primitive: String, payload:
  serde_json::Value, deadline_ms: Option<u64> }`；
- `ExecutionFeedback { task_id: TaskId, progress: f32, phase: String }`；
- `ExecutionResult { task_id: TaskId, state: String, error_code:
  Option<String>, message: String }`；
- `ExecutionError::UnsupportedCommand(String)`、`ExecutionError::Busy`、
  `ExecutionError::Failed(String)`；
- `type ExecutionHandle = Arc<dyn ExecutionHandlePort>`。

Port 签名为：

```rust
pub trait Executor: Send + Sync {
    fn descriptor(&self) -> DeviceDescriptor;
    fn execute(&self, command: ExecutionCommand) -> Result<ExecutionHandle, ExecutionError>;
    fn cancel(&self, task_id: &TaskId) -> Result<(), ExecutionError>;
    fn state(&self) -> DeviceState;
}

pub trait ExecutionHandlePort: Send + Sync {
    fn result(&self) -> Option<ExecutionResult>;
}
```

具体设备只能通过 Endpoint Adapter 实现 `Executor`，控制平面不得导入其 SDK。
[ROS 2 RFC mock action](../guide/ros-mocks.md) 不是该 port 的 mapper；两套字段
之间仍需后续显式映射。
