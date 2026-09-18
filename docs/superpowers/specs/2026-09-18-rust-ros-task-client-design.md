# Rust ROS Task Client Design

## 1. Goal

Add the Rust-to-ROS task connection package that the future Server will use to
call Exec through `task_interfaces/action/ExecuteTask`.

This work ends when a Rust client proves the following flows against the Python
mock Exec:

- successful `hold`;
- successful `go_to_tag` with typed feedback;
- failed `go_to_tag` with `SDK_ERROR`;
- accepted cancellation followed by a canceled result;
- invalid JSON in ROS feedback/result handling produces a mapping error;
- the ROS executor shuts down and its thread is joined.

Passing these checks is the prerequisite for moving the existing Rust services
into `ros2_ws`. Moving or redesigning those services is not part of this work.

## 2. Scope Boundary

Create one ROS Rust library package:

```text
ros2_ws/src/control_plane/ros_task_client/
```

The package owns:

- `rclrs` context, node, executor, and ActionClients;
- the dedicated executor OS thread;
- typed connection-boundary task types;
- conversion between boundary types and generated ROS types;
- JSON serialization for primitive payloads;
- JSON deserialization for primitive feedback details;
- goal acceptance, feedback, result, and cancellation transport;
- endpoint lookup by `device_id`;
- connection-specific errors and shutdown.

The package does not own:

- HTTP or WebSocket handling;
- task ID generation;
- device selection or capability matching;
- DeviceRegistry storage or discovery;
- task lifecycle persistence;
- Server or Orchestrator state machines;
- configuration-file, environment-variable, or ROS-parameter parsing;
- dynamic device discovery;
- Sensor topics;
- movement or SDK behavior.

The following existing directories are not moved or behaviorally changed:

```text
services/platform/
services/gateway/
services/orchestration/
services/execution/
webui/
```

## 3. Package Layout

```text
ros2_ws/src/control_plane/ros_task_client/
├── package.xml
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── client.rs
│   ├── config.rs
│   ├── error.rs
│   ├── mapper.rs
│   ├── runtime.rs
│   └── types.rs
├── tests/
│   ├── config.rs
│   ├── mapper.rs
│   └── mock_exec.rs
└── test/
    └── run_mock_exec_integration.sh
```

Responsibilities:

- `lib.rs`: export only the Server-facing API;
- `types.rs`: typed commands, feedback, results, sessions, and cancellation;
- `mapper.rs`: all generated-ROS-type and JSON conversion;
- `config.rs`: typed startup configuration and validation;
- `client.rs`: async submit/cancel API and Action event routing;
- `runtime.rs`: context/node/executor construction, executor thread, shutdown;
- `error.rs`: stable connection-layer error categories.

Generated ROS types and raw `rclrs` goal objects are private implementation
details. No public function accepts or returns them.

## 4. Public Contract

The public command is strongly typed:

```rust
pub struct ExecuteCommand {
    pub task_id: String,
    pub device_id: String,
    pub primitive: PrimitiveCommand,
    pub deadline_unix_ms: Option<i64>,
}

pub enum PrimitiveCommand {
    Hold,
    GoToTag { target_tag: i32 },
}
```

The connector produces typed feedback and result values:

```rust
pub struct TaskFeedback {
    pub task_id: String,
    pub state: FeedbackState,
    pub progress: f32,
    pub phase: String,
    pub details: PrimitiveDetails,
    pub timestamp: RosTimestamp,
}

pub enum PrimitiveDetails {
    Hold,
    GoToTag {
        current_tag: Option<i32>,
        next_tag: Option<i32>,
    },
}

pub struct TaskResult {
    pub task_id: String,
    pub final_state: FinalState,
    pub error_code: Option<String>,
    pub message: String,
    pub finished_time: RosTimestamp,
}
```

`RosTimestamp` is a connection-boundary value containing signed seconds and
unsigned nanoseconds. The package does not impose the Server's eventual time
storage type.

`-1` in `current_tag` or `next_tag` maps to `None`. A non-negative tag maps to
`Some(tag)`. A tag smaller than `-1` is rejected as malformed feedback.

The async API has this shape:

```rust
let (client, runtime) = RosTaskClient::start(config)?;
let session = client.execute(command).await?; // returns only after goal accept

while let Some(feedback) = session.feedback.recv().await {
    // Server-owned state update
}

session.cancel().await?;          // returns after ROS accepts cancellation
let result = session.result.await?;

runtime.shutdown()?;              // stops executor and joins its OS thread
```

Exact ownership syntax may be adjusted to satisfy Rust's single-consumer rules,
but the observable boundary remains:

- `execute` resolves only after action-server availability and goal acceptance;
- feedback is a bounded asynchronous stream;
- result is a one-shot completion;
- cancellation has its own accepted/rejected response;
- the Server never handles an `rclrs` or generated ROS handle.

No `ExecutionPort` trait or transport-adapter trait is introduced. The future
Orchestrator calls the concrete `RosTaskClient`.

## 5. Mapping Rules

Generated `task_interfaces/action/ExecuteTask` types are confined to the private
transport implementation in `client.rs` and `mapper.rs`. `mapper.rs` owns every
field-level and JSON conversion; `client.rs` only supplies mapped values to the
typed `rclrs` ActionClient and routes its events. No generated type is publicly
exported.

### Goal mapping

| Boundary value | ROS value |
|---|---|
| `task_id` | `task_id` unchanged |
| `device_id` | `device_id` unchanged |
| `Hold` | `primitive="hold"`, `payload_json="{}"` |
| `GoToTag { target_tag }` | `primitive="go_to_tag"`, compact JSON payload |
| `None` deadline | `deadline_unix_ms=0` |
| `Some(ms)` deadline | positive absolute Unix milliseconds |

An empty task/device ID or non-positive explicit deadline is rejected before a
ROS goal is sent.

### Feedback mapping

- `task_id` must equal the submitted command's task ID;
- state must be `running` or `canceled`;
- progress must be finite and inside `[0.0, 1.0]`;
- details JSON must match the submitted primitive;
- `hold` details must be `{}`;
- `go_to_tag` details contain exactly `current_tag` and `next_tag`;
- timestamps must have `nanosec < 1_000_000_000`.

### Result mapping

- `task_id` must equal the submitted command's task ID;
- final state must be `succeeded`, `failed`, or `canceled`;
- successful/canceled results normalize an empty error code to `None`;
- a failed result preserves Exec's error code and message;
- unknown state strings and malformed timestamps are mapping errors.

Transport errors and malformed remote data are not converted into guessed
business failures. They are returned as connection errors for the Server to
interpret.

## 6. Runtime and Concurrency

`RosTaskRuntime` owns the ROS context, node, executor commands, and executor
thread. The executor runs on a dedicated `std::thread`, not `tokio::spawn` and
not `tokio::task::spawn_blocking`.

```text
Tokio runtime
├── future Server/Gateway/Orchestrator
├── RosTaskClient async methods
└── Tokio mpsc/oneshot endpoints
                 ↕
dedicated OS thread
└── rclrs executor + node + ActionClients
```

The dedicated thread only calls blocking `executor.spin(...)`. `rclrs`
ActionClient objects are reference-counted and their goal/feedback/result
handles are async futures and streams, so `RosTaskClient::execute` awaits them
from the caller's Tokio runtime while the ROS thread services the underlying
wait set. After acceptance, a Tokio relay task consumes the private `rclrs`
goal stream, maps its events, and forwards typed values to bounded channels.
No blocking ROS spin runs on a Tokio worker, and the ROS thread never blocks
waiting for a Tokio receiver.

Each accepted goal owns:

- one bounded feedback sender;
- one result sender;
- a private `rclrs` cancellation handle;
- a cancellation request receiver exposed through a typed public handle.

The public cancel operation resolves only after ROS returns its cancellation
response. Accepting cancellation does not synthesize a canceled result; the
result future resolves only when Exec reports the terminal action result.

Shutdown is explicit and idempotent:

1. stop accepting new commands;
2. signal the ROS context/executor to stop;
3. join the executor thread;
4. report executor or thread-panic errors;
5. close outstanding streams/futures with `Shutdown` errors.

`Drop` performs best-effort cleanup but is not the primary shutdown API.

## 7. Configuration and Future Discovery

The connection package receives configuration as Rust values:

```rust
pub struct RosConnectionConfig {
    pub node_name: String,
    pub endpoints: Vec<ExecEndpointConfig>,
    pub feedback_buffer: usize,
    pub server_wait_timeout: Duration,
}

pub struct ExecEndpointConfig {
    pub device_id: String,
    pub action_name: String,
}
```

The Server composition root will later decide whether these values come from
YAML, environment variables, ROS parameters, or another source.

Startup rejects:

- an empty node name;
- duplicate or empty device IDs;
- duplicate or empty action names;
- zero feedback capacity;
- zero server wait timeout.

The first implementation treats the endpoint set as immutable. Internally,
ActionClients are keyed by `device_id`; no code assumes a single mock endpoint
or derives an action name from a device type.

Future dynamic discovery can update DeviceRegistry and then call a future
register/remove endpoint API. ROS graph discovery alone is not sufficient
because it does not describe device IDs, capabilities, or supported
primitives. Dynamic registration is deliberately outside this iteration.

