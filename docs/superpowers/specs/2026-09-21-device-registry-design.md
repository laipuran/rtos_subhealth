# Device Registry Design

## Goal

Replace the Gateway's single hard-coded execution device and action endpoint with a static YAML-backed device registry owned by the ROS client layer.

## Architectural Decision

`Execution` remains the concrete implementation of `orchestration::ExecutionPort`. `Gateway` and `Orchestrator` do not receive a `DeviceRegistry`, `DeviceHandler`, ROS node, or ROS action client.

The runtime dependency direction is:

```text
Gateway → Orchestrator → ExecutionPort → Execution → RosTaskClient → DeviceRegistry → DeviceHandler → ROS ActionClient
```

`gateway/src/main.rs` is only the composition root. It calls `Execution::init()` and does not read or parse the device configuration. The configuration loader belongs to `ros_task_client`; `Execution::init()` delegates to it.

`Execution::init()` is accepted as the public constructor name for this design because initialization creates the ROS node, action clients, and background runtime. It must remain a single initialization path; no legacy `start(device_id, action_name)` path is retained.

## Configuration

The registry is static and loaded once at startup from YAML. JSON remains reserved for HTTP and event serialization.

```yaml
version: 1

ros:
  node_name: execution
  feedback_buffer: 64
  server_wait_timeout_ms: 5000

devices:
  - id: mock_exec
    action_name: /mock_exec/execute_task
    enabled: true
```

The first version contains only transport and startup fields required by the existing `ExecuteTask` action. It does not add protocol, model, health-check, retry, topic, service, or capability fields before there is a concrete need.

## Responsibilities

### Gateway

- Accept `device_id` as an opaque task field.
- Never load, parse, or query the registry.
- Never depend on ROS client types.

### Orchestrator

- Own task lifecycle and future business-level task decomposition.
- Depend only on `ExecutionPort`.
- Never hold a ROS action client or `DeviceHandler`.

### Execution

- Initialize the concrete execution implementation.
- Delegate configuration loading and ROS client construction.
- Expose only `ExecutionPort` and shutdown behavior to the composition root.

### RosTaskClient

- Load and validate YAML configuration.
- Build one `DeviceHandler` per enabled device.
- Resolve `Task.device_id` to a handler.
- Submit the existing `ExecuteTask` action.
- Keep unknown-device and action-server errors in the execution layer.

### Platform and repository

`DeviceId` and `Task.device_id` remain part of the domain model. The repository continues to enforce per-device task exclusivity but does not know the registry or ROS configuration.

## Unknown Device Behavior

An unknown device must be rejected before a task is reported as `accepted`. The execution abstraction therefore needs a validation capability that does not expose the registry itself. The orchestrator validates the task through `ExecutionPort` before creating the repository record, then executes the validated task.

## Future Task Decomposition

The current `go_to_tag` task is executed directly. If composite primitives are added later, Orchestrator may decompose a domain task into abstract execution commands. Those commands still go through `ExecutionPort`; Orchestrator does not become ROS-aware.
