# Task 2 Report: Internal Device Registry and Handlers

## Changed files

- `ros2_ws/src/control_plane/ros_task_client/src/client.rs`
  - Replaced the endpoint map with private `DeviceRegistry` and `DeviceHandler` types.
  - Added environment-backed `RosTaskClient::init()`.
  - Added registry-only asynchronous validation.
  - Resolved the handler before ROS goal mapping and submission.

## Checks and output

- `cargo fmt --manifest-path ros2_ws/src/control_plane/ros_task_client/Cargo.toml -- --check`
  - Passed (exit 0; no output).
- `cargo fmt --manifest-path ros2_ws/src/control_plane/ros_task_client/Cargo.toml`
  - Passed (exit 0; no output).
- `git diff --check`
  - Passed (exit 0; no output).
- `cargo check --manifest-path ros2_ws/src/control_plane/ros_task_client/Cargo.toml --lib`
  - Blocked before compilation because the host lacks the ROS dependency manifest: `/opt/ros/jazzy/share/action_msgs/rust/Cargo.toml`.

## Self-review

- `DeviceRegistry` and `DeviceHandler` are private to the ROS client module.
- The registry creates one `ActionClient<ExecuteTask>` for every enabled configured device and retains each action name for unavailable-server errors.
- `validate` performs only registry lookup and cannot wait for the ROS action graph.
- `execute` resolves a handler before mapping the task to a ROS goal; `ClientState` no longer has an endpoint list.
- Gateway and Orchestrator were not changed and do not receive registry, handler, or ROS types.
- No special Rust visibility qualifiers or compatibility startup path were added.

## Concerns

- ROS-dependent compilation could not run on this host because the Jazzy Rust package for `action_msgs` is absent.
- Existing callers of the removed `RosTaskClient::start` remain for the planned Task 3 migration; this Task 2 commit intentionally does not modify them.
