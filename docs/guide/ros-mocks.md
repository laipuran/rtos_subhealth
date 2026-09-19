# ROS 2 task and sensor mocks

This workspace provides the unified task action and a small sensor surface for
development validation. The current Rust server and WebUI still use legacy task
shapes and are not connected to this action yet; their migration is a separate
step.

## Workspace

```text
ros2_ws/src/
├── interfaces/
│   ├── task_interfaces/
│   │   └── action/ExecuteTask.action
│   └── physio_interfaces/
│       └── msg/PhysioSample.msg
└── mocks/
    ├── mock_exec_layer/          # ExecuteTask action server
    └── physio_mock_publisher/    # PhysioSample topic publisher
```

The active interfaces are:

- `task_interfaces/action/ExecuteTask`, the unified task contract;
- RFC 009 `physio_interfaces/msg/PhysioSample`.

Planning, diagnosis, and AprilTag perception interfaces are deliberately
excluded. The execution mock does not provide AprilTag detection.

The execution package follows the same responsibility-oriented module pattern
as the Rust crates without mirroring Rust filenames:

```text
mock_exec_layer/
├── node.py             # ROS node, ActionServer, message mapping, lifecycle
├── contract.py         # primitive payload parsing and validation
├── execution.py        # deterministic execution and feedback model
└── terminal_state.py   # thread-safe action terminal-state coordination
```

Only `node.py` depends on ROS. The other modules contain independently tested
Python logic.

## Rust task client

The Rust `ros_task_client` package is an `ament_cargo` package at
`ros2_ws/src/control_plane/ros_task_client`. It consumes the generated
`task_interfaces/action/ExecuteTask` types but keeps those generated types and
the JSON wire representation private to `client.rs` and `mapper.rs`. Its public
boundary is typed Rust commands, feedback, results, cancellation, and errors.

The package is split into these responsibility-oriented modules:

```text
ros_task_client/
├── src/client.rs   # ActionClient calls and Tokio relays
├── src/config.rs   # endpoint and connection validation
├── src/error.rs    # typed client, mapping, and ROS errors
├── src/mapper.rs   # typed values <-> generated ROS messages and JSON
├── src/runtime.rs  # dedicated blocking ROS executor lifecycle
├── src/types.rs    # public commands, feedback, results, and sessions
└── tests/          # sourced-environment and mock Exec coverage
```

The development image installs the distro-matched Rust ROS interface
generator, `ros-${ROS_DISTRO}-rosidl-generator-rs`, from the ros2-rust apt
repository. The repository branches are `jammy-humble` for Humble and
`noble-jazzy` for Jazzy; this setup is in `docker/dev/Dockerfile`. A custom
image must provide that package before `make build`, along with `rclrs` and
the generated interfaces.

ROS spinning and Tokio work have separate responsibilities. `RosTaskRuntime`
owns a dedicated OS thread for the blocking `rclrs` executor. Tokio tasks
relay native action feedback, results, and cancellation to the caller; no
Tokio worker spins the ROS executor, and the executor thread does not wait on
Tokio receivers.

Endpoint selection is static: a configured `device_id` maps to one canonical
absolute action name, for example `mock_exec` maps to
`/mock_exec/execute_task`. The client validates this name and requires the
five action graph entities before sending a goal. Action remapping is not
supported by this connector because `rclrs 0.7` does not expose the resolved
ActionClient name; configure the final absolute name instead.

The connector remains separate from the existing server integration. The
`ros2_ws/src/services/` packages and WebUI are not wired to this client;
they remain separate ROS workspace components.

## Interface fields

### `ExecuteTask`

| Section | Fields |
| --- | --- |
| Goal | `string task_id`, `string device_id`, `string primitive`, `string payload_json`, `int64 deadline_unix_ms` |
| Result | `string task_id`, `string final_state`, `string error_code`, `string message`, `builtin_interfaces/Time finished_time` |
| Feedback | `string task_id`, `string state`, `float32 progress`, `string phase`, `string details_json`, `builtin_interfaces/Time timestamp` |

The execution mock accepts `hold` with payload `{}` and `go_to_tag` with payload
`{"target_tag":42}`. Unknown primitives, malformed JSON, unknown payload fields,
and invalid payload types reject the goal. `deadline_unix_ms=0` disables the
deadline; otherwise it is an absolute Unix epoch timestamp in milliseconds.

Normal feedback uses `state=running`. `go_to_tag` reports
`phase=moving_to_tag` and JSON details containing `current_tag` and `next_tag`;
unknown or absent tags use `-1`. `hold` reports `phase=holding` and details
`{}`. Progress is always in `[0.0, 1.0]`. Accepted cancellation emits one
terminal `state=canceled`, `phase=canceled` feedback before returning a
`final_state=canceled` result. A configured target failure aborts with
`final_state=failed` and `error_code=SDK_ERROR`.

### RFC 009: `PhysioSample`

| Field | Type |
| --- | --- |
| `timestamp` | `builtin_interfaces/Time` |
| `data_src` | `string` |
| `data_type` | `string` |
| `data` | `float32` |
| `valid` | `bool` |

The publisher emits reliable, depth-10 samples on `/physio/mock_spo2`,
`/physio/mock_heart_rate`, `/physio/mock_bp_systolic`,
`/physio/mock_bp_diastolic`, `/physio/mock_body_temp`, and
`/physio/mock_respiratory_rate`. Every emitted sample has the current ROS time
and `valid=true`.

## Build and run

Build the selected image on the host and enter its interactive container:

```bash
make jazzy
# inside the container
make build
```

