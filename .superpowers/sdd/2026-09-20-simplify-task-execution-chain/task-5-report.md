# Task 5 report

## Status

Implemented the real Execution → ros_task_client connection. Focused ROS-enabled check, build, and Clippy pass. Full Gateway-to-mock validation is blocked by pending Gateway composition and the mock's obsolete singular-target schema. No Gateway or mock implementation changes were made.

## Interface and dependency decisions

- `execution::Execution::start(DeviceId, String)` accepts the concrete device ID and absolute action endpoint. It owns both `RosTaskClient` and `RosTaskRuntime`; node name is `execution`, feedback capacity 64, server/acceptance timeout five seconds. Runtime drop shuts down ROS; explicit `shutdown()` is also available.
- `Execution` implements the existing `orchestration::ExecutionPort` directly and passes `Task` unchanged to the client. There is no additional task/session translation, fake executor, compatibility layer, or parallel execution path.
- The client directly depends on `platform`. Deleted its duplicate `ExecuteCommand`, `PrimitiveCommand`, `TaskFeedback`, `TaskResult`, `FinalState`, and `TaskSession` definitions. The authoritative input remains exactly `Task { id, device_id, primitive, target, deadline_ms }`.
- The existing ROS boundary (`mapper.rs`) now converts platform Task directly to ExecuteTask_Goal, retaining `go_to_tag`, `{"target_tags":[...]}`, checked unsigned-to-signed deadline conversion, and absent deadline = ROS zero. Feedback maps directly to `{task_id, progress, phase}`; result maps directly to `{task_id, state}`. ROS task-ID/status/progress validation remains in this boundary.
- The client returns platform `ExecutionSession` directly. Its existing bounded feedback channel is exposed as the platform stream via tokio-stream; its result channel becomes the platform future. Transport/channel failures become `ExecutionError::Failed` inside the ROS client. These are channel mechanics within the existing boundary, not a separate compatibility implementation.
- `ExecutionPort::execute` is now an object-safe boxed Send future returning `Result<ExecutionSession, ExecutionError>`. `Orchestrator::submit` is async and awaits it before inserting accepted task/device state. This is necessary because real ROS discovery and goal acceptance are asynchronous; reporting acceptance before ROS rejects a goal would be incorrect. Task 3 must call `submit(task).await`.
- Orchestration imports only platform/std types and has no ROS dependency. Dependency direction: execution → orchestration, execution → ros_task_client → platform. Colcon package.xml dependencies mirror the new Cargo edges.
- Removed the obsolete platform `Executor` trait/export and unused `ExecutionError::Busy`. Orchestration remains authoritative for device busy errors. ExecutionRuntime was already absent when this task began.
- Removed the client's standalone `[workspace]`, allowing its direct path dependency to join the existing root workspace. No physical crate move was needed. **The root Cargo.toml exclusion removal was already an uncommitted change on arrival and is required for this composition; it is preserved for the parent session to commit.** Root Cargo.lock was also already dirty; Cargo refreshed it during checks, and it is left for the parent to commit with the workspace metadata. The ignored local client Cargo.lock was removed because it is no longer authoritative.

## Changed tracked source/metadata

- `ros2_ws/src/services/execution/{Cargo.toml,package.xml,src/lib.rs}`
- `ros2_ws/src/services/orchestration/src/lib.rs`
- `ros2_ws/src/services/platform/src/{execution.rs,lib.rs}`
- `ros2_ws/src/control_plane/ros_task_client/{Cargo.toml,package.xml}`
- `ros2_ws/src/control_plane/ros_task_client/src/{client.rs,error.rs,lib.rs,mapper.rs}`
- Deleted `ros2_ws/src/control_plane/ros_task_client/src/types.rs`.
- Migrated the existing client consumers in `tests/{lifecycle.rs,mock_exec.rs}` to the canonical types/stream/error contract; no new tests, cases, or scaffolding were added. Tests were not run.
- This report.

Unrelated pre-existing `.devcontainer/devcontainer.json`, `.gitignore`, AGENTS.md, and design-plan changes were preserved and excluded from this commit, as were the parent-owned root Cargo metadata edits described above.

## Verification commands and output

