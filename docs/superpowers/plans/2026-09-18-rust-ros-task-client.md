# Rust ROS Task Client Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build and verify a typed Rust `ExecuteTask` ActionClient that connects the future Server to the Python mock Exec without moving or redesigning current Rust services.

**Architecture:** An `ament_cargo` library under `ros2_ws` owns a dedicated blocking `rclrs` executor thread while exposing Tokio-native typed commands, feedback, results, and cancellation. Generated ROS types and JSON remain private to client/mapper modules; the future Server calls the concrete client with typed endpoint configuration.

**Tech Stack:** ROS 2 Humble/Jazzy, Rust 1.85, `rclrs 0.7`, `ros-env 0.2`, `rosidl_runtime_rs 0.6`, Tokio, Serde, colcon/ament_cargo, Python `rclpy` mock Exec.

**Spec:** `docs/superpowers/specs/2026-09-18-rust-ros-task-client-design.md`

## Global Constraints

- Create only `ros2_ws/src/control_plane/ros_task_client`; do not move or behaviorally modify any crate under `services/`.
- Do not add `ExecutionPort`, adapter traits, HTTP concerns, DeviceRegistry behavior, Sensor transport, persistence, or Server composition.
- Public API must not expose generated ROS types, `rclrs` handles, or JSON strings.
- Generated ROS types may appear only in private `client.rs` and `mapper.rs`; only `mapper.rs` handles JSON.
- Blocking ROS spin runs on a dedicated `std::thread`, never Tokio `spawn`/`spawn_blocking`.
- Tokio tasks may relay native async `rclrs` goal streams while the ROS thread services wait sets.
- Endpoint configuration is immutable and explicit: `device_id` never implies `action_name`.
- The ROS-only crate remains excluded from the top-level pure Cargo workspace.
- Cargo versions are locked; third-party ros2-rust sources are not vendored.
- Follow red-green-refactor and commit after each independently testable task.

## File Structure

Create:

```text
ros2_ws/src/control_plane/ros_task_client/
├── package.xml
├── Cargo.toml
├── Cargo.lock
├── src/
│   ├── lib.rs
│   ├── client.rs
│   ├── config.rs
│   ├── error.rs
│   ├── mapper.rs
│   ├── runtime.rs
│   └── types.rs
├── tests/
│   ├── generated_action.rs
│   ├── config.rs
│   ├── lifecycle.rs
│   └── mock_exec.rs
└── test/
    └── run_mock_exec_integration.sh
```

Modify:

```text
Cargo.toml
docker/dev/Dockerfile
Makefile
docs/guide/ros-mocks.md
```

Do not modify `task_interfaces/package.xml` unless Task 1 proves the installed generator requires an explicit dependency.

---

### Task 1: Rust ROS Generator and Package Build Proof

**Files:**
- Create: `ros2_ws/src/control_plane/ros_task_client/package.xml`
- Create: `ros2_ws/src/control_plane/ros_task_client/Cargo.toml`
- Create: `ros2_ws/src/control_plane/ros_task_client/src/lib.rs`
- Create: `ros2_ws/src/control_plane/ros_task_client/tests/generated_action.rs`
- Modify: `Cargo.toml:1-10`
- Modify: `docker/dev/Dockerfile:14-24`

**Interfaces:**
- Consumes: existing `task_interfaces/action/ExecuteTask.action`.
- Produces: an `ament_cargo` library able to import generated `ExecuteTask` Rust types.

- [ ] **Step 1: Create metadata and the failing generated-type test**

Create `package.xml`:

```xml
<?xml version="1.0"?>
<package format="3">
  <name>ros_task_client</name>
  <version>0.1.0</version>
  <description>Typed Rust ROS ExecuteTask action client.</description>
  <maintainer email="puranlai@qq.com">Duck Ran</maintainer>
  <license>Apache-2.0</license>
  <depend>rclrs</depend>
  <depend>rosidl_runtime_rs</depend>
  <depend>task_interfaces</depend>
  <export><build_type>ament_cargo</build_type></export>
</package>
```

Create `Cargo.toml`:

```toml
[package]
name = "ros_task_client"
version = "0.1.0"
edition = "2021"
rust-version = "1.85"
license = "Apache-2.0"

[dependencies]
futures = "0.3"
rclrs = "0.7"
ros-env = "0.2"
rosidl_runtime_rs = "0.6"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "sync", "time"] }

[package.metadata.ros]
```

