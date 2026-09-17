# ROS Python Mocks Design

## 1. Goal

Restore a minimal ROS 2 workspace under `ros2_ws/` for validating the
repository's ROS image, interface generation, Python package build, node
startup, action lifecycle, and sensor publication workflows.

The deliverable contains only a Python execution mock and a Python sensor
mock. It does not connect Gateway or Orchestration to ROS, and it does not
implement a production endpoint runtime.

## 2. Architectural Boundary

The mocks are endpoint-side development tools:

```text
ExecTask client ──ROS action──► mock_exec_layer

physio_mock_publisher ──ROS topics──► developer/test subscriber
```

They do not import or call Rust services. In particular, this change does not:

- implement the Orchestration `ExecutionPort`;
- add a Gateway-to-Orchestration path;
- add a `platform` to ROS mapper;
- add planner, diagnosis, persistence, or device SDK behavior.

RFC 003/004/009 predate the current canonical types in `services/platform`.
Their fields do not map one-to-one to the current contracts. The restored ROS
packages are therefore RFC-compatible workflow mocks, not the completed
production ROS transport. Documentation must state this explicitly.

## 3. Workspace Layout

All ROS source packages live in `ros2_ws/src`:

```text
ros2_ws/
└── src/
    ├── interfaces/
    │   ├── ros_interfaces/
    │   │   ├── action/ExecTask.action
    │   │   ├── msg/Constraints.msg
    │   │   ├── CMakeLists.txt
    │   │   └── package.xml
    │   └── physio_interfaces/
    │       ├── msg/PhysioSample.msg
    │       ├── CMakeLists.txt
    │       └── package.xml
    └── mocks/
        ├── mock_exec_layer/
        │   ├── mock_exec_layer/
        │   │   ├── __init__.py
        │   │   └── node.py
        │   ├── resource/mock_exec_layer
        │   ├── package.xml
        │   ├── setup.cfg
        │   └── setup.py
        └── physio_mock_publisher/
            ├── physio_mock_publisher/
            │   ├── __init__.py
            │   └── node.py
            ├── resource/physio_mock_publisher
            ├── package.xml
            ├── setup.cfg
            └── setup.py
```

Colcon build, install, and log artifacts live under the existing
`ROS_RUST_WS=/ws` volume and are not written into `ros2_ws/`.

## 4. Minimal RFC Contract Set

Only three generated ROS interfaces are restored.

### 4.1 RFC 003: `ros_interfaces/action/ExecTask`

The action is the mock execution task entry point.

Goal fields:

- `string type`
- `int32 priority`
- `string route_id`
- `int32[] target_tags`
- `ros_interfaces/Constraints constraints`
- `int64 deadline_ms`
- `builtin_interfaces/Time issue_time`

Result fields:

- `string final_state`
- `string error_code`
- `string message`
- `builtin_interfaces/Time finished_time`

Feedback fields:

- `string state`
- `float32 progress`
- `int32 current_tag`
- `int32 next_tag`
- `int32 finished_stages`
- `int32[] route`
- `string error_code`
- `string message`
- `builtin_interfaces/Time timestamp`

### 4.2 RFC 004: `ros_interfaces/msg/Constraints`

RFC 004 refines the execution planning fields used by RFC 003. Only the
constraint message required by `ExecTask` is restored:

- `float32 max_speed_mps`
- `float32 min_clearance_m`
- `int32[] avoid_tags`

The mock follows RFC 004's `progress`, `current_tag`, and `next_tag` update
rules. It does not restore `PlanPath.srv` or `Segment.msg`, because the mock
does not call a planner.

### 4.3 RFC 009: `physio_interfaces/msg/PhysioSample`

The sensor mock publishes:

- `builtin_interfaces/Time timestamp`
- `string data_src`
- `string data_type`
- `float32 data`
- `bool valid`

No diagnosis messages, triggers, aggregation, LLM, HTTP, or WebSocket
contracts are restored.

## 5. Execution Mock Behavior

`mock_exec_layer` exposes an `ExecTask` ActionServer. Its ROS parameters are:

- `action_name`, default `mock_exec_task`;
- `step_delay_s`, default `1.0`, required to be non-negative.

Behavior:

- reject goals with an empty `type`;
- `hold` succeeds immediately and reports completed feedback;
- `go_to_tag` uses the first `target_tags` item, or a deterministic fallback;
- `patrol_route` walks the supplied tags, or a deterministic fallback route;
- negative `constraints.max_speed_mps` produces a deterministic failed result;
- unknown task types produce a failed result;
- accepted cancellation produces `final_state=canceled`;
- all terminal results set `finished_time`;
- feedback progress is within `[0.0, 1.0]`;
- completed or canceled feedback uses `next_tag=-1`.

The mock never controls hardware and never imports a device SDK.

## 6. Sensor Mock Behavior

`physio_mock_publisher` publishes one `PhysioSample` topic per sensor source:

- `/physio/mock_spo2`;
- `/physio/mock_heart_rate`;
- `/physio/mock_bp_systolic`;
- `/physio/mock_bp_diastolic`;
- `/physio/mock_body_temp`;
- `/physio/mock_respiratory_rate`.

Its ROS parameters are:

- `scenario`, default `normal`, allowed values `normal` and `anomaly`;
- `rate_hz`, default `1.0`; non-positive values fall back to `1.0` with a
  warning;
- `random_seed`, default `0`, so development runs can be reproduced.

The anomaly scenario lowers only SpO2 into the RFC 009 abnormal range. Every
published sample contains a current ROS timestamp and `valid=true`.

## 7. Build and Run Flow

The image remains the only image-related target:

```bash
make image humble
make image jazzy
```

Inside the image, `make build` performs:

1. `cargo build --workspace`;
2. source `/opt/ros/${ROS_DISTRO}/setup.bash`;
3. `colcon build --symlink-install` over `ros2_ws/src`;
4. write build/install/log output under `${ROS_RUST_WS:-/ws}`.

Outside the image, `make build` remains a pure Rust build so ROS is not a host
dependency.

No new mock phony target is introduced. The existing endpoint route starts one
mock at a time:

```bash
make run endpoint DEVICE_TYPE=mock-exec ENDPOINT_ARGS="-p step_delay_s:=0.1"
make run endpoint DEVICE_TYPE=mock-sensor ENDPOINT_ARGS="-p scenario:=anomaly"
```

The endpoint route sources both the selected ROS distribution and
`${ROS_RUST_WS:-/ws}/install/setup.bash`. `ENDPOINT_ARGS` is appended after
`--ros-args`. Unknown `DEVICE_TYPE` values fail with a usage message.

The Makefile dispatch is deployment composition only; it does not add device
branches to Gateway, Orchestration, Execution, or `platform`.

## 8. Validation and Errors

Validation covers:

- `colcon build` generates all three interfaces and installs both Python
  packages;
- `ros2 interface show` succeeds for `ExecTask`, `Constraints`, and
  `PhysioSample`;
- `ros2 action list` shows the configured execution action;
- a CLI action goal receives feedback and a successful result;
- cancellation receives a canceled result;
- `ros2 topic echo --once` receives a valid physiological sample;
- normal and anomaly sensor scenarios can be distinguished;
- unsupported endpoint types, missing install setup, and invalid goal types
  fail visibly rather than silently.

Python logic that does not require a live ROS graph is kept in small helper
functions so package tests can verify goal classification and sensor value
generation deterministically. Live graph checks are documented as container
smoke commands rather than adding a fifth ROS test package.

## 9. Documentation

The implementation updates:

- `README.md` for the available mock endpoint profiles;
- `docs/architecture/build-and-run.md` for image, container build, and run
  commands;
- a ROS mock guide documenting the exact RFC subset and smoke checks;
- `TODO.md` to mark only the ROS workspace/interface-generation and mock
  portions delivered, without marking production transport complete.

Field-level documentation is added to all existing canonical contract pages:

- `docs/contracts/domain.md`;
- `docs/contracts/execution.md`;
- `docs/contracts/sensor.md`;
- `docs/contracts/event.md`.

This is necessary because `docs/contracts/README.md` identifies these pages as
the contract documentation entry point. The pages must describe exact current
Rust fields and explicitly distinguish them from the RFC-compatible ROS mock
interfaces. `docs/contracts/task.md` already contains its canonical fields and
only needs a cross-reference if required.

## 10. Non-Goals

- No `PlanPath.srv` or any other service.
- No `Segment`, diagnosis, vitals aggregate, or AprilTag interfaces.
- No Orchestration-to-execution connection.
- No Rust ROS client or server.
- No production endpoint configuration registry.
- No ORM or persistence.
- No launch stack that starts both mocks automatically.
