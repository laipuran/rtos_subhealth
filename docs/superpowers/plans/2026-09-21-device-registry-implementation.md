# Device Registry Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Gateway's single hard-coded ROS device with a static YAML device registry owned and loaded by `ros_task_client`.

**Architecture:** `gateway/src/main.rs` only initializes `Execution`; it does not read configuration. `Execution::init()` delegates startup to `ros_task_client`, which loads YAML, validates device entries, creates internal handlers, and resolves task device IDs. Orchestrator remains ROS-agnostic and validates a task through `ExecutionPort` before persisting it.

**Tech Stack:** Rust 1.85, serde, serde_yaml, Tokio, rclrs, ROS 2 ExecuteTask action.

**Spec:** `docs/superpowers/specs/2026-09-21-device-registry-design.md`

## Global Constraints

- Use one static YAML configuration format; do not add JSON configuration support.
- Gateway and Orchestrator must not depend on `DeviceRegistry`, `DeviceHandler`, ROS node types, or ROS action clients.
- Replace the old single-device `Execution::start(device_id, action_name)` path; do not retain a compatibility path.
- `Execution::init()` and the ROS client initialization path are the only startup paths.
- Keep `DeviceId` and `Task.device_id` in the platform domain model.
- Reject unknown devices before a task is stored as `accepted`.
- Do not add new test files; update existing configuration checks only when required by the interface replacement, and use existing build/check commands for validation.
- Follow repository visibility rules: use `pub` or private visibility only.

---

### Task 1: Define and load the static YAML registry in ros_task_client

**Files:**
- Modify: `ros2_ws/src/control_plane/ros_task_client/Cargo.toml`
- Modify: `ros2_ws/src/control_plane/ros_task_client/src/config.rs`
- Modify: `ros2_ws/src/control_plane/ros_task_client/src/lib.rs`
- Modify: `ros2_ws/src/control_plane/ros_task_client/src/error.rs`
- Modify: `ros2_ws/src/control_plane/ros_task_client/tests/config.rs`
- Create: `ros2_ws/config/devices.yaml`

**Interfaces:**

```rust
pub struct RosTaskClientConfig {
    pub version: u32,
    pub ros: RosRuntimeConfig,
    pub devices: Vec<DeviceConfig>,
}

pub struct RosRuntimeConfig {
    pub node_name: String,
    pub feedback_buffer: usize,
    pub server_wait_timeout_ms: u64,
}

pub struct DeviceConfig {
    pub id: String,
    pub action_name: String,
    pub enabled: bool,
}

impl RosTaskClientConfig {
    pub fn from_environment() -> Result<Self, RosTaskError>;
    pub fn from_path(path: &Path) -> Result<Self, RosTaskError>;
    pub fn validate(&self) -> Result<(), RosTaskError>;
}
```

- [ ] Add `serde_yaml` and retain `serde` derive support.
- [ ] Deserialize the exact YAML shape from the design spec.
- [ ] Read the path from `ROS_TASK_CLIENT_CONFIG`; return a configuration error when it is missing, unreadable, malformed, or invalid.
- [ ] Validate version `1`, non-empty names, positive buffer and timeout, enabled device IDs, canonical absolute action names, duplicate device IDs, and duplicate action names.
- [ ] Remove the old `RosConnectionConfig` and `ExecEndpointConfig` public configuration model rather than maintaining both models.
- [ ] Update existing configuration checks to exercise the replacement config model without adding a new test file.
- [ ] Add the repository sample configuration with `mock_exec` and `/mock_exec/execute_task`.

---

### Task 2: Build the internal DeviceRegistry and handlers

**Files:**
- Modify: `ros2_ws/src/control_plane/ros_task_client/src/client.rs`
- Modify: `ros2_ws/src/control_plane/ros_task_client/src/lib.rs`

**Interfaces:**

```rust
pub struct RosTaskClient;

impl RosTaskClient {
    pub fn init() -> Result<(Self, RosTaskRuntime), RosTaskError>;
    pub async fn validate(&self, task: &Task) -> Result<(), RosTaskError>;
    pub async fn execute(&self, task: Task) -> Result<ExecutionSession, ExecutionError>;
}
```