## 8. Error Contract

The public error enum distinguishes:

- `InvalidConfig`;
- `InvalidCommand`;
- `UnknownDevice`;
- `ActionServerUnavailable`;
- `GoalRejected`;
- `CancelRejected`;
- `Mapping`;
- `Ros`;
- `ChannelClosed`;
- `Shutdown`;
- `ExecutorPanicked`.

Errors retain a source where available. They do not contain HTTP status codes or
mutate Server task state.

## 9. Rust ROS Dependencies

The current development image contains colcon Cargo plugins but does not contain
the Rust ROS interface generator. The implementation therefore adds the
ros2-rust package repository matching the selected image and installs:

```text
ros-${ROS_DISTRO}-rosidl-generator-rs
```

Repository branches are:

- Ubuntu Jammy + ROS Humble: `jammy-humble`;
- Ubuntu Noble + ROS Jazzy: `noble-jazzy`.

The connector uses Cargo releases compatible with the current ros2-rust
examples:

```text
rclrs 0.7
ros-env 0.2
rosidl_runtime_rs 0.6
```

Cargo dependency resolution is committed in `Cargo.lock`. Third-party ros2-rust
source trees are not vendored into this repository.

The package is an `ament_cargo` package and declares ROS dependencies on
`rclrs`, `rosidl_runtime_rs`, and `task_interfaces` in `package.xml`.

Because the connector can only build in a sourced ROS environment with generated
Rust interfaces, it is built by colcon and excluded from the current top-level
pure Cargo workspace. The later Server move may replace this temporary split
with one ROS-owned Rust workspace.

## 10. Repository Changes

Create:

```text
ros2_ws/src/control_plane/ros_task_client/package.xml
ros2_ws/src/control_plane/ros_task_client/Cargo.toml
ros2_ws/src/control_plane/ros_task_client/Cargo.lock
ros2_ws/src/control_plane/ros_task_client/src/lib.rs
ros2_ws/src/control_plane/ros_task_client/src/client.rs
ros2_ws/src/control_plane/ros_task_client/src/config.rs
ros2_ws/src/control_plane/ros_task_client/src/error.rs
ros2_ws/src/control_plane/ros_task_client/src/mapper.rs
ros2_ws/src/control_plane/ros_task_client/src/runtime.rs
ros2_ws/src/control_plane/ros_task_client/src/types.rs
ros2_ws/src/control_plane/ros_task_client/tests/config.rs
ros2_ws/src/control_plane/ros_task_client/tests/mapper.rs
ros2_ws/src/control_plane/ros_task_client/tests/mock_exec.rs
ros2_ws/src/control_plane/ros_task_client/test/run_mock_exec_integration.sh
```

Modify:

```text
Cargo.toml
docker/dev/Dockerfile
Makefile
docs/guide/ros-mocks.md
```

Modify `task_interfaces/package.xml` only if the installed generator requires an
explicit package dependency. Do not add speculative dependencies.

## 11. Testing and Acceptance

### Unit tests

- configuration validation and endpoint lookup;
- every command-to-goal mapping;
- every feedback/details mapping;
- every result mapping;
- malformed JSON, mismatched task ID, unknown states, non-finite/out-of-range
  progress, and invalid timestamps;
- channel closure and idempotent shutdown behavior that does not require a
  remote action server.

### ROS integration

The integration runner starts `mock_exec_layer`, runs the Rust integration test,
and always terminates the child process. It verifies:

1. `hold` accepted, feedback received, succeeded result received;
2. `go_to_tag(7)` produces typed details and succeeds;
3. configured `fail_target_tag=42` produces failed/`SDK_ERROR`;
4. cancellation is accepted and the later result is canceled;
5. `shutdown()` returns and joins the executor thread.

### Build matrix

The package and generated `ExecuteTask` Rust types must build in both supported
development images:

- Ubuntu 22.04 / ROS Humble;
- Ubuntu 24.04 / ROS Jazzy.

The existing Python mock and PhysioSample packages must continue to pass their
tests. The top-level `make check` must include the Rust ROS package tests when
run inside the ROS container.

## 12. Handoff to Server Migration

After acceptance, the Server migration may begin. Its only required dependency
on this work is the public API exported by `ros_task_client`:

- typed startup config;
- cloneable client;
- typed execute command;
- accepted goal session;
- feedback stream;
- cancellation handle;
- result future;
- explicit runtime shutdown.

The Server remains free to choose its own task entities, state machine,
persistence, DeviceRegistry, HTTP DTOs, and composition structure. It converts
its own types at the `ros_task_client` call site and never needs generated ROS
types or `rclrs` handles.
