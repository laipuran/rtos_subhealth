# Task 1 Implementation Report

## Status

Implemented the canonical contract simplification within the Task 1 file scope. No test code, compatibility layer, adapter type, Gateway code, Orchestration code, ExecutionRuntime code, or `ros_task_client` implementation was modified.

## Changed files

- `ros2_ws/src/services/platform/src/task.rs`
  - Reduced `Primitive` to `GoToTag`.
  - Removed `TaskTarget` and its default implementation.
  - Made `device_id` required.
  - Replaced target variants with the ordered `Vec<i32>` tag route.
  - Removed `required_capabilities` and `parameters`.
- `ros2_ws/src/services/platform/src/domain.rs`
  - Removed `capabilities` and `primitives` from `DeviceDescriptor`.
- `ros2_ws/src/services/platform/src/execution.rs`
  - Removed `ExecutionCommand`; `Executor::execute` now consumes canonical `Task` directly.
  - Removed cancellation, `ExecutionHandle`, and `ExecutionHandlePort`.
  - Removed the obsolete unsupported-command error.
  - Reduced `ExecutionResult` to the fields consumed for terminal outcome handling.
- `ros2_ws/src/services/platform/src/lib.rs`
  - Replaced glob re-exports with explicit exports of the remaining contract surface.
- `docs/contracts/task.md`
  - Documented the five canonical task fields and ordered AprilTag route.
- `docs/contracts/execution.md`
  - Documented direct canonical task consumption, reduced feedback/result contracts, and no cancellation.
- `docs/contracts/domain.md`
  - Removed capability and primitive-list fields from the documented device contract.

## Decisions

- Kept the Rust canonical field name `Task::id` exactly as specified by the Task 1 brief; it is the canonical task ID represented by `TaskId` and appears as `task_id` in execution feedback/results.
- Kept `Primitive::as_str` because current downstream Orchestration still references it, but reduced it to the sole authoritative value `go_to_tag`; all obsolete branches were removed.
- Used `Task` itself at the execution boundary rather than creating a replacement command type, avoiding a duplicate contract and any parameter/payload compatibility layer.
- Kept all `ExecutionFeedback` fields because Orchestration consumes task ID, progress, and phase.
- Reduced `ExecutionResult` to `task_id` and `state` because those are the fields currently consumed to expose and apply the terminal Exec outcome.
- Retained `Executor::descriptor` and `Executor::state` for this contract-only step; removing the generic runtime path belongs to later plan tasks.

## Commands and outputs

1. `cargo fmt --check`
   - Exit 0 after applying rustfmt's requested single-line re-export formatting.
2. `git diff --check`
   - Exit 0; no whitespace errors.
3. Contract stale-symbol searches in the modified Rust and contract documentation files.
   - No matches for removed task targets, capabilities, parameters, cancellation/handle APIs, unsupported command, or obsolete primitive variants.
4. `cargo check --workspace` from the repository root.
   - Exit 101 before compilation because `.cargo/config.toml` points to missing `/opt/ros/jazzy/share/action_msgs/rust/Cargo.toml`.
5. Isolated platform-package check outside the repository Cargo configuration:
   - Copied `ros2_ws/src/services/platform` to `/tmp/opencode/task-1-platform-check`.
   - Ran `cargo check --manifest-path Cargo.toml` from that directory.
   - Exit 0; `platform` compiled successfully.
6. Isolated workspace check outside the repository Cargo configuration:
   - Copied the workspace manifests and `ros2_ws` to `/tmp/opencode/task-1-workspace-check`.
   - Ran `cargo check --workspace` from that directory.
   - Exit 101 with expected stale downstream consumer errors only: Orchestration and ExecutionRuntime still reference removed `ExecutionCommand`, `ExecutionHandle`, optional device IDs, capability fields, `parameters`, and cancellation.

No tests were run or added, per repository and task instructions.

## Concerns

- The in-repository workspace check remains environment-blocked by the missing local ROS generated crate path.
- As expected for Task 1, the full workspace does not compile until later tasks update Orchestration and ExecutionRuntime consumers. Those files were intentionally left unchanged to preserve scope.
- Existing unrelated working-tree changes in `.devcontainer/devcontainer.json`, `.gitignore`, `Cargo.lock`, `Cargo.toml`, `AGENTS.md`, and the plan document were not modified or included in this task's commit.

## Review fix: remove canceled task state

Review identified `TaskState::Canceled` as stale cancellation contract surface. Removed that variant from `ros2_ws/src/services/platform/src/task.rs`. A scoped search confirmed there were no other cancellation symbols or now-obsolete imports/branches in the Task 1 Rust contract files or their three contract documents.

### Review fix verification

1. `cargo fmt --check`
   - Exit 0; formatting is clean.
2. Scoped cancellation search across `task.rs`, `domain.rs`, `execution.rs`, `lib.rs`, `task.md`, `execution.md`, and `domain.md`.
   - No stale cancellation symbols found.
3. Isolated platform-package check outside the repository Cargo configuration:
   - Refreshed `/tmp/opencode/task-1-platform-check` from `ros2_ws/src/services/platform`.
   - Ran `cargo check --manifest-path Cargo.toml` from the isolated directory.
   - Exit 0; `platform` compiled successfully.
