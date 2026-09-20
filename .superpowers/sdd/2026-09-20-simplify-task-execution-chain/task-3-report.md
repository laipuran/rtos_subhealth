# Task 3 Report: Gateway orchestration integration

## Status

Implemented. Gateway now receives the reduced task input, submits the canonical domain task to the injected Orchestrator before recording a query projection, and consumes the returned execution session to apply feedback and terminal results to both Orchestration and the Gateway projection.

## Changes

- `CreateTask` now has only `device_id`, `primitive`, `target`, and `deadline_ms`. Its conversion creates the five-field domain `Task`, adding only the generated `id`.
- Replaced `GatewayState` with Axum's actual `AppState`. One `Arc<AppStateInner>` owns the task-history projection, task/event sequences, broadcast sender, and `Mutex<Orchestrator>`; no redundant inner `Arc`s, registry, or execution client were added.
- `main.rs` creates concrete `Execution`, injects it into `Orchestrator`, then builds `AppState`. The device/action endpoint can be selected with `GATEWAY_EXECUTION_DEVICE_ID` and `GATEWAY_EXECUTION_ACTION_NAME`; the existing mock endpoint remains the default. The retained `Arc<Execution>` is shut down after the Axum server exits.
- `create_task` waits for `Orchestrator::submit`. A submission error returns HTTP 409 for a busy/duplicate task and 502 for execution submission failure, with no Gateway projection inserted.
- After a successful submission, Gateway inserts the accepted projection, emits the existing `SystemEvent::TaskStateChanged`, and spawns one session consumer. Feedback calls `Orchestrator::feedback`, updates progress/phase/state in the projection, and emits a state event only when state changes. A successful terminal result calls `Orchestrator::complete` and updates/emits its terminal state.
- Removed the cancellation handler and `/api/v1/tasks/{id}/cancel` route.
- Added Gateway's `execution`, `orchestration`, and stream utility dependencies, matching ROS package metadata. Exported the existing `OrchestrationError` because Gateway must map the public `submit` failure.
- Kept the required root workspace exclusion removal and lockfile update: concrete Gateway composition reaches `execution`, which path-depends on `ros_task_client` and therefore requires it to participate in the root workspace.

## HTTP shape inspection

The accepted creation JSON has exactly these four client fields:

```json
{
  "device_id": "mock_exec",
  "primitive": "go_to_tag",
  "target": [7, 12],
  "deadline_ms": null
}
```

`CreateTask::into_task` adds Gateway-generated `id` and produces only:

```json
{
  "id": "task-1",
  "device_id": "mock_exec",
  "primitive": "go_to_tag",
  "target": [7, 12],
  "deadline_ms": null
}
```

There are no optional device IDs, target variants, capabilities, parameters, cancellation fields, or additional task fields in Gateway production Rust.

## Verification

All Rust compilation requiring ROS was run in `ros-dev-dev-1` with:

```sh
source /opt/ros/jazzy/setup.bash
source /ws/jazzy/install/setup.bash
cd /tmp
```

- `cargo fmt --manifest-path Cargo.toml --all`: exit 0.
- `cargo check --manifest-path /workspace/Cargo.toml -p gateway`: exit 0.
- `cargo check --manifest-path /workspace/Cargo.toml --workspace`: exit 0.
- `cargo clippy --manifest-path /workspace/Cargo.toml -p gateway -- -D warnings`: exit 0.
- `git diff --check`: exit 0.
- Focused Gateway search found no restricted Rust visibility, `TaskTarget`, capabilities, parameters, `GatewayState`, cancellation handler, or cancel route.
- Manual source inspection of the request and generated domain-task shapes above confirms the five-field contract.

## Runtime limitations / concerns

- Host checks cannot compile the concrete ROS path: `/opt/ros` is absent and `ROS_DISTRO`, `AMENT_PREFIX_PATH`, and `COLCON_PREFIX_PATH` are unset. The ROS container checks above are the available complete verification.
- Gateway runtime requires a reachable ROS action server for the configured device/action name. No HTTP end-to-end run was claimed because the current mock still requires the obsolete singular `{"target_tag": ...}` payload, while the agreed production client emits `{"target_tags": [...]}`. Task 5 manually confirmed that mismatch causes goal rejection; no compatibility payload was added.
- A transport failure returned through the execution session has no canonical terminal `ExecutionResult`; Gateway logs it rather than fabricating completion. Consequently, the Orchestrator remains lifecycle owner and no fake terminal projection is introduced.