1. Host `cargo check -p execution --lib`: exit 101 before compilation. `.cargo/config.toml` contains generated ROS patches; `/opt/ros/jazzy/share/action_msgs/rust/Cargo.toml` does not exist on the host. `/opt/ros` is absent and ROS_DISTRO/AMENT_PREFIX_PATH/COLCON_PREFIX_PATH are unset. Host ros2/colcon are unavailable.
2. Discovered running container with `docker ps --format '{{.Names}} {{.Image}}'`: `ros-dev-dev-1`. `docker inspect` confirms repository bind mounted at `/workspace` and ROS volume at `/ws`.
3. Container checks use this exact prefix, to source ROS and installed interfaces while avoiding the host-oriented repository Cargo patch configuration:

   ```sh
   docker exec ros-dev-dev-1 bash -lc 'source /opt/ros/jazzy/setup.bash && source /ws/jazzy/install/setup.bash && cd /tmp && cargo check --manifest-path /workspace/Cargo.toml -p execution --lib'
   ```

   Exit 0. Output includes checking platform, orchestration, ros_task_client, execution; `Finished dev profile ... in 3.48s`.
4. Same container prefix, `cargo build --manifest-path /workspace/Cargo.toml -p execution --lib`: exit 0; compiled all four crates, `Finished dev profile ... in 6.44s`.
5. Same prefix, `cargo clippy --manifest-path /workspace/Cargo.toml -p execution --lib -- -D warnings`: exit 0; `Finished dev profile ... in 1.62s`.
6. `cargo fmt -p platform -p orchestration -p execution -p ros_task_client`, followed by the same command with `--check`: exit 0, no output. `git diff --check`: exit 0, no output.
7. Same container prefix, `cargo check --manifest-path /workspace/Cargo.toml --workspace`: exit 101, five Gateway errors:
   - `dto.rs:2`: unresolved `platform::TaskTarget`.
   - `handlers.rs:47`: missing `TaskState::Canceled`.
   - `dto.rs:28`: `Option<DeviceId>` supplied where `DeviceId` is required.
   - `dto.rs:29`: removed `Task.required_capabilities` field.
   - `dto.rs:32`: removed `Task.parameters` field.
   These are in the expressly deferred Task 3 scope. A full workspace build cannot pass until those are addressed.
8. Manual ROS mock probe (no tests): started installed `ros2 run mock_exec_layer mock_exec_layer_node --ros-args -p step_delay_s:=0.05` in the container, then sourced the same ROS setup files and ran:

   ```sh
   timeout 15s ros2 action send_goal --feedback /mock_exec/execute_task task_interfaces/action/ExecuteTask '{task_id: task-5-manual, device_id: mock_exec, primitive: go_to_tag, payload_json: "{\"target_tags\":[7]}", deadline_unix_ms: 0}'
   ```

   Output printed the expected goal fields and `Goal was rejected.` (ros2 CLI itself exited 0). Source inspection confirms `mocks/mock_exec_layer/mock_exec_layer/contract.py:30-31` requires exactly `{'target_tag'}`; `execution.py` and `node.py` also read singular target_tag. This is a real schema blocker, not ROS discovery failure. No fallback to the old payload was added. The initial long-running launch command hit its one-second tool timeout; the created container processes were identified and explicitly stopped afterwards (SIGINT node PID 55464; SIGTERM launcher PIDs 55461, 55445, 55437).

## Concerns / parent handoff

1. Commit the pre-existing root workspace exclusion removal plus refreshed root lockfile when integrating; they were deliberately not absorbed into this source commit.
2. Task 3 must construct `Arc<Execution>`, pass it to Orchestrator, await submit, consume both session feedback and terminal future, and retain/shutdown the execution owner. Async submit holds its mutable Orchestrator borrow across goal acceptance (bounded five-second discovery/acceptance); Gateway lock ownership must account for that.
3. Feedback/result stream failures need Task 3's lifecycle handling so tasks are not left occupied after a transport failure; no synthetic terminal result was fabricated here.
4. The mock must adopt the already-agreed plural target list before ROS acceptance/end-to-end success can be verified. Per the repository's stop-on-design-conflict rule, integration validation stopped at this rejection; no mock schema or payload workaround was introduced.
5. Full `Gateway → Orchestrator → Execution → ros_task_client → Mock Exec` runtime success is **not** claimed. The concrete typed path through Execution is compiled and built, but runtime Gateway composition is explicitly sequenced later.