Create `src/lib.rs`:

```rust
//! Typed Rust client for the ExecuteTask ROS action.
```

Create `tests/generated_action.rs`:

```rust
use ros_env::task_interfaces::action::{
    ExecuteTask, ExecuteTask_Feedback, ExecuteTask_Goal, ExecuteTask_Result,
};
use rosidl_runtime_rs::Action;

fn assert_action<A: Action>() {}

#[test]
fn execute_task_has_generated_rust_action_types() {
    assert_action::<ExecuteTask>();
    let _: <ExecuteTask as Action>::Goal = ExecuteTask_Goal::default();
    let _: <ExecuteTask as Action>::Feedback = ExecuteTask_Feedback::default();
    let _: <ExecuteTask as Action>::Result = ExecuteTask_Result::default();
}
```

- [ ] **Step 2: Exclude the package from the pure Cargo workspace**

Add after `resolver = "2"` in root `Cargo.toml`:

```toml
exclude = ["ros2_ws/src/control_plane/ros_task_client"]
```

- [ ] **Step 3: Verify RED before generator installation**

Run:

```bash
docker compose -f docker/dev/compose.yaml run --rm dev bash -lc '
  source /opt/ros/$ROS_DISTRO/setup.bash
  cargo test --manifest-path ros2_ws/src/control_plane/ros_task_client/Cargo.toml \
    --test generated_action
'
```

Expected: FAIL because the current image/workspace has no generated Rust `task_interfaces`. If it passes, retain the regression test and record the installed generator version.

- [ ] **Step 4: Install the distro-matched generator**

After the current base dependencies in `docker/dev/Dockerfile`, add:

```dockerfile
RUN case "${ROS_DISTRO}:${UBUNTU_VERSION}" in \
      humble:22.04) package_branch=jammy-humble ;; \
      jazzy:24.04) package_branch=noble-jazzy ;; \
      *) echo "unsupported ROS/Ubuntu pair: ${ROS_DISTRO}/${UBUNTU_VERSION}" >&2; exit 2 ;; \
    esac \
 && echo "deb [trusted=yes] https://raw.githubusercontent.com/ros2-rust/rosidl_rust/${package_branch}/ ./" \
      > /etc/apt/sources.list.d/ros2-rust-rosidl.list \
 && apt-get update \
 && apt-get install -y --no-install-recommends ros-${ROS_DISTRO}-rosidl-generator-rs \
 && rm -rf /var/lib/apt/lists/*
```

- [ ] **Step 5: Rebuild Jazzy and build the package**

Run:

```bash
ROS_DISTRO=jazzy UBUNTU_VERSION=24.04 docker compose -f docker/dev/compose.yaml build dev
docker compose -f docker/dev/compose.yaml run --rm dev make build
```

Expected: colcon builds `task_interfaces` before `ros_task_client`; Cargo creates the package-local `Cargo.lock`.

- [ ] **Step 6: Verify GREEN**

Run:

```bash
docker compose -f docker/dev/compose.yaml run --rm dev bash -lc '
  source /opt/ros/$ROS_DISTRO/setup.bash
  source /ws/install/setup.bash
  cargo test --locked --manifest-path ros2_ws/src/control_plane/ros_task_client/Cargo.toml \
    --test generated_action
'
```

Expected: PASS. Do not continue if custom Action Rust generation is unavailable.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml docker/dev/Dockerfile ros2_ws/src/control_plane/ros_task_client
git commit -m "build(ros): generate Rust task action types"
```

---

### Task 2: Typed Boundary, Errors, and Configuration

**Files:**
- Create: `ros2_ws/src/control_plane/ros_task_client/src/types.rs`
- Create: `ros2_ws/src/control_plane/ros_task_client/src/error.rs`
- Create: `ros2_ws/src/control_plane/ros_task_client/src/config.rs`
- Create: `ros2_ws/src/control_plane/ros_task_client/tests/config.rs`
- Modify: `ros2_ws/src/control_plane/ros_task_client/src/lib.rs`

**Interfaces:**
- Consumes: `Duration`, Tokio channel types.
- Produces: typed command/feedback/result values, `RosConnectionConfig`, and `RosTaskError`.

- [ ] **Step 1: Write failing configuration tests**

Create `tests/config.rs`:

```rust
use ros_task_client::{ExecEndpointConfig, RosConnectionConfig, RosTaskError};
use std::time::Duration;