The container build runs `colcon build --merge-install --symlink-install`.
With the default `ROS_BUILD_ROOT=/ws/$ROS_DISTRO`, logs, intermediate files, and
the merged install are written below `/ws/$ROS_DISTRO` (for example,
`/ws/jazzy/log`, `/ws/jazzy/build/merged-symlink`, and
`/ws/jazzy/install`). They are not written below `ros2_ws/`, and the existing
top-level `/ws` data is left untouched. Set `ROS_BUILD_ROOT` to override this
location. `make humble` builds and enters the Humble image instead.

Run the Rust package checks explicitly after the build:

```bash
# inside the sourced development container
make ros-task-test
make ros-task-integration
```

The unit target sources both `/opt/ros/$ROS_DISTRO/setup.bash` and
`$ROS_BUILD_ROOT/install/setup.bash` before its package Cargo test. It uses a temporary
Cargo target by default so generated ROS linkage cannot reuse stale artifacts;
set `ROS_TASK_CARGO_TARGET=/ws/$ROS_DISTRO/target/ros-task-client` when intentionally
reusing a verified target. The integration target starts the mock only for
that explicit command and performs the same sourced setup through its runner;
it is not part of lint-only checks.

The Rust package's `Cargo.lock` is intentionally not tracked. `colcon-cargo`
generates Cargo patches for the selected ROS distribution, so dependency
resolution is distro-specific (for example, Jazzy and Humble use different
`action_msgs` versions). `make ros-task-test` therefore allows Cargo to create
or update the package lock in the ROS build environment. The top-level Rust
workspace remains tracked and reproducible with its own pure-Cargo lockfile;
its build, lint, and integration runner use isolated working directories so
the generated ROS patch config cannot add `[[patch.unused]]` entries to that
lockfile.

The same commands can be run from the host without opening a shell:

```bash
docker compose -f docker/dev/compose.yaml run --rm dev make ros-task-test
docker compose -f docker/dev/compose.yaml run --rm dev make ros-task-integration
```

Start the execution mock inside the container:

```bash
make run endpoint DEVICE_TYPE=mock-exec \
  ENDPOINT_ARGS="-p step_delay_s:=0.2"
```

Start the sensor mock in normal mode in one container:

```bash
make run endpoint DEVICE_TYPE=mock-sensor \
  ENDPOINT_ARGS="-p scenario:=normal -p rate_hz:=5.0 -p random_seed:=0"
```

With the normal sensor mock running, read one sample:

```bash
ros2 topic echo --once /physio/mock_spo2 physio_interfaces/msg/PhysioSample
```

The seeded normal SpO2 sample is approximately `98.57`. Stop that publisher,
then start anomaly mode for comparison:

```bash
make run endpoint DEVICE_TYPE=mock-sensor \
  ENDPOINT_ARGS="-p scenario:=anomaly -p rate_hz:=5.0 -p random_seed:=0"
```

`make run endpoint` sources the selected ROS distribution and the merged
install. To use ROS CLI tools, open another sourced container shell:

```bash
docker compose -f docker/dev/compose.yaml run --rm dev bash
source /opt/ros/$ROS_DISTRO/setup.bash
source "$ROS_BUILD_ROOT/install/setup.bash"
```

Inspect both installed interfaces and discover the action with its type:

```bash
ros2 interface show task_interfaces/action/ExecuteTask
ros2 interface show physio_interfaces/msg/PhysioSample
ros2 action list -t
```

Send a goal and display feedback:

```bash
ros2 action send_goal /mock_exec/execute_task task_interfaces/action/ExecuteTask \
  "{task_id: task-1, device_id: mock_exec, primitive: go_to_tag, payload_json: '{\"target_tag\":42}', deadline_unix_ms: 0}" \
  --feedback
```

To exercise cancellation, increase the step delay, send `go_to_tag`, then press
`Ctrl-C` after the first feedback. The action server accepts cancellation and
returns `final_state: canceled`:

```bash
ros2 action send_goal /mock_exec/execute_task task_interfaces/action/ExecuteTask \
  "{task_id: task-cancel, device_id: mock_exec, primitive: go_to_tag, payload_json: '{\"target_tag\":7}', deadline_unix_ms: 0}" \
  --feedback
```

Start the node with `-p fail_target_tag:=42`, then send the first example to
exercise the `SDK_ERROR` result without adding test-only fields to the goal.

Read the anomaly sample with the same command:

```bash
ros2 topic echo --once /physio/mock_spo2 physio_interfaces/msg/PhysioSample
```

The seeded anomaly SpO2 sample is approximately `85.57` (and always below
`90.0`), while both samples have `data_type: spo2` and `valid: true`.

## Node parameters

### `mock_exec_layer`

| Parameter | Type | Default | Meaning |
| --- | --- | --- | --- |
| `action_name` | string | `/mock_exec/execute_task` | Action server name. |
| `device_id` | string | `mock_exec` | Device ID accepted by this endpoint. |
| `step_delay_s` | number | `1.0` | Non-negative sleep before each feedback step; a negative value fails node initialization. |
| `fail_target_tag` | integer | `2147483648` | `go_to_tag` target that produces `SDK_ERROR`; the out-of-range default disables failure. |

### `physio_mock_publisher`

| Parameter | Type | Default | Meaning |
| --- | --- | --- | --- |
| `scenario` | string | `normal` | `normal`, or `anomaly` to lower only SpO2; other values are unsupported. |
| `rate_hz` | number | `1.0` | Publish rate; non-positive values warn and fall back to `1.0`. |
| `random_seed` | integer | `0` | Seed for reproducible sample noise. |

These packages are development mocks only: they do not connect to
Orchestration, implement a production endpoint runtime, control hardware, or
load a backend SDK.
