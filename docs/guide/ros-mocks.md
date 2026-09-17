# ROS 2 RFC mocks

This workspace provides a small RFC-compatible ROS 2 surface for development
validation. It is **not** the canonical ROS transport for the Rust `platform`
contracts. The fields do not map one-to-one; a production transport still needs
an explicit, versioned domain/ROS mapper.

## Workspace

```text
ros2_ws/src/
├── interfaces/
│   ├── ros_interfaces/
│   │   ├── action/ExecTask.action
│   │   └── msg/Constraints.msg
│   └── physio_interfaces/
│       └── msg/PhysioSample.msg
└── mocks/
    ├── mock_exec_layer/          # ExecTask action server
    └── physio_mock_publisher/    # PhysioSample topic publisher
```

Only the following RFC subset is present:

- RFC 003 `ros_interfaces/action/ExecTask`;
- RFC 004 `ros_interfaces/msg/Constraints` and its execution-feedback rules;
- RFC 009 `physio_interfaces/msg/PhysioSample`.

`PlanPath.srv`, `Segment.msg`, diagnosis interfaces, and AprilTag perception
interfaces are deliberately excluded. The integer tag fields in `ExecTask` are
deterministic mock inputs; they do not provide AprilTag detection.

## Interface fields

### RFC 003: `ExecTask`

| Section | Fields |
| --- | --- |
| Goal | `string type`, `int32 priority`, `string route_id`, `int32[] target_tags`, `ros_interfaces/Constraints constraints`, `int64 deadline_ms`, `builtin_interfaces/Time issue_time` |
| Result | `string final_state`, `string error_code`, `string message`, `builtin_interfaces/Time finished_time` |
| Feedback | `string state`, `float32 progress`, `int32 current_tag`, `int32 next_tag`, `int32 finished_stages`, `int32[] route`, `string error_code`, `string message`, `builtin_interfaces/Time timestamp` |

The execution mock accepts `hold`, `go_to_tag`, and `patrol_route`. An empty
`type` is rejected. `go_to_tag` uses the first supplied tag or `42`;
`patrol_route` uses the supplied route or `[10, 20, 30]`. A negative
`constraints.max_speed_mps` produces `SIMULATED_FAILURE`; another task type
produces `UNSUPPORTED_TYPE`.

For every route step, feedback has `state=executing`, progress `(index + 1) /
route length`, the current and next tags, a one-based `finished_stages`, the
whole route, and a current timestamp. Thus progress stays in `[0.0, 1.0]` and
the final step uses `next_tag=-1`. `error_code` and `message` remain empty in
normal feedback. `hold` has an empty route and therefore emits no feedback.

### RFC 004: `Constraints`

| Field | Type |
| --- | --- |
| `max_speed_mps` | `float32` |
| `min_clearance_m` | `float32` |
| `avoid_tags` | `int32[]` |

The mock only uses `max_speed_mps` for its deterministic failure case. It does
not run a planner.

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

Build the selected image on the host, then run the build in the container:

```bash
make image jazzy
docker compose -f docker/dev/compose.yaml run --rm dev make build
```

The container build runs `colcon build --merge-install --symlink-install`.
With the default `ROS_BUILD_ROOT=/ws`, logs, intermediate files, and the merged
install are written to `/ws/log`, `/ws/build/merged-symlink`, and `/ws/install`;
they are not written below `ros2_ws/`. `make image humble` selects the Humble
image instead.

Start the execution mock in one container:

```bash
docker compose -f docker/dev/compose.yaml run --rm dev \
  make run endpoint DEVICE_TYPE=mock-exec \
  ENDPOINT_ARGS="-p step_delay_s:=0.2"
```

Start the sensor mock in one container:

```bash
docker compose -f docker/dev/compose.yaml run --rm dev \
  make run endpoint DEVICE_TYPE=mock-sensor \
  ENDPOINT_ARGS="-p scenario:=anomaly -p rate_hz:=5.0 -p random_seed:=0"
```

`make run endpoint` sources the selected ROS distribution and the merged
install. To use ROS CLI tools, open another sourced container shell:

```bash
docker compose -f docker/dev/compose.yaml run --rm dev bash
source /opt/ros/$ROS_DISTRO/setup.bash
source /ws/install/setup.bash
```

Send a goal and display feedback:

```bash
ros2 action send_goal /mock_exec_task ros_interfaces/action/ExecTask \
  "{type: go_to_tag, target_tags: [42]}" --feedback
```

To exercise cancellation, send a route long enough to interrupt, then press
`Ctrl-C` after the first feedback. The action server accepts the cancellation
request and returns `final_state: canceled`:

```bash
ros2 action send_goal /mock_exec_task ros_interfaces/action/ExecTask \
  "{type: patrol_route, target_tags: [1, 2, 3, 4, 5]}" --feedback
```

With the sensor mock running, read one sample:

```bash
ros2 topic echo --once /physio/mock_spo2 physio_interfaces/msg/PhysioSample
```

## Node parameters

### `mock_exec_layer`

| Parameter | Type | Default | Meaning |
| --- | --- | --- | --- |
| `action_name` | string | `mock_exec_task` | Action server name. |
| `step_delay_s` | number | `1.0` | Sleep before each route-step feedback when greater than zero. |

### `physio_mock_publisher`

| Parameter | Type | Default | Meaning |
| --- | --- | --- | --- |
| `scenario` | string | `normal` | `normal`, or `anomaly` to lower only SpO2; other values are unsupported. |
| `rate_hz` | number | `1.0` | Publish rate; non-positive values warn and fall back to `1.0`. |
| `random_seed` | integer | `0` | Seed for reproducible sample noise. |

These packages are development mocks only: they do not connect to
Orchestration, implement a production endpoint runtime, control hardware, or
load a backend SDK.
