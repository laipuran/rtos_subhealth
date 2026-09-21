# Task 4 Report: Validate devices before repository acceptance

## Completed changes

- Added `ExecutionPort::validate(&Task)` with an `ExecutionError` result, so
  orchestration depends only on the execution abstraction.
- `Execution` delegates validation to `RosTaskClient::validate` and converts
  its client error at the execution boundary.
- `Orchestrator::submit` now awaits validation before calling
  `TaskRepository::create_task`. A validation failure is returned through the
  existing `OrchestrationError::Execution` path and cannot create an accepted
  task record.
- The repository busy-device behavior is unchanged. Orchestration imports no
  registry or ROS types.
- No changes were needed in `ros_task_client/src/client.rs` or
  `orchestration/src/error.rs`: the client validation method and the existing
  execution-error variant already provide the required behavior.
- No tests were added, as required by repository instructions.

## Verification

- `cargo fmt --all -- --check` passed after formatting.
- `git diff --check` passed.
- `cargo check --workspace --all-targets` was attempted but blocked by the ROS
  environment before compilation.
- `cargo clippy --workspace --all-targets -- -D warnings` was attempted but
  blocked by the same ROS environment dependency before linting.

## ROS environment blocker

Both workspace commands exited with status 101 before compiling because Cargo
could not load the ROS Jazzy `action_msgs` package. The exact failure was:

```text
error: failed to load source for dependency `action_msgs`

Caused by:
  Unable to update /opt/ros/jazzy/share/action_msgs/rust

Caused by:
  failed to read `/opt/ros/jazzy/share/action_msgs/rust/Cargo.toml`

Caused by:
  No such file or directory (os error 2)
```

Run the workspace checks on a host with the ROS Jazzy Rust package installed
at that path.
