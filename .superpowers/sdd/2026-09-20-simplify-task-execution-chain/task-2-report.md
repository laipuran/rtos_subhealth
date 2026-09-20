# Task 2 Report: Simplify device selection and Orchestration

## Status

Implemented Task 2, including the review fix. Orchestration forwards the canonical `Task` directly, tracks occupancy by its concrete `device_id`, and returns the execution feedback/result session to the caller. The session is transport-neutral, contains no cancellation API, and remains compatible with `Arc<dyn ExecutionPort>`.

## Changed files

- `ros2_ws/src/services/platform/src/execution.rs`
  - Added the transport-neutral feedback stream, terminal result future, and `ExecutionSession` abstractions.
- `ros2_ws/src/services/platform/src/lib.rs`
  - Re-exported the execution session types.
- `ros2_ws/src/services/orchestration/src/lib.rs`
  - Changed `ExecutionPort::execute` and `Orchestrator::submit` to return `ExecutionSession` instead of discarding it.
  - Retains direct canonical `Task` forwarding, duplicate-task validation, busy-device validation, and active-task transitions.
  - Contains no registry, capability-selection, or cancellation path.
- `ros2_ws/src/services/orchestration/src/device.rs`
  - Deleted the unconsumed registry, device selection, and capability predicate in the original Task 2 change.
- `ros2_ws/src/services/orchestration/src/error.rs`
  - Removed the obsolete `NoDevice` selection error in the original Task 2 change.
- `ros2_ws/src/services/execution/src/lib.rs`
  - Removed the unused generic execution runtime and retained only the crate-level composition note in the original Task 2 change.
- `ros2_ws/src/services/execution/Cargo.toml`
  - Removed the now-unused `platform` and `thiserror` dependencies from the empty crate.
- `Cargo.lock`
  - Removed those execution package dependency edges.

## Exact session contract

```rust
pub type ExecutionFeedbackStream =
    Pin<Box<dyn Stream<Item = Result<ExecutionFeedback, ExecutionError>> + Send>>;
pub type ExecutionResultFuture =
    Pin<Box<dyn Future<Output = Result<ExecutionResult, ExecutionError>> + Send>>;

pub struct ExecutionSession {
    pub feedback: ExecutionFeedbackStream,
    pub result: ExecutionResultFuture,
}

pub trait ExecutionPort: Send + Sync {
    fn execute(&self, task: Task) -> Result<ExecutionSession, String>;
}

pub fn submit(&mut self, task: Task) -> Result<ExecutionSession, OrchestrationError>;
```

The boxed stream and future make the session concrete and the port object-safe, preserving `Arc<dyn ExecutionPort>` usage. The abstractions depend only on platform domain types and `futures-core`; they do not depend on ROS or `ros_task_client`.

## Verification

- `cargo fmt --manifest-path /home/duckran/codes/rtos_subhealth/Cargo.toml --package platform --package orchestration -- --check` (from `/tmp/opencode`)
  - Exit 0; no output.
- `cargo check --manifest-path /home/duckran/codes/rtos_subhealth/Cargo.toml --package platform --package orchestration --package execution` (from `/tmp/opencode`)
  - Exit 0; all three Task 2 crates checked successfully.
- `cargo clippy --manifest-path /home/duckran/codes/rtos_subhealth/Cargo.toml --package platform --package orchestration --package execution --lib -- -D warnings` (from `/tmp/opencode`)
  - Exit 0; no warnings.
- `cargo tree --manifest-path /home/duckran/codes/rtos_subhealth/ros2_ws/src/services/execution/Cargo.toml --depth 1` (from `/tmp/opencode`)
  - Exit 0; output contains only the `execution` package, confirming the empty crate has no dependencies.
- `cargo check --manifest-path /home/duckran/codes/rtos_subhealth/Cargo.toml --workspace` (from `/tmp/opencode`)
  - Exit 101 after successfully checking the Task 2 crates.
  - Failed in out-of-scope Gateway code with the same five stale Task 1 contract errors: removed `TaskTarget`, removed `TaskState::Canceled`, optional `device_id`, and removed `required_capabilities`/`parameters` fields.
- Focused source searches
  - No ROS or `ros_task_client` references exist in platform or orchestration.
  - No service-side `ExecutionPort::execute(Task) -> Result<(), String>` or `submit(Task) -> Result<DeviceId, _>` signature remains.
- `git diff --check` on the changed Task 2 source/manifests
  - Exit 0; no output.

## Concerns / follow-up

- Full workspace verification remains blocked by the out-of-scope Gateway stale-contract errors listed above.
- Runtime composition must consume both returned session fields and translate its transport-specific errors into `ExecutionError`; that work is intentionally outside Task 2.
- Cargo commands are run from `/tmp/opencode` with an absolute manifest path to avoid the repository-local generated ROS Cargo patch configuration.
