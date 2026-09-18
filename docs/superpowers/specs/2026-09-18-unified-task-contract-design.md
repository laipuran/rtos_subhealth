# Unified Task Contract Design

## 1. Goal

Define one task contract that can carry the first complete user workflow:

```text
WebUI → HTTP Gateway → Orchestrator → ROS Action → Exec → native SDK
                                             ↘ feedback/result/cancel ↗
```

The first deliverable supports:

- `hold`;
- `go_to_tag`;
- successful execution;
- cancellation;
- execution failure.

Sensor publication remains independent and does not participate in task
decisions in this iteration.

## 2. Architecture

The whole server runs in the ROS workspace and ROS environment. Gateway and
Orchestration are composed into one server process. Orchestration directly owns
the ROS ActionClient used to call Exec; no ExecutionPort or transport-adapter
layer is introduced at this stage.

```text
ros2_ws/src/
├── interfaces/
│   ├── task_interfaces/
│   └── physio_interfaces/
├── control_plane/
│   └── server/
│       ├── gateway
│       └── orchestration
└── endpoints/
    └── mock_exec_layer/
```

The logical responsibilities remain separate even though they share one
process:

- Gateway owns HTTP/WS transport;
- Orchestration owns task IDs, device selection, lifecycle state, and ROS goal
  handles;
- Exec owns execution, cancellation, native SDK access, feedback, and terminal
  results.

## 3. Contract Strategy

All boundaries use the same semantic envelope:

```text
task_id
device_id
primitive
payload
deadline_unix_ms
```

`payload` and feedback `details` have primitive-specific JSON schemas. HTTP
uses JSON objects. ROS carries their compact JSON serialization in string
fields. Rust and TypeScript expose discriminated types where practical.

This is a semantic contract, not a requirement that TypeScript, Rust, ROS, and
Python use the same generated language type. Mechanical conversion remains,
but no boundary may rename fields or redefine their meaning.

## 4. Primitive Contracts

### 4.1 `hold`

Payload:

```json
{}
```

Feedback details:

```json
{}
```

### 4.2 `go_to_tag`

Payload:

```json
{
  "target_tag": 42
}
```

Constraints:

- `target_tag` is a required signed 32-bit integer;
- unknown fields are rejected;
- device capability selection requires `tag_navigation`;
- the selected device must declare primitive `go_to_tag`.

Feedback details:

```json
{
  "current_tag": 1,
  "next_tag": 42
}
```

`current_tag` and `next_tag` use `-1` when unknown or absent. A terminal
feedback sets `next_tag` to `-1`.

## 5. HTTP Contract

### 5.1 Create task

`POST /api/v1/tasks`

Common request fields:

| Field | Type | Required | Meaning |
|---|---|---:|---|
| `device_id` | string | no | Explicit device; omitted for capability selection |
| `required_capabilities` | string[] | no | Additional selection requirements |
| `primitive` | string | yes | `hold` or `go_to_tag` |
| `payload` | object | yes | Primitive-specific payload |
| `deadline_unix_ms` | integer | no | Absolute Unix epoch milliseconds |

Examples:

```json
{"primitive":"hold","payload":{}}
```

```json
{
  "required_capabilities":["tag_navigation"],
  "primitive":"go_to_tag",
  "payload":{"target_tag":42},
  "deadline_unix_ms":1789700000000
}
```

Accepted response uses HTTP `202`:

```json
{"task_id":"task-1","state":"accepted"}
```

The request waits for device selection, ROS action-server availability, and
goal acceptance before returning `202`. The task is inserted into the
Orchestrator's active state before sending the goal so an immediate feedback
callback has a destination. If transport or goal acceptance fails, that
provisional record is removed and no queryable task is created. An immediate
feedback event may race the HTTP response; WebSocket consumers therefore
upsert complete snapshots by `task_id` rather than requiring prior list state.

Creation errors:

| HTTP | Code | Meaning |
|---:|---|---|
| 400 | `INVALID_REQUEST` | Envelope or primitive payload is invalid |
| 409 | `DEVICE_BUSY` | Selected device already owns an active task |
| 422 | `NO_DEVICE` | No device satisfies primitive/capabilities |
| 502 | `GOAL_REJECTED` | Exec rejected the goal |
| 503 | `EXEC_UNAVAILABLE` | ROS action server cannot be reached |

### 5.2 Task view

```json
{
  "task_id":"task-1",
  "device_id":"mock_exec",
  "primitive":"go_to_tag",
  "payload":{"target_tag":42},
  "deadline_unix_ms":null,
  "state":"running",
  "progress":0.5,
  "phase":"moving_to_tag",
  "details":{"current_tag":1,"next_tag":42},
  "error_code":null,
  "message":"",
  "created_at_ms":1789690000000,
  "updated_at_ms":1789690001000
}
```

Task states are:

```text
accepted
running
canceling
succeeded
failed
canceled
```

Allowed transitions:

```text
accepted → running → succeeded
accepted → failed
accepted → canceling → canceled
running  → failed
running  → canceling → canceled
```

Terminal states never transition again.

### 5.3 List tasks

`GET /api/v1/tasks?offset=0&limit=50`

```json
{"tasks":[],"total":0,"offset":0,"limit":50}
```

### 5.4 Cancel task

`POST /api/v1/tasks/{task_id}/cancel`

After ROS accepts cancellation, respond with HTTP `202`:

```json
{"task_id":"task-1","state":"canceling"}
```

Unknown tasks return `404`. Terminal tasks and rejected cancellation return
`409` with the current task state.

## 6. WebSocket Contract

`GET /api/v1/events` sends complete task snapshots rather than patches:

```json
{
  "sequence":12,
  "event":"task_updated",
  "task":{...TaskView}
}
```

The WebUI replaces the record matching `task_id`. It does not independently
reconstruct state transitions from partial feedback fields.

## 7. ROS Action Contract

Replace the legacy RFC action with `task_interfaces/action/ExecuteTask.action`:

```text
# Goal
string task_id
string device_id
string primitive
string payload_json
int64 deadline_unix_ms
---
# Result
string task_id
string final_state
string error_code
string message
builtin_interfaces/Time finished_time
---
# Feedback
string task_id
string state
float32 progress
string phase
string details_json
builtin_interfaces/Time timestamp
```

Rules:

- `deadline_unix_ms=0` means no deadline;
- feedback progress is finite and clamped to `[0.0, 1.0]`;
- feedback state is `running` during normal execution and `canceled` for the
  terminal cancellation feedback;
- result state is exactly `succeeded`, `failed`, or `canceled`;
- `error_code` is empty for successful and user-canceled execution;
- every feedback/result `task_id` must equal the goal `task_id`;
- malformed JSON, unknown primitive, or invalid payload rejects the goal;
- native SDK/runtime failures abort an accepted goal with a failed result.

The first configured action endpoint is:

```text
device_id: mock_exec
action_name: /mock_exec/execute_task
capabilities: [tag_navigation]
primitives: [hold, go_to_tag]
```

The action name is deployment configuration, not a field inferred from a
device vendor or type.

## 8. Error Codes

The first closed loop uses:

| Code | Producer | Meaning |
|---|---|---|
| `INVALID_PAYLOAD` | Server or Exec | Primitive payload schema violation |
| `UNSUPPORTED_PRIMITIVE` | Server or Exec | Device does not support primitive |
| `NO_DEVICE` | Orchestrator | No matching device |
| `DEVICE_BUSY` | Orchestrator | Device already executing another task |
| `EXEC_UNAVAILABLE` | Orchestrator | ROS action server unavailable |
| `GOAL_REJECTED` | Orchestrator | Exec rejected a validly transported goal |
| `DEADLINE_EXCEEDED` | Exec | Absolute deadline elapsed |
| `SDK_ERROR` | Exec | Native backend operation failed |
| `INTERNAL` | Producing component | Unclassified internal failure |

