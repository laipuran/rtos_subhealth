# Simplify Task Execution Chain Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Simplify the backend execution chain to one concrete `GoToTag` task shape, remove cancellation/capability/parameter concepts, and keep ROS conversion confined below Orchestration.

**Architecture:** Gateway accepts a complete task containing only `task_id`, `device_id`, `primitive`, `target`, and `deadline_ms`. Orchestration owns task lifecycle and sends the same domain execution data to Execution. Execution owns the ROS boundary; `ros_task_client` is reduced to a thin ROS Action transport/type conversion component and is not imported by Gateway or Orchestration.

**Tech Stack:** Rust workspace, Tokio, Axum, ROS 2 `ExecuteTask` Action, existing Python Mock Exec.

**Spec:** User-approved direction in the current conversation; this plan assumes `target` is the ordered `Vec<i32>` of AprilTag IDs.

## Global Constraints

- Use only `pub` or private Rust visibility; never use `pub(crate)`, `pub(super)`, or other restricted `pub` forms.
- Remove obsolete code in every modified module; do not retain parallel legacy paths.
- Do not add compatibility wrappers, mappers, bridges, or extra adapters. The existing ROS boundary is the only transport conversion boundary.
- Do not add tests or test scaffolding. Validate with existing build/check commands and existing manual ROS mock execution only.
- Do not add task fields beyond `task_id`, `device_id`, `primitive`, `target`, and `deadline_ms`.
- `Primitive` has only `GoToTag`; remove `Hold`, `Stop`, `ExecutePrimitive`, `MoveToPose`, and `SetVelocity`.
- `target` is the ordered tag route; use `Vec<i32>` and do not add `parameters`.
- Cancellation is not part of the first implementation; remove its public and internal Rust flow.

---

### Task 1: Collapse the canonical task contract

**Files:**
- Modify: `ros2_ws/src/services/platform/src/task.rs:4-59`
- Modify: `ros2_ws/src/services/platform/src/domain.rs:12-19`
- Modify: `ros2_ws/src/services/platform/src/execution.rs:5-49`
- Modify: `ros2_ws/src/services/platform/src/lib.rs`
- Modify: `docs/contracts/task.md`
- Modify: `docs/contracts/execution.md`
- Modify: `docs/contracts/domain.md`

**Interfaces:**
- Produces one canonical task shape:

```rust
pub struct Task {
    pub id: TaskId,
    pub device_id: DeviceId,
    pub primitive: Primitive,
    pub target: Vec<i32>,
    pub deadline_ms: Option<u64>,
}
```

- Produces `Primitive::GoToTag` only.
- Removes `TaskTarget`, `required_capabilities`, `parameters`, optional `device_id`, and capability fields from `DeviceDescriptor`.
- Removes execution fields that have no consumer in the simplified path. Keep only the feedback/result fields needed to update Orchestration state and expose the Exec outcome.

- [ ] Replace `Primitive` with a one-variant enum and remove its obsolete `as_str` branches.
- [ ] Replace `TaskTarget` with `Task.target: Vec<i32>` and make `device_id` required.
- [ ] Remove capability fields and capability-related serialization/contracts.
- [ ] Change the execution boundary to consume the canonical task data without reintroducing `parameters`.
- [ ] Remove `cancel` and `ExecutionHandle` from the execution contract because the first path has no cancellation.
- [ ] Update the three contract documents so they describe only the five task fields and the ordered Tag route.

**Verification:** Run `cargo check --workspace` and use compiler errors to identify every stale contract consumer before proceeding.

---

### Task 2: Simplify device selection and Orchestration

**Files:**
- Modify: `ros2_ws/src/services/orchestration/src/lib.rs:14-141`
- Modify: `ros2_ws/src/services/orchestration/src/device.rs:1-41`
- Modify: `ros2_ws/src/services/orchestration/src/error.rs`
- Modify: `ros2_ws/src/services/execution/src/lib.rs:1-115`

**Interfaces:**
- `Orchestration` receives a task with a concrete `device_id`; it must not select a device by capabilities.
- `ExecutionPort` accepts the simplified task/execution command and returns the execution session/result required by the existing feedback/result path.
- No `cancel`, capability selection, `ExecutionHandle`, or `result(task_id)` API remains unless a remaining consumer is demonstrated by the compiler.

