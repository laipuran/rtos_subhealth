# Task 3 Report: Remove task state from Orchestration

## Status

Implemented and committed as Task 3.

## Changes

- Replaced `Orchestrator`'s local active-task and device-occupancy maps with an injected `Arc<dyn TaskRepository>`.
- Removed `ActiveTask`, `Orchestrator::task`, and all orchestration-owned task state.
- Routed task creation, feedback, and result transitions through the repository.
- Execution now receives the repository-created canonical task.
- If execution submission fails, the accepted repository record is transitioned to `Failed` before the execution error is returned.
- Feedback and result updates reject already-terminal records without mutating them; terminal records remain stored in the repository.
- Mapped repository errors into orchestration errors and added explicit terminal-task handling.

## Verification

- `rustfmt --edition 2021 --check ros2_ws/src/services/orchestration/src/lib.rs ros2_ws/src/services/orchestration/src/error.rs`: passed.
- Focused source invariant search for `HashMap`, `active`, `device_tasks`, `TaskView`, and `InMemoryTaskRepository` under orchestration: passed with no matches.
- `cargo fmt --all -- --check`: not passed because the modified orchestration files required formatting; direct rustfmt was then run and the focused rustfmt check passed.
- `cargo check -p orchestration`: blocked by the environment before compilation. Cargo attempted to load the existing ROS dependency at `/opt/ros/jazzy/share/action_msgs/rust/Cargo.toml`, which is absent.

## Concerns

- Gateway still targets the pre-Task-3 Orchestrator constructor and `ActiveTask` API by design; Gateway changes are explicitly deferred to Task 4.
- No tests were added, per repository instructions.
