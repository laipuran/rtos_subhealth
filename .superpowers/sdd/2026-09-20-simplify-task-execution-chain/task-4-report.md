# Task 4 report — thin ROS task client

## Status

Implemented the client changes. Formatting and whitespace checks pass. ROS compilation and live feedback/result verification are blocked by the host environment; no build or runtime success is claimed.

## Files

All code paths below are relative to `ros2_ws/src/control_plane/ros_task_client/`:

- `src/types.rs`: canonical command fields (`task_id`, `device_id`, unit `GoToTag`, ordered `target: Vec<i32>`, `deadline_ms: Option<u64>`); minimal feedback and result; feedback/result-only session. Removed Hold, primitive detail types, timestamps, feedback state, canceled terminal state, and cancellation handles/requests.
- `src/mapper.rs`: serialize exactly `{"target_tags":[...]}` from the target slice, preserving order and duplicates. Map deadline to ROS int64 with checked conversion and absent deadline to zero. Retain task correlation, consumed progress validation, and terminal status/final-state consistency checks. Remove all parsing and validation of unused detail JSON, feedback state, timestamps, result error codes, and messages. Removed obsolete mapper tests that depended on the removed command/detail/timestamp/cancellation boundary.
- `src/client.rs`: one feedback/result/shutdown relay; remove cancellation channels, states, selectors, responses, and graph requirement. Separate readiness waiting, relay, and feedback delivery into focused functions. Preserve result priority, bounded ready-feedback drain, bounded public feedback channel errors, endpoint lookup, readiness timeout, goal rejection, and shutdown signaling. Retain the existing cloneability test; remove tests coupled to eliminated relay selectors and old mapping contracts.
- `src/error.rs`: remove unreachable cancellation error.
- `src/lib.rs`: export only remaining public types.
- `src/runtime.rs`: change the existing cross-module `start` visibility to plain `pub`, as required by AGENTS.md. Retain ROS executor startup/join/shutdown lifecycle.
- `Cargo.toml`: remove unused direct `futures` dependency.
- `tests/lifecycle.rs`: delete the obsolete cancellation-handle drop test; mechanically migrate existing command fixtures to the canonical GoToTag fields.
- `tests/mock_exec.rs`: remove obsolete Hold/cancellation/detail/error-metadata tests and their unused scaffolding; retain the existing feedback-overflow test with its command fixture migrated to the canonical fields. No new or replacement tests were introduced.

No Gateway, Orchestration, Execution, platform, or Python Mock Exec files were modified.

## Decisions

- Public client data mirrors the canonical execution data without introducing a platform dependency, compatibility layer, wrapper, or second execution path.
- Feedback contains only task ID, progress, and phase; result contains only task ID and succeeded/failed terminal state. These are the values consumed by the completed platform execution boundary. Unsupported ROS terminal pairs produce a mapping error; there is no cancellation-specific branch.
- Deadline values above `i64::MAX` produce `InvalidCommand` instead of truncation. Domain validation stays upstream; ROS boundary validation is limited to representability and consumed response fields.
- Readiness checks the send-goal/get-result services and feedback/status publishers. The cancellation service is no longer a readiness dependency. rclrs still owns its native ROS action transport entities internally.
- Shutdown closes the relay with `Shutdown` and joins the executor through the existing lifecycle implementation. It does not send a ROS cancellation request.
- The existing bounded feedback drain limit (16) and overflow behavior are retained.

## Verification commands and results

1. `cargo fmt --manifest-path ros2_ws/src/control_plane/ros_task_client/Cargo.toml`
   - Exit 0, no output.
2. `cargo fmt --manifest-path ros2_ws/src/control_plane/ros_task_client/Cargo.toml -- --check`
   - Exit 0, no output, including after the final code edit.
3. `git diff --check`
   - Exit 0, no output.
4. `cargo check --manifest-path ros2_ws/src/control_plane/ros_task_client/Cargo.toml --lib`
   - Exit 101 before checking the client:
   ```text
   error: failed to load source for dependency `action_msgs`
   Unable to update /opt/ros/jazzy/share/action_msgs/rust
   failed to read `/opt/ros/jazzy/share/action_msgs/rust/Cargo.toml`
   No such file or directory (os error 2)
   ```
5. From `/tmp/opencode`, to avoid checkout-local Cargo patch configuration:
   `cargo check --manifest-path /home/duckran/codes/rtos_subhealth/ros2_ws/src/control_plane/ros_task_client/Cargo.toml --lib --offline`
   - Exit 101: `no matching package named ros-env found`, searched crates.io index. The offline registry is insufficient as well.
6. Environment inspection: `ROS_DISTRO` and `AMENT_PREFIX_PATH` are unset; `command -v ros2 colcon` found neither executable; `/opt/ros` does not exist.
7. Manual goal attempt:
   ```sh
   ros2 action send_goal /mock_exec/execute_task task_interfaces/action/ExecuteTask \
     '{task_id: task-4-manual, device_id: mock_exec, primitive: go_to_tag, payload_json: "{\"target_tags\":[7]}", deadline_unix_ms: 0}' --feedback
   ```
   - Exit 127: `zsh:1: command not found: ros2`. No goal was sent and no feedback/result was observed.
8. Source scan across package Rust files for `Hold|Cancellation|CancelRequest|PendingCancel|AwaitingCancel|cancel|PrimitiveDetails|RosTimestamp|FeedbackState|pub\(` returned no matches.
9. Inspected installed rclrs 0.7 source: `RequestedGoalClient` implements `Future<Output = Option<GoalClient<A>>>`; `GoalClient` exposes independent feedback/result fields; feedback dereferences to Tokio's unbounded receiver. This supports the retained transport usage but does not substitute for compilation.

## Concerns / follow-up

- Run the non-test package check in a sourced ROS environment with generated task interfaces before claiming compilation success.
- The unchanged Python Mock Exec currently requires exactly `target_tag` in `contract.py`; it will reject the required `target_tags` payload until its separate planned migration lands. Live end-to-end verification therefore also depends on that task.
- Re-run the manual goal above after the Mock Exec migration and verify feedback followed by succeeded/failed result.
- The retained mock integration coverage now checks feedback overflow only; removed Hold/cancellation/detail tests have no replacements, as requested.
- Existing unrelated working-tree modifications in `.devcontainer/devcontainer.json`, `.gitignore`, root `Cargo.toml`, root `Cargo.lock`, untracked `AGENTS.md`, and the plan document were left out of this task's commit.