- [ ] Remove `DeviceRegistry::select` and the `supports` capability predicate.
- [ ] Remove registry ownership from `Orchestrator` if no remaining code consumes registry data after concrete `device_id` is mandatory.
- [ ] Make `submit` validate and forward the concrete task to Execution without constructing a second command containing unused fields.
- [ ] Retain only active-task state needed to apply feedback and terminal results.
- [ ] Remove `Orchestrator::cancel` and all cancellation error variants.
- [ ] Remove the unused generic ExecutionRuntime state/handle paths rather than preserving an unused abstraction.
- [ ] Keep Orchestration feedback/result transitions limited to fields that update task state, progress, phase, and terminal outcome.

**Verification:** Run `cargo check --workspace`; confirm no production Rust code references capabilities, cancellation, parameters, or removed primitives.

---

### Task 3: Reduce Gateway input and route creation into Orchestration

**Files:**
- Modify: `ros2_ws/src/services/gateway/src/dto.rs:1-44`
- Modify: `ros2_ws/src/services/gateway/src/state.rs:1-100`
- Modify: `ros2_ws/src/services/gateway/src/handlers.rs:1-69`
- Modify: `ros2_ws/src/services/gateway/src/lib.rs:1-22`
- Modify: `ros2_ws/src/services/gateway/src/main.rs:1-15`
- Modify: `ros2_ws/src/services/gateway/Cargo.toml`

**Interfaces:**
- `CreateTask` contains exactly:

```rust
pub device_id: DeviceId,
pub primitive: Primitive,
pub target: Vec<i32>,
pub deadline_ms: Option<u64>,
```

- `create_task` creates a domain task, submits it to Orchestration, and stores a Gateway projection only after successful submission.
- The Gateway state contains the task query projection, event sender, task ID sequence, and the chosen Orchestration control handle/state owner; it does not duplicate a registry or execution client.

- [ ] Remove `required_capabilities`, `parameters`, optional device IDs, and all old target variants from `CreateTask` and `into_task`.
- [ ] Remove the cancel route and handler from `lib.rs` and `handlers.rs`.
- [ ] Finish `AppState`/`AppStateInner` initialization; create the event channel and task sequence without nested redundant `Arc`s.
- [ ] Inject the simplified Orchestrator execution dependency from `main.rs` rather than constructing an incomplete default Orchestrator inside Gateway state.
- [ ] Change `create_task` to submit before inserting the projection, so failed execution submission does not create an accepted task that never reached Exec.
- [ ] Keep `tasks` only as the Gateway query/history projection; Orchestrator remains the lifecycle owner.
- [ ] Emit only the existing task-state event when a state transition has a real consumer; do not add event fields without a consumer.

**Verification:** Run `cargo check --workspace` and manually inspect the generated HTTP task shape against the five-field contract.

---

### Task 4: Make `ros_task_client` a thin ROS boundary

**Files:**
- Modify: `ros2_ws/src/control_plane/ros_task_client/src/types.rs:1-128`
- Modify: `ros2_ws/src/control_plane/ros_task_client/src/mapper.rs:1-240`
- Modify: `ros2_ws/src/control_plane/ros_task_client/src/client.rs:30-448`
- Modify: `ros2_ws/src/control_plane/ros_task_client/src/error.rs:1-44`
- Modify: `ros2_ws/src/control_plane/ros_task_client/src/lib.rs:1-17`
- Delete obsolete cancellation-only code in the existing ROS client test modules without adding replacement tests.

**Interfaces:**
- The client consumes the canonical execution data: `task_id`, `device_id`, `GoToTag`, ordered tag target, and deadline.
- The client produces only the execution feedback/result stream required by Execution/Orchestration.
- ROS conversion is limited to:

```text
domain target Vec<i32> → ExecuteTask_Goal.payload_json
domain deadline_ms     → ExecuteTask_Goal.deadline_unix_ms
ROS feedback/result    → execution feedback/result
```