Mock execution failure is configured on the mock node, for example
`fail_target_tag=42`; it is not added to task payload and therefore does not
pollute the production contract.

## 9. Sequence Validation

### 9.1 Successful `hold`

```text
User       WebUI       Server/Orch       Exec
 |           |              |              |
 | submit    |              |              |
 |---------->| POST /tasks  |              |
 |           |------------->| create ID    |
 |           |              | select device|
 |           |              | ROS goal     |
 |           |              |------------->|
 |           | 202 accepted | goal accepted|
 |           |<-------------|<-------------|
 |           |              | feedback     |
 |           | WS running   |<-------------|
 |           |<-------------|              |
 |           |              | result       |
 |           | WS succeeded |<-------------|
 |           |<-------------| release device
```

Every consumed value has one producer:

- WebUI produces primitive/payload;
- Server produces task ID and timestamps;
- Orchestrator produces selected device;
- Exec produces progress, phase, details, and terminal result.

### 9.2 Canceled `go_to_tag`

```text
User       WebUI       Server/Orch       Exec
 | submit tag 42            |              |
 |------------------------->| ROS goal     |
 |                          |------------->|
 |                          |<-- feedback--|
 |<--------- WS running ----|              |
 | cancel                   |              |
 |------------------------->| cancel goal  |
 |                          |------------->|
 |                          | cancel accept|
 |<-------- canceling ------|<-------------|
 |                          | canceled     |
 |<-------- canceled -------|<-------------|
 |                          | release device
```

Orchestration retains the ROS goal handle by `task_id`. It enters
`canceling` only after ROS accepts cancellation and enters `canceled` only
after the action result confirms it.

### 9.3 Failed `go_to_tag`

```text
User       WebUI       Server/Orch       Exec       SDK/mock
 | go_to_tag(42)            |              |           |
 |------------------------->| ROS goal     |           |
 |                          |------------->| execute   |
 |                          |              |---------->|
 |                          |<-- feedback--|           |
 |<--------- WS running ----|              | failure   |
 |                          |              |<----------|
 |                          |<-- failed ---|           |
 |<--------- WS failed -----| release device           |
```

Exec is the sole producer of execution error details. Orchestration persists
and forwards them without guessing the cause.

## 10. Interface Migration

The first migration changes only interfaces and the existing mock endpoint:

1. create `task_interfaces/action/ExecuteTask.action`;
2. remove legacy `ros_interfaces/action/ExecTask.action`;
3. remove `ros_interfaces/msg/Constraints.msg`, because constraints move into
   primitive payload schemas;
4. update the mock Exec package to consume `task_interfaces/ExecuteTask`;
5. keep `physio_interfaces/msg/PhysioSample` unchanged;
6. update ROS package metadata, tests, Make verification, and docs;
7. do not connect Gateway/Orchestration in this migration step.

The subsequent server migration will move the complete control-plane process
into `ros2_ws`, implement the ROS ActionClient, and align HTTP/WebSocket DTOs.

## 11. Acceptance Criteria

- active ROS interfaces use the unified envelope and current contract docs
  identify the legacy Rust/WebUI shapes as pending migration;
- generated ROS Python types expose the exact action fields in section 7;
- `hold` succeeds through the migrated mock action;
- `go_to_tag` publishes schema-valid feedback and succeeds;
- cancellation yields canceling/canceled semantics without an invalid ROS state
  transition;
- configured mock execution failure yields `SDK_ERROR` and failed result;
- `PhysioSample` continues to build and publish unchanged;
- no legacy `ExecTask`, `Constraints`, `target_tags`, or `route_id` remains in
  active ROS packages;
- current Rust services are not connected or behaviorally changed in the
  interface migration step.