fn valid_config() -> RosConnectionConfig {
    RosConnectionConfig {
        node_name: "control_plane_server".into(),
        endpoints: vec![ExecEndpointConfig {
            device_id: "mock_exec".into(),
            action_name: "/mock_exec/execute_task".into(),
        }],
        feedback_buffer: 16,
        server_wait_timeout: Duration::from_secs(2),
    }
}

#[test]
fn valid_static_endpoint_configuration_is_accepted() {
    assert!(valid_config().validate().is_ok());
}

#[test]
fn duplicate_device_ids_and_action_names_are_rejected() {
    let mut ids = valid_config();
    ids.endpoints.push(ExecEndpointConfig {
        device_id: "mock_exec".into(), action_name: "/other/task".into(),
    });
    assert!(matches!(ids.validate(), Err(RosTaskError::InvalidConfig { .. })));

    let mut names = valid_config();
    names.endpoints.push(ExecEndpointConfig {
        device_id: "other".into(), action_name: "/mock_exec/execute_task".into(),
    });
    assert!(matches!(names.validate(), Err(RosTaskError::InvalidConfig { .. })));
}

#[test]
fn empty_names_zero_buffer_and_zero_timeout_are_rejected() {
    let mut node = valid_config(); node.node_name.clear(); assert!(node.validate().is_err());
    let mut device = valid_config(); device.endpoints[0].device_id.clear(); assert!(device.validate().is_err());
    let mut action = valid_config(); action.endpoints[0].action_name.clear(); assert!(action.validate().is_err());
    let mut buffer = valid_config(); buffer.feedback_buffer = 0; assert!(buffer.validate().is_err());
    let mut timeout = valid_config(); timeout.server_wait_timeout = Duration::ZERO; assert!(timeout.validate().is_err());
}
```

- [ ] **Step 2: Verify RED**

Run the sourced Cargo command from Task 1 with `--test config`.

Expected: FAIL because exported configuration types do not exist.

- [ ] **Step 3: Implement exact boundary values**

Create `types.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecuteCommand {
    pub task_id: String,
    pub device_id: String,
    pub primitive: PrimitiveCommand,
    pub deadline_unix_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrimitiveCommand { Hold, GoToTag { target_tag: i32 } }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackState { Running, Canceled }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrimitiveDetails {
    Hold,
    GoToTag { current_tag: Option<i32>, next_tag: Option<i32> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RosTimestamp { pub sec: i32, pub nanosec: u32 }

#[derive(Debug, Clone, PartialEq)]
pub struct TaskFeedback {
    pub task_id: String,
    pub state: FeedbackState,
    pub progress: f32,
    pub phase: String,
    pub details: PrimitiveDetails,
    pub timestamp: RosTimestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinalState { Succeeded, Failed, Canceled }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskResult {
    pub task_id: String,
    pub final_state: FinalState,
    pub error_code: Option<String>,
    pub message: String,
    pub finished_time: RosTimestamp,
}
```

- [ ] **Step 4: Implement errors and validation**

Define `RosTaskError` in `error.rs` with these variants:

```rust
InvalidConfig { field: &'static str, message: String }
InvalidCommand { field: &'static str, message: String }
UnknownDevice { device_id: String }
ActionServerUnavailable { device_id: String, action_name: String }
GoalRejected { task_id: String }
CancelRejected { task_id: String, reason: String }
Mapping { field: &'static str, message: String }
Ros(String)
ChannelClosed { channel: &'static str }
Shutdown
ExecutorPanicked
```

Use `thiserror::Error` and stable human-readable messages. In `config.rs`, define the two config structs from the spec and `pub fn validate(&self) -> Result<(), RosTaskError>`. Use two `HashSet<&str>` values for duplicate detection. Reject empty or whitespace-padded names; never silently trim identifiers.

- [ ] **Step 5: Export the boundary**

Update `lib.rs`:

```rust
mod config;
mod error;
mod types;

pub use config::{ExecEndpointConfig, RosConnectionConfig};
pub use error::RosTaskError;
pub use types::{
    ExecuteCommand, FeedbackState, FinalState, PrimitiveCommand,
    PrimitiveDetails, RosTimestamp, TaskFeedback, TaskResult,
};
```

- [ ] **Step 6: Verify GREEN and commit**

Run:

```bash
cargo test --locked --manifest-path \
  ros2_ws/src/control_plane/ros_task_client/Cargo.toml
```

inside the sourced container. Expected: generated-action and config tests pass without warnings.

```bash
git add ros2_ws/src/control_plane/ros_task_client
git commit -m "feat(ros): define typed task client boundary"
```

---

### Task 3: Private ROS and JSON Mapper

**Files:**
- Create: `ros2_ws/src/control_plane/ros_task_client/src/mapper.rs`
- Modify: `ros2_ws/src/control_plane/ros_task_client/src/lib.rs`
- Test: private `#[cfg(test)]` module in `mapper.rs`

**Interfaces:**
- Consumes: Task 2 types and generated Goal/Feedback/Result.
- Produces: private `to_ros_goal`, `from_ros_feedback`, `from_ros_result`.

- [ ] **Step 1: Add failing goal tests**

In `mapper.rs`, add tests asserting these exact values:

```rust
#[test]
fn hold_maps_to_empty_payload_and_zero_deadline() {
    let raw = to_ros_goal(&ExecuteCommand {
        task_id: "task-1".into(), device_id: "mock_exec".into(),
        primitive: PrimitiveCommand::Hold, deadline_unix_ms: None,
    }).unwrap();
    assert_eq!(raw.primitive, "hold");
    assert_eq!(raw.payload_json, "{}");
    assert_eq!(raw.deadline_unix_ms, 0);
}

#[test]
fn go_to_tag_maps_to_compact_payload() {
    let raw = to_ros_goal(&ExecuteCommand {
        task_id: "task-2".into(), device_id: "mock_exec".into(),
        primitive: PrimitiveCommand::GoToTag { target_tag: 42 },
        deadline_unix_ms: Some(1_789_700_000_000),
    }).unwrap();
    assert_eq!(raw.primitive, "go_to_tag");
    assert_eq!(raw.payload_json, r#"{"target_tag":42}"#);
}
```

Add invalid cases for empty IDs and `Some(0)`/negative deadlines.

- [ ] **Step 2: Verify RED**

Run in the sourced container:

```bash
cargo test --locked --manifest-path \
  ros2_ws/src/control_plane/ros_task_client/Cargo.toml \
  mapper::tests -- --nocapture
```

Expected: FAIL because mapper functions do not exist.

- [ ] **Step 3: Implement goal mapping**

Use:

```rust
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GoToTagPayload { target_tag: i32 }

pub(crate) fn to_ros_goal(
    command: &ExecuteCommand,
) -> Result<ExecuteTask_Goal, RosTaskError>;
```

Construct generated messages with `Default` and explicit field assignment. Use `serde_json::to_string` for compact deterministic JSON.

- [ ] **Step 4: Add failing feedback tests**

Test the literal conversion:

```rust
let raw = ExecuteTask_Feedback {
    task_id: "task-2".into(), state: "running".into(), progress: 0.5,
    phase: "moving_to_tag".into(),
    details_json: r#"{"current_tag":1,"next_tag":-1}"#.into(),
    timestamp: Time { sec: 10, nanosec: 20 },
};
let mapped = from_ros_feedback(
    "task-2", &PrimitiveCommand::GoToTag { target_tag: 42 }, raw,
).unwrap();
assert_eq!(mapped.details, PrimitiveDetails::GoToTag {
    current_tag: Some(1), next_tag: None,
});
```

Also reject mismatched task IDs, malformed/extra JSON, wrong primitive details, unknown states, NaN/infinite/out-of-range progress, tags below `-1`, and nanoseconds at or above one billion.

- [ ] **Step 5: Implement strict feedback mapping**

Implement:

```rust
pub(crate) fn from_ros_feedback(
    expected_task_id: &str,
    primitive: &PrimitiveCommand,
    raw: ExecuteTask_Feedback,
) -> Result<TaskFeedback, RosTaskError>;
```

Use `#[serde(deny_unknown_fields)]`; map `-1` to `None`, non-negative tags to `Some`, and reject lower values. Reject invalid progress rather than clamping it.

- [ ] **Step 6: Add failing result tests**

Cover all valid states and all invalid combinations. Enforce:

```text
GoalStatusCode::Succeeded ↔ FinalState::Succeeded
GoalStatusCode::Cancelled ↔ FinalState::Canceled
GoalStatusCode::Aborted   ↔ FinalState::Failed
```

Assert empty error code becomes `None`, while failed `SDK_ERROR` remains unchanged.

- [ ] **Step 7: Implement result mapping**

```rust
pub(crate) fn from_ros_result(
    expected_task_id: &str,
    status: rclrs::GoalStatusCode,
    raw: ExecuteTask_Result,
) -> Result<TaskResult, RosTaskError>;
```

Reject nonterminal ROS statuses, unknown string states, mismatched task IDs/statuses, and malformed timestamps.

- [ ] **Step 8: Verify and commit**

Run all package tests. Expected: generated/config/mapper tests pass.

```bash
git add ros2_ws/src/control_plane/ros_task_client/src
git commit -m "feat(ros): map typed tasks to ExecuteTask action"
```

---

### Task 4: Dedicated ROS Executor Lifecycle

**Files:**
- Create: `ros2_ws/src/control_plane/ros_task_client/src/runtime.rs`
- Create: `ros2_ws/src/control_plane/ros_task_client/src/client.rs`
- Create: `ros2_ws/src/control_plane/ros_task_client/tests/lifecycle.rs`
- Modify: `ros2_ws/src/control_plane/ros_task_client/src/lib.rs`

**Interfaces:**
- Consumes: validated config and generated `ExecuteTask` marker.
- Produces: `RosTaskClient::start(config)` and idempotent `RosTaskRuntime::shutdown()`.

- [ ] **Step 1: Write the failing lifecycle test**

```rust
#[test]
fn runtime_shutdown_is_idempotent_and_joins_executor() {
    let config = RosConnectionConfig {
        node_name: format!("ros_task_client_lifecycle_{}", std::process::id()),
        endpoints: vec![ExecEndpointConfig {
            device_id: "mock_exec".into(),
            action_name: "/mock_exec/execute_task".into(),
        }],
        feedback_buffer: 8,
        server_wait_timeout: Duration::from_millis(100),
    };
    let (_client, runtime) = RosTaskClient::start(config).unwrap();
    runtime.shutdown().unwrap();
    runtime.shutdown().unwrap();
}
```

- [ ] **Step 2: Verify RED**

Run sourced Cargo with `--test lifecycle`. Expected: FAIL because runtime/client are absent.

- [ ] **Step 3: Implement startup**

Create context, basic executor, node, and one `ActionClient<ExecuteTask>` per endpoint before starting a named `std::thread`. The thread owns the executor and calls:

```rust
executor.spin(rclrs::SpinOptions::default())
```

The client owns the endpoint map, shared `AtomicBool` stopping flag, and a
`tokio::sync::watch::Receiver<bool>` used by goal relays. Runtime owns cloned
`ExecutorCommands`, the matching `watch::Sender<bool>`, and
`Mutex<Option<JoinHandle<Vec<RclrsError>>>>`.

- [ ] **Step 4: Implement idempotent shutdown**

Implement:

```rust
pub fn shutdown(&self) -> Result<(), RosTaskError> {
    self.stopping.store(true, Ordering::Release);
    let _ = self.shutdown_tx.send(true);
    self.commands.halt_spinning();
    let Some(thread) = self.thread.lock().expect("runtime mutex poisoned").take() else {
        return Ok(());
    };
    let errors = thread.join().map_err(|_| RosTaskError::ExecutorPanicked)?;
    if errors.is_empty() { Ok(()) } else { Err(RosTaskError::Ros(format_errors(errors))) }
}
```

`Drop` performs best-effort shutdown and never panics.

- [ ] **Step 5: Export and stress lifecycle**

Export `RosTaskClient` and `RosTaskRuntime`. In the sourced container run:

```bash
for i in $(seq 1 20); do
  cargo test --manifest-path ros2_ws/src/control_plane/ros_task_client/Cargo.toml \
    --test lifecycle -- --test-threads=1 || exit 1
done
```

Expected: all runs terminate without hanging.

- [ ] **Step 6: Verify and commit**

```bash
git add ros2_ws/src/control_plane/ros_task_client
git commit -m "feat(ros): own task client executor lifecycle"
```

---

### Task 5: Successful Goal, Feedback, and Result Flow

**Files:**
- Modify: `ros2_ws/src/control_plane/ros_task_client/src/types.rs`
- Modify: `ros2_ws/src/control_plane/ros_task_client/src/client.rs`
- Create: `ros2_ws/src/control_plane/ros_task_client/tests/mock_exec.rs`
- Create: `ros2_ws/src/control_plane/ros_task_client/test/run_mock_exec_integration.sh`

**Interfaces:**
- Consumes: mapper, endpoint ActionClients, running executor.
- Produces: `execute(ExecuteCommand) -> TaskSession`, feedback receiver, result receiver, cancellation handle.

- [ ] **Step 1: Write ignored success integration tests**

Add two `#[tokio::test(flavor = "multi_thread")]` tests marked `#[ignore = "requires running mock_exec_layer"]`.

The hold test must execute:

```rust
let session = client.execute(ExecuteCommand {
    task_id: "rust-hold".into(), device_id: "mock_exec".into(),
    primitive: PrimitiveCommand::Hold, deadline_unix_ms: None,
}).await.unwrap();
let TaskSession { mut feedback, result, .. } = session;
assert_eq!(feedback.recv().await.unwrap().unwrap().details, PrimitiveDetails::Hold);
assert_eq!(result.await.unwrap().unwrap().final_state, FinalState::Succeeded);
```

The `go_to_tag(7)` test must consume feedback through completion and assert the final details equal `GoToTag { current_tag: Some(7), next_tag: None }` and the result succeeded.

- [ ] **Step 2: Create integration runner and verify RED**

Create executable `test/run_mock_exec_integration.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail
source "/opt/ros/${ROS_DISTRO}/setup.bash"
source "${ROS_BUILD_ROOT:-/ws}/install/setup.bash"
ros2 run mock_exec_layer mock_exec_layer_node \
  --ros-args -p step_delay_s:=0.05 -p fail_target_tag:=42 \
  > /tmp/ros-task-client-mock.log 2>&1 &
mock_pid=$!
trap 'kill "$mock_pid" 2>/dev/null || true; wait "$mock_pid" 2>/dev/null || true' EXIT

deadline=$((SECONDS + 10))
until ros2 action list | grep -Fxq '/mock_exec/execute_task'; do
  if (( SECONDS >= deadline )); then cat /tmp/ros-task-client-mock.log >&2; exit 1; fi
  sleep 0.1
done

cargo test --locked \
  --manifest-path ros2_ws/src/control_plane/ros_task_client/Cargo.toml \
  --test mock_exec -- --ignored --test-threads=1
```

Expected: FAIL because `execute` and `TaskSession` are absent.

- [ ] **Step 3: Define session/cancel types**

Add:

```rust
pub struct TaskSession {
    pub task_id: String,
    pub feedback: mpsc::Receiver<Result<TaskFeedback, RosTaskError>>,
    pub result: oneshot::Receiver<Result<TaskResult, RosTaskError>>,
    pub cancellation: CancellationHandle,
}

#[derive(Clone)]
pub struct CancellationHandle { request_tx: mpsc::Sender<CancelRequest> }
impl CancellationHandle {
    pub async fn cancel(&self) -> Result<(), RosTaskError>;
}
```

Keep `CancelRequest` private and give each request a one-shot response sender.

- [ ] **Step 4: Implement availability and acceptance**

`execute` must validate/map before transport, look up device, reject shutdown, poll `server_is_available()` every 25ms until timeout, call `try_request_goal`, await acceptance with timeout, and return `GoalRejected` on `None`. Return `TaskSession` only after acceptance.

- [ ] **Step 5: Implement Tokio event relay**

Destructure private `GoalClient` fields. Spawn a Tokio relay using
`tokio::select!` over feedback, result, cancel requests, and the runtime
shutdown watch receiver. Map before forwarding; send mapping failures as the
terminal result error; forward feedback through the bounded channel; resolve
result once and exit. On shutdown, resolve the result with
`RosTaskError::Shutdown` before dropping private ROS handles. Never run ROS
spin in this Tokio task.

- [ ] **Step 6: Verify successful integration and commit**

Run the integration script, then all package tests. Expected: hold and go-to-tag pass and runtime joins.

```bash
git add ros2_ws/src/control_plane/ros_task_client
git commit -m "feat(ros): execute typed tasks through ActionClient"
```

---

### Task 6: Failure, Cancellation, and Relay Error Semantics

**Files:**
- Modify: `ros2_ws/src/control_plane/ros_task_client/tests/mock_exec.rs`
- Modify: `ros2_ws/src/control_plane/ros_task_client/tests/lifecycle.rs`
- Modify: `ros2_ws/src/control_plane/ros_task_client/src/client.rs`
- Modify: `ros2_ws/src/control_plane/ros_task_client/src/error.rs`

**Interfaces:**
- Consumes: Task 5 session/relay.
- Produces: accepted cancellation, failed-result propagation, deterministic shutdown/channel errors.

- [ ] **Step 1: Add failing SDK failure test**

Submit target 42 and assert:

```rust
assert_eq!(result.final_state, FinalState::Failed);
assert_eq!(result.error_code.as_deref(), Some("SDK_ERROR"));
assert_eq!(result.task_id, "rust-failure");
```

Run integration. Expected: FAIL until aborted results are preserved.

- [ ] **Step 2: Preserve failed result**

Pass `GoalStatusCode::Aborted` and raw result to `from_ros_result`. Return typed `Failed` and `SDK_ERROR`; never replace the Exec cause with a transport guess.

- [ ] **Step 3: Add failing cancellation test**

Submit go-to-tag 7, wait for one feedback, call `session.cancellation.cancel().await`, assert `Ok(())`, then await and assert `FinalState::Canceled`. If terminal canceled feedback arrives, assert `FeedbackState::Canceled`.

- [ ] **Step 4: Route cancellation**

In the relay cancellation branch:

```rust
let response = cancellation.cancel().await;
let mapped = match response.code {
    rclrs::CancelResponseCode::Accept => Ok(()),
    code => Err(RosTaskError::CancelRejected {
        task_id: task_id.clone(), reason: format!("{code:?}"),
    }),
};
let _ = request.response.send(mapped);
```

Do not close streams or synthesize a result after cancel acceptance; await the actual terminal result.

- [ ] **Step 5: Add lifecycle edge tests**

In `tests/lifecycle.rs`, prove:

- unknown device returns immediately;
- a configured but absent action server returns `ActionServerUnavailable`
  after the configured timeout;
- execute after shutdown returns `Shutdown`;
- dropping a session does not panic;
- dropping client before explicit shutdown still allows join.

Add a private `client.rs` unit test for `map_cancel_response` proving `Accept`
returns `Ok(())` and `Reject`, `UnknownGoal`, and `GoalTerminated` return
`CancelRejected` with the task ID.

Add an ignored integration test that submits `go_to_tag`, waits for the first
feedback, calls `runtime.shutdown()`, and asserts the outstanding result
receiver resolves to `RosTaskError::Shutdown` instead of hanging.

- [ ] **Step 6: Verify and commit**

Run all unit tests and the integration runner. Expected: success, typed feedback, `SDK_ERROR`, cancellation, and lifecycle cases pass.

```bash
git add ros2_ws/src/control_plane/ros_task_client
git commit -m "feat(ros): support task cancellation and failure results"
```

---

### Task 7: Make Targets, Repository Verification, and Documentation

**Files:**
- Modify: `Makefile:21-78`
- Modify: `docs/guide/ros-mocks.md`
- Modify: `ros2_ws/src/control_plane/ros_task_client/test/run_mock_exec_integration.sh`

**Interfaces:**
- Consumes: complete package.
- Produces: `make ros-task-test`, `make ros-task-integration`, user workflow.

- [ ] **Step 1: Verify targets are absent**

Run `make -n ros-task-test` and `make -n ros-task-integration`.

Expected: both fail with “No rule to make target”.

- [ ] **Step 2: Add Make targets**

Add variables and container-aware targets:

```make
ROS_TASK_MANIFEST := ros2_ws/src/control_plane/ros_task_client/Cargo.toml
ROS_TASK_INTEGRATION := ros2_ws/src/control_plane/ros_task_client/test/run_mock_exec_integration.sh

ros-task-test: build
ifeq ($(IN_CONTAINER),1)
	source /opt/ros/$(ROS_DISTRO)/setup.bash && source $(ROS_BUILD_ROOT)/install/setup.bash && cargo test --locked --manifest-path $(ROS_TASK_MANIFEST)
else
	$(DEV) make ros-task-test
endif

ros-task-integration: build
ifeq ($(IN_CONTAINER),1)
	ROS_BUILD_ROOT=$(ROS_BUILD_ROOT) $(ROS_TASK_INTEGRATION)
else
	$(DEV) make ros-task-integration
endif
```

Add them to `.PHONY` and help. Keep real integration explicit; do not start background nodes during every lint-only run.

- [ ] **Step 3: Verify targets**

Run inside the container:

```bash
make ros-task-test
make ros-task-integration
```

Expected: unit and real ROS integration pass.

- [ ] **Step 4: Update guide**

Document generator installation, package modules, typed mapper boundary, dedicated executor versus Tokio relays, static endpoint mapping, exact Make commands, and the fact that `services/` remain unmoved until acceptance.

- [ ] **Step 5: Run full verification**

```bash
docker compose -f docker/dev/compose.yaml run --rm dev make check
docker compose -f docker/dev/compose.yaml run --rm dev make ros-task-integration
git diff --check
```

Expected: zero failures.

- [ ] **Step 6: Commit**

```bash
git add Makefile docs/guide/ros-mocks.md ros2_ws/src/control_plane/ros_task_client/test
git commit -m "test(ros): verify Rust task client against mock Exec"
```

---

### Task 8: Humble/Jazzy Matrix and Services-Migration Gate

**Files:**
- Modify only on demonstrated incompatibility: `docker/dev/Dockerfile`, connector `Cargo.toml`/`Cargo.lock`, `docs/guide/ros-mocks.md`
- Verify: spec and this plan

**Interfaces:**
- Consumes: completed connector/container workflow.
- Produces: evidence allowing services migration to start.

- [ ] **Step 1: Verify Jazzy**

```bash
ROS_DISTRO=jazzy UBUNTU_VERSION=24.04 docker compose -f docker/dev/compose.yaml build dev
ROS_DISTRO=jazzy UBUNTU_VERSION=24.04 docker compose -f docker/dev/compose.yaml run --rm dev make check
ROS_DISTRO=jazzy UBUNTU_VERSION=24.04 docker compose -f docker/dev/compose.yaml run --rm dev make ros-task-integration
```

Expected: all exit zero.

- [ ] **Step 2: Verify Humble**

```bash
ROS_DISTRO=humble UBUNTU_VERSION=22.04 docker compose -f docker/dev/compose.yaml build dev
ROS_DISTRO=humble UBUNTU_VERSION=22.04 docker compose -f docker/dev/compose.yaml run --rm dev make check
ROS_DISTRO=humble UBUNTU_VERSION=22.04 docker compose -f docker/dev/compose.yaml run --rm dev make ros-task-integration
```

Expected: all exit zero. Keep one source-compatible lockfile; do not branch APIs by distro without proven incompatibility.

- [ ] **Step 3: Verify scope mechanically**

Run:

```bash
git diff --name-only 5787a10..HEAD
```

Implementation changes must be limited to root build/container/docs files and `ros2_ws/src/control_plane/ros_task_client/**`. No `services/` or `webui/` path may appear.

- [ ] **Step 4: Commit compatibility fixes only if files changed**

```bash
git add docker/dev/Dockerfile ros2_ws/src/control_plane/ros_task_client/Cargo.toml \
  ros2_ws/src/control_plane/ros_task_client/Cargo.lock docs/guide/ros-mocks.md
git commit -m "fix(ros): support task client on Humble and Jazzy"
```

Do not create an empty commit.

- [ ] **Step 5: Confirm migration gate with fresh evidence**

```text
[ ] generated ExecuteTask Rust action types build
[ ] typed goal/feedback/result mapper tests pass
[ ] dedicated executor repeatedly starts and joins
[ ] hold succeeds against Python mock
[ ] go_to_tag succeeds with typed feedback
[ ] SDK_ERROR is preserved as a failed result
[ ] cancellation is accepted and later resolves canceled
[ ] malformed remote data is reported as Mapping
[ ] Humble and Jazzy checks pass
[ ] no existing services/ crate changed
```