- [ ] Remove `Hold`, `PrimitiveDetails`, and all other primitive-specific variants except `GoToTag`.
- [ ] Remove `CancellationHandle`, `CancelRequest`, `PendingCancel`, `AwaitingCancel`, cancel response mapping, and cancel-specific relay branches.
- [ ] Keep one relay path for feedback, result, and shutdown only.
- [ ] Serialize the ordered target as the single `go_to_tag` payload `{"target_tags":[...]}`; do not invent a second parameters object.
- [ ] Remove parsing/validation for fields no longer consumed by Rust.
- [ ] Retain Action Server readiness and ROS executor lifecycle only because the ROS transport requires them.
- [ ] Reduce `RosTaskError` to configuration, command, availability, mapping, channel, shutdown, and ROS execution failures that remain reachable.

**Verification:** Run the existing package build/check command for `ros_task_client` in the ROS environment; manually send one `go_to_tag` goal to the existing Mock Exec and verify feedback followed by a terminal result.

---

### Task 5: Connect Execution to the thin ROS boundary

**Files:**
- Modify: `ros2_ws/src/services/execution/src/lib.rs`
- Modify: `ros2_ws/src/services/execution/Cargo.toml`
- Modify: `ros2_ws/src/control_plane/ros_task_client/Cargo.toml` only if the direct canonical contract dependency is required
- Modify: `ros2_ws/src/services/gateway/src/main.rs:1-15` as the current runtime composition entrypoint

**Interfaces:**
- Execution owns the ROS-specific boundary and calls the thin `ros_task_client` directly.
- Orchestration sees only the execution contract; it does not import `rclrs`, generated ROS Action types, or `ros_task_client` transport details.
- No separate compatibility adapter, mapper, wrapper, or bridge is introduced between Orchestration and Execution.

- [ ] Replace the current unused generic `Executor`/`ExecutionRuntime` path with the one concrete execution path needed by `GoToTag`.
- [ ] Construct the ROS endpoint configuration from the concrete device ID/action endpoint without adding task fields.
- [ ] Forward the canonical task target and deadline to the ROS client.
- [ ] Route feedback and terminal result back to Orchestration using only the reduced execution contract.
- [ ] Remove any old implementation that remains unreachable after the direct path is connected.

**Verification:** Run the complete workspace build/check plus the existing ROS mock manual flow; confirm the call chain is `Gateway → Orchestrator → Execution → ros_task_client → Mock Exec`.

---

### Task 6: Remove stale API surface and documentation

**Files:**
- Modify or delete stale references across `ros2_ws/src`, `docs/contracts`, `docs/architecture`, `docs/guide/ros-mocks.md`, and root build files.
- Modify: `ros2_ws/src/mocks/mock_exec_layer/mock_exec_layer/contract.py`
- Modify: `ros2_ws/src/mocks/mock_exec_layer/mock_exec_layer/execution.py`
- Modify: `ros2_ws/src/mocks/mock_exec_layer/mock_exec_layer/node.py`
- Remove old routes/configuration that describe cancellation, capabilities, parameters, Hold, or unrelated primitives.

**Interfaces:**
- Documentation and build metadata describe one implementation path only.

- [ ] Search for and remove every production reference to `required_capabilities`, `capabilities`, `parameters`, `Hold`, `Stop`, `MoveToPose`, `SetVelocity`, and cancellation.
- [ ] Update ROS mock documentation to describe the final `GoToTag` target payload and the no-cancel first version.
- [ ] Update Mock Exec to consume `{"target_tags":[...]}` and execute the ordered route without adding a new task field.
- [ ] Ensure no stale `pub(crate)` or other restricted visibility remains in modified Rust modules.
- [ ] Run `cargo fmt --check` and the repository’s existing build/check command; do not add tests.

**Verification:** Confirm with repository-wide search that only the five task fields and one primitive remain in the canonical backend path.

**Review follow-up:** Removed the stale capability reference from `docs/contracts/domain.md`; repository-wide search now reports no capability reference in that contract.

---

## Open design assumption to confirm before implementation

This plan treats `target` as the ordered AprilTag route:

```rust
pub target: Vec<i32>
```

The ROS payload emitted for this target is exactly `{"target_tags":[...]}`. The plan also treats `deadline_ms` as an absolute Unix epoch millisecond value, with `None`/`0` meaning no deadline, so the ROS boundary can pass it through as `deadline_unix_ms`. If either target encoding or deadline semantics differs, that contract must be decided before Task 1 starts; no extra field should be introduced implicitly.