- [ ] Add private `DeviceHandler` and private `DeviceRegistry` types inside the ROS client module or a focused child module.
- [ ] Construct one `ActionClient<ExecuteTask>` for every enabled `DeviceConfig`.
- [ ] Store handlers by device ID and preserve action names for unavailable-server errors.
- [ ] Make `RosTaskClient::init()` load `RosTaskClientConfig::from_environment()`, create the node and runtime, and build the registry.
- [ ] Make `validate` check registry membership without waiting for an action server; return the existing unknown-device error for missing IDs.
- [ ] Make `execute` resolve the task through the registry before mapping and submitting the ROS goal.
- [ ] Remove direct use of the old endpoint list in `ClientState`.

---

### Task 3: Move startup ownership into Execution

**Files:**
- Modify: `ros2_ws/src/services/execution/src/lib.rs`
- Modify: `ros2_ws/src/services/gateway/src/main.rs`
- Modify: `ros2_ws/src/services/gateway/Cargo.toml` only if the replacement removes an unused dependency.

**Interfaces:**

```rust
impl Execution {
    pub fn init() -> Result<Self, ExecutionError>;
    pub fn shutdown(&self) -> Result<(), ExecutionError>;
}
```

- [ ] Replace `Execution::start(device_id, action_name)` with parameterless `Execution::init()`.
- [ ] Have `Execution::init()` call `RosTaskClient::init()` and translate client initialization errors into `ExecutionError`.
- [ ] Remove `DeviceId` from the execution startup signature and remove the Gateway environment variables `GATEWAY_EXECUTION_DEVICE_ID` and `GATEWAY_EXECUTION_ACTION_NAME`.
- [ ] Change `gateway/src/main.rs` to call only `Execution::init()`; it must not read `ROS_TASK_CLIENT_CONFIG` or parse YAML.
- [ ] Keep the existing `ExecutionPort` implementation and shutdown behavior.

---

### Task 4: Validate devices before repository acceptance

**Files:**
- Modify: `ros2_ws/src/services/orchestration/src/lib.rs`
- Modify: `ros2_ws/src/services/orchestration/src/error.rs`
- Modify: `ros2_ws/src/services/execution/src/lib.rs`
- Modify: `ros2_ws/src/control_plane/ros_task_client/src/client.rs`

**Interfaces:**

```rust
pub trait ExecutionPort: Send + Sync {
    fn validate(
        &self,
        task: &Task,
    ) -> Pin<Box<dyn Future<Output = Result<(), ExecutionError>> + Send + '_>>;

    fn execute(
        &self,
        task: Task,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionSession, ExecutionError>> + Send + '_>>;
}
```

- [ ] Add `validate` to `ExecutionPort` without exposing Registry or ROS types.
- [ ] Delegate `Execution::validate` to `RosTaskClient::validate`.
- [ ] Call `execution.validate(&task)` at the beginning of `Orchestrator::submit`, before `repository.create_task`.
- [ ] Map validation failures to the existing execution error path without creating a repository record.
- [ ] Keep repository busy-device checks unchanged; the repository still does not own device configuration.
- [ ] Remove any obsolete startup-time device validation or duplicate fallback branch introduced during this change.

---

### Task 5: Update documentation and verify the full migration

**Files:**
- Modify: `README.md` or the existing runtime setup documentation where the Gateway environment variables are documented.
- Modify: `ros2_ws/config/devices.yaml` if the final launch path requires an explicit example.
- Search all `ros2_ws/src` callers of `Execution::start`, `RosConnectionConfig`, and `ExecEndpointConfig`.

- [ ] Replace old environment-variable instructions with `ROS_TASK_CLIENT_CONFIG=/path/to/devices.yaml`.
- [ ] Document that the Gateway only starts Execution and that the ROS client owns registry loading.
- [ ] Confirm no `GATEWAY_EXECUTION_DEVICE_ID`, `GATEWAY_EXECUTION_ACTION_NAME`, `RosConnectionConfig`, or `ExecEndpointConfig` references remain.
- [ ] Confirm no Gateway or Orchestrator source imports ROS client configuration or handler types.
- [ ] Run the existing Rust workspace checks/build commands used by the repository.
- [ ] Run `git diff --check` and confirm the change is limited to the planned layers and configuration files.
