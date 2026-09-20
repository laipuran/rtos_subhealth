# Task 2 Report: Simplify device selection and Orchestration

## Status

Implemented the Task 2 scope. Orchestration now forwards the canonical `Task` directly to a minimal execution submission boundary, tracks occupancy by the task's concrete `device_id`, and contains no registry, capability-selection, or cancellation path. The unused generic execution runtime was removed.

## Changed files

- `ros2_ws/src/services/orchestration/src/lib.rs`
  - Changed `ExecutionPort::execute` to accept the canonical `Task` directly.
  - Removed cancellation from `ExecutionPort` and `Orchestrator`.
  - Removed registry and sensor ownership and their accessors.
  - Kept duplicate-task and busy-device validation.
  - Removed the redundant `ActiveTask.device_id`; occupancy cleanup uses `task.device_id`.
  - Removed the canceled terminal-state branch.
- `ros2_ws/src/services/orchestration/src/device.rs`
  - Deleted the unconsumed registry, device selection, and capability predicate.
- `ros2_ws/src/services/orchestration/src/error.rs`
  - Removed the obsolete `NoDevice` selection error.
- `ros2_ws/src/services/execution/src/lib.rs`
  - Removed `ExecutionRuntime`, `ExecutionHandle` storage, runtime state/error types, cancellation, polling, and generic executor/sensor ownership.
  - Left the crate ready for the concrete ROS transport composition in Task 5.

## Verification

- `cargo fmt --manifest-path /home/duckran/codes/rtos_subhealth/Cargo.toml --all -- --check`
  - Passed with no output.
- `cargo check --manifest-path /home/duckran/codes/rtos_subhealth/Cargo.toml --package orchestration --package execution` (run from `/tmp/opencode` to avoid the repository-local generated ROS Cargo patch configuration)
  - Passed; both changed crates and `platform` compiled successfully.
- `cargo clippy --manifest-path /home/duckran/codes/rtos_subhealth/Cargo.toml --package orchestration --package execution --lib -- -D warnings` (run from `/tmp/opencode`)
  - Passed with no warnings.
- `cargo check --manifest-path /home/duckran/codes/rtos_subhealth/Cargo.toml --workspace` (run from `/tmp/opencode`)
  - Reached and successfully checked `platform`, `execution`, `sensor`, and `orchestration`.
  - Failed in the out-of-scope Gateway Task 3 code with five pre-existing stale-contract errors: removed `TaskTarget`, removed `TaskState::Canceled`, optional `device_id`, and removed `required_capabilities`/`parameters` fields.
- Focused source searches under `orchestration/src` and `execution/src`
  - No references remain to capability selection, parameters, cancellation, `ExecutionHandle`, `ExecutionRuntime`, or the removed command/runtime types.
- `git diff --check`
  - Passed with no output.

## Concerns / follow-up

- The current ROS client exposes an asynchronous feedback/result session, while Task 2 is prohibited from importing ROS types or `ros_task_client` into Orchestration. The smallest coherent boundary is therefore `ExecutionPort::execute(Task) -> Result<(), String>`, with feedback and terminal results continuing to enter through `Orchestrator::feedback` and `Orchestrator::complete`. Task 5 must implement the concrete Execution-owned ROS session and route those events back without introducing an adapter layer.
- Full workspace verification remains blocked until Task 3 updates Gateway to the Task 1 canonical contract.
- Running Cargo from the repository root additionally loads `.cargo/config.toml`, whose generated ROS patch paths point to unavailable `/opt/ros/jazzy` packages in this environment. Verification was run from `/tmp/opencode` with the repository manifest path so Cargo checked the same workspace sources without that generated local configuration.
