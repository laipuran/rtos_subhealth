# ROS Python Mocks Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a minimal ROS 2 workspace containing an RFC-compatible Python execution action mock and physiological sensor publisher, integrated with the existing image, build, and endpoint run workflow.

**Architecture:** Keep ROS sources in `ros2_ws/src` and generated artifacts in `${ROS_RUST_WS:-/ws}`. Restore only the RFC interfaces consumed by the two mocks; do not connect them to Rust services or present them as the canonical `platform` mapper.

**Tech Stack:** ROS 2 Humble/Jazzy, `rosidl`, `ament_cmake`, `ament_python`, `rclpy`, Python 3, pytest, GNU Make, Docker Compose.

**Spec:** `docs/superpowers/specs/2026-09-17-ros-python-mocks-design.md`

## Global Constraints

- All ROS package sources live under `ros2_ws/src`.
- Restore only `ExecTask.action`, `Constraints.msg`, and `PhysioSample.msg` from master RFC contracts.
- Do not add `PlanPath.srv`, `Segment.msg`, diagnosis messages, or AprilTag interfaces.
- Do not modify Gateway, Orchestration, Execution, Sensor, or `platform` Rust behavior.
- Do not add mock-specific phony targets; use `make build` and `make run endpoint`.
- Host `make build` remains usable without ROS; container `make build` additionally runs colcon.
- ROS build/install/log output goes to `${ROS_RUST_WS:-/ws}`.
- Python mocks must not import device SDKs or control hardware.
- Documentation must distinguish RFC workflow mocks from the current canonical contracts.

## File Structure

```text
ros2_ws/src/interfaces/ros_interfaces/       RFC 003/004 action and constraint types
ros2_ws/src/interfaces/physio_interfaces/    RFC 009 physiological sample type
ros2_ws/src/mocks/mock_exec_layer/           Python ExecTask ActionServer
ros2_ws/src/mocks/physio_mock_publisher/     Python physiological topic publisher
docs/guide/ros-mocks.md                       Build, run, RFC subset, smoke checks
```

---

### Task 1: Minimal RFC Interface Packages

**Files:**
- Create: `ros2_ws/src/interfaces/ros_interfaces/action/ExecTask.action`
- Create: `ros2_ws/src/interfaces/ros_interfaces/msg/Constraints.msg`
- Create: `ros2_ws/src/interfaces/ros_interfaces/CMakeLists.txt`
- Create: `ros2_ws/src/interfaces/ros_interfaces/package.xml`
- Create: `ros2_ws/src/interfaces/physio_interfaces/msg/PhysioSample.msg`
- Create: `ros2_ws/src/interfaces/physio_interfaces/CMakeLists.txt`
- Create: `ros2_ws/src/interfaces/physio_interfaces/package.xml`

**Interfaces:**
- Consumes: RFC 003 `ExecTask`, RFC 004 `Constraints`, RFC 009 `PhysioSample` from `master`.
- Produces: `ros_interfaces/action/ExecTask`, `ros_interfaces/msg/Constraints`, and `physio_interfaces/msg/PhysioSample` Python modules.

- [ ] **Step 1: Verify the interfaces are initially unavailable**

Run inside the ROS image:

```bash
source /opt/ros/$ROS_DISTRO/setup.bash
ros2 interface show ros_interfaces/action/ExecTask
```

Expected: non-zero exit stating that `ros_interfaces/action/ExecTask` is unknown.

- [ ] **Step 2: Add the exact minimal interface definitions**

`ExecTask.action`:

```text
string type
int32 priority
string route_id
int32[] target_tags
ros_interfaces/Constraints constraints
int64 deadline_ms
builtin_interfaces/Time issue_time
---
string final_state
string error_code
string message
builtin_interfaces/Time finished_time
---
string state
float32 progress
int32 current_tag
int32 next_tag
int32 finished_stages
int32[] route
string error_code
string message
builtin_interfaces/Time timestamp
```

`Constraints.msg`:

```text
float32 max_speed_mps
float32 min_clearance_m
int32[] avoid_tags
```

`PhysioSample.msg`:

```text
builtin_interfaces/Time timestamp
string data_src
string data_type
float32 data
bool valid
```

Each CMake package must call `rosidl_generate_interfaces`, depend on
`builtin_interfaces`, export `rosidl_default_runtime`, and declare membership
in `rosidl_interface_packages`. Do not list any other message or service.

- [ ] **Step 3: Build only the interface packages**

Run inside the ROS image:

```bash
source /opt/ros/$ROS_DISTRO/setup.bash
colcon --log-base /ws/log build \
  --base-paths ros2_ws/src \
  --build-base /ws/build \
  --install-base /ws/install \
  --packages-select ros_interfaces physio_interfaces
```

Expected: both packages finish successfully.

- [ ] **Step 4: Verify generated interfaces**

```bash
source /opt/ros/$ROS_DISTRO/setup.bash
source /ws/install/setup.bash
ros2 interface show ros_interfaces/action/ExecTask
ros2 interface show ros_interfaces/msg/Constraints
ros2 interface show physio_interfaces/msg/PhysioSample
```

Expected: output exactly contains the fields above and no planner/diagnosis fields.

- [ ] **Step 5: Commit**

```bash
git add ros2_ws/src/interfaces
git commit -m "feat(ros): add minimal RFC interfaces"
```

---

### Task 2: Python Execution Mock

**Files:**
- Create: `ros2_ws/src/mocks/mock_exec_layer/mock_exec_layer/__init__.py`
- Create: `ros2_ws/src/mocks/mock_exec_layer/mock_exec_layer/logic.py`
- Create: `ros2_ws/src/mocks/mock_exec_layer/mock_exec_layer/node.py`
- Create: `ros2_ws/src/mocks/mock_exec_layer/test/test_logic.py`
- Create: `ros2_ws/src/mocks/mock_exec_layer/resource/mock_exec_layer`
- Create: `ros2_ws/src/mocks/mock_exec_layer/package.xml`
- Create: `ros2_ws/src/mocks/mock_exec_layer/setup.cfg`
- Create: `ros2_ws/src/mocks/mock_exec_layer/setup.py`

**Interfaces:**
- Consumes: `ros_interfaces.action.ExecTask` from Task 1.
- Produces: executable `mock_exec_layer_node`; pure helpers `route_for(task_type: str, target_tags: Sequence[int]) -> list[int]` and `step_feedback(route: Sequence[int], index: int) -> StepFeedback`.

- [ ] **Step 1: Write failing pure behavior tests**

```python
import pytest

from mock_exec_layer.logic import UnsupportedTask, route_for, step_feedback


def test_hold_has_no_route():
    assert route_for("hold", [1]) == []


def test_go_to_tag_uses_first_target():
    assert route_for("go_to_tag", [7, 8]) == [7]


def test_patrol_uses_all_targets():
    assert route_for("patrol_route", [7, 8]) == [7, 8]


def test_unknown_task_is_rejected():
    with pytest.raises(UnsupportedTask):
        route_for("dance", [])


def test_final_step_clears_next_tag():
    feedback = step_feedback([7, 8], 1)
    assert feedback.progress == 1.0
    assert feedback.current_tag == 8
    assert feedback.next_tag == -1
```

- [ ] **Step 2: Run tests and verify the missing module failure**

```bash
PYTHONPATH=ros2_ws/src/mocks/mock_exec_layer \
python3 -m pytest ros2_ws/src/mocks/mock_exec_layer/test/test_logic.py -q
```

Expected: FAIL because `mock_exec_layer.logic` does not exist.

- [ ] **Step 3: Implement deterministic execution helpers**

Implement immutable `StepFeedback(progress, current_tag, next_tag,
finished_stages)` and these rules:

```python
FALLBACK_GOAL_TAG = 42
FALLBACK_PATROL_ROUTE = (10, 20, 30)

hold -> []
go_to_tag -> [first target] or [42]
patrol_route -> supplied list or [10, 20, 30]
progress -> (index + 1) / len(route)
next_tag -> route[index + 1], otherwise -1
```

Raise `UnsupportedTask(task_type)` for all other values and `IndexError` for an
invalid feedback index.

- [ ] **Step 4: Run pure tests and verify they pass**

Run the command from Step 2. Expected: all five tests pass.

- [ ] **Step 5: Implement the ROS ActionServer and package metadata**

Implement `MockExecLayerNode` with:

- parameters `action_name="mock_exec_task"`, `step_delay_s=1.0`;
- empty task type rejected in `goal_callback`;
- cancellation accepted in `cancel_callback`;
- negative max speed aborted with `final_state="failed"` and
  `error_code="SIMULATED_FAILURE"`;
- helper-driven feedback for `hold`, `go_to_tag`, and `patrol_route`;
- unknown task type aborted with `error_code="UNSUPPORTED_TYPE"`;
- cancellation completed with `final_state="canceled"`;
- all feedback timestamps and result `finished_time` populated from the node
  clock.

Expose:

```text
mock_exec_layer_node = mock_exec_layer.node:main
```

Declare `ament_python`, `rclpy`, `ros_interfaces`, and `builtin_interfaces` in
`package.xml`; declare `python3-pytest` as a test dependency.

- [ ] **Step 6: Build and inspect the executable**

```bash
source /opt/ros/$ROS_DISTRO/setup.bash
colcon --log-base /ws/log build \
  --base-paths ros2_ws/src \
  --build-base /ws/build \
  --install-base /ws/install \
  --packages-up-to mock_exec_layer \
  --symlink-install
source /ws/install/setup.bash
ros2 pkg executables mock_exec_layer
```

Expected: output contains `mock_exec_layer mock_exec_layer_node`.

- [ ] **Step 7: Commit**

```bash
git add ros2_ws/src/mocks/mock_exec_layer
git commit -m "feat(ros): add Python execution mock"
```

---

### Task 3: Python Physiological Sensor Mock

**Files:**
- Create: `ros2_ws/src/mocks/physio_mock_publisher/physio_mock_publisher/__init__.py`
- Create: `ros2_ws/src/mocks/physio_mock_publisher/physio_mock_publisher/model.py`
- Create: `ros2_ws/src/mocks/physio_mock_publisher/physio_mock_publisher/node.py`
- Create: `ros2_ws/src/mocks/physio_mock_publisher/test/test_model.py`
- Create: `ros2_ws/src/mocks/physio_mock_publisher/resource/physio_mock_publisher`
- Create: `ros2_ws/src/mocks/physio_mock_publisher/package.xml`
- Create: `ros2_ws/src/mocks/physio_mock_publisher/setup.cfg`
- Create: `ros2_ws/src/mocks/physio_mock_publisher/setup.py`

**Interfaces:**
- Consumes: `physio_interfaces.msg.PhysioSample` from Task 1.
- Produces: executable `physio_mock_publisher_node`; `SensorSpec` records and `sample_value(spec: SensorSpec, scenario: str, rng: random.Random) -> float`.

- [ ] **Step 1: Write failing deterministic model tests**

```python
import random

import pytest

from physio_mock_publisher.model import SENSOR_SPECS, sample_value


def test_all_rfc_sensor_types_are_present():
    assert {spec.data_type for spec in SENSOR_SPECS} == {
        "spo2", "heart_rate", "systolic_mmhg", "diastolic_mmhg",
        "body_temp_c", "respiratory_rate",
    }


def test_anomaly_only_forces_spo2_low():
    rng = random.Random(0)
    spo2 = next(spec for spec in SENSOR_SPECS if spec.data_type == "spo2")
    assert sample_value(spo2, "anomaly", rng) < 90.0


def test_unknown_scenario_is_rejected():
    with pytest.raises(ValueError):
        sample_value(SENSOR_SPECS[0], "invalid", random.Random(0))
```

- [ ] **Step 2: Run tests and verify the missing module failure**

```bash
PYTHONPATH=ros2_ws/src/mocks/physio_mock_publisher \
python3 -m pytest ros2_ws/src/mocks/physio_mock_publisher/test/test_model.py -q
```

Expected: FAIL because `physio_mock_publisher.model` does not exist.

- [ ] **Step 3: Implement the sensor model**

Create immutable `SensorSpec(data_src, data_type, base, noise)` records for the
six RFC 009 sources. `sample_value` must:

- accept only `normal` and `anomaly`;
- use `base + rng.uniform(-noise, noise)` normally;
- use `85.0 + rng.uniform(-1.0, 1.0)` only for anomalous SpO2;
- round to two decimal places.

- [ ] **Step 4: Run model tests and verify they pass**

Run the command from Step 2. Expected: all three tests pass.

- [ ] **Step 5: Implement the ROS publisher and package metadata**

Implement `PhysioMockPublisher` with:

- `scenario="normal"`, `rate_hz=1.0`, and `random_seed=0` parameters;
- fallback to 1 Hz with a warning when `rate_hz <= 0`;
- RELIABLE, KEEP_LAST, depth 10 QoS;
- one publisher at `/physio/{data_src}` per `SensorSpec`;
- current node timestamp and `valid=True` for each message;
- a private `random.Random(random_seed)` instance.

Expose:

```text
physio_mock_publisher_node = physio_mock_publisher.node:main
```

Declare `ament_python`, `rclpy`, `physio_interfaces`, and
`builtin_interfaces`; declare `python3-pytest` as a test dependency.

- [ ] **Step 6: Build and inspect the executable**

```bash
source /opt/ros/$ROS_DISTRO/setup.bash
colcon --log-base /ws/log build \
  --base-paths ros2_ws/src \
  --build-base /ws/build \
  --install-base /ws/install \
  --packages-up-to physio_mock_publisher \
  --symlink-install
source /ws/install/setup.bash
ros2 pkg executables physio_mock_publisher
```

Expected: output contains
`physio_mock_publisher physio_mock_publisher_node`.

- [ ] **Step 7: Commit**

```bash
git add ros2_ws/src/mocks/physio_mock_publisher
git commit -m "feat(ros): add Python sensor mock"
```

---

### Task 4: Make and Container Workflow

**Files:**
- Modify: `Makefile`
- Modify: `docker/dev/Dockerfile`

**Interfaces:**
- Consumes: the four ROS packages from Tasks 1-3 and existing `ROS_RUST_WS=/ws`.
- Produces: container-aware `make build`; endpoint profiles `mock-exec` and `mock-sensor`; `ENDPOINT_ARGS` forwarding.

- [ ] **Step 1: Capture the current missing ROS build behavior**

Inside the ROS image, remove only generated workspace artifacts and run:

```bash
rm -rf /ws/build /ws/install /ws/log
make build
test -f /ws/install/setup.bash
```

Expected before the Makefile change: final `test` fails because `make build`
only builds Rust.

- [ ] **Step 2: Add container-aware build variables and recipe**

Add:

```make
ROS_BUILD_ROOT ?= $(if $(ROS_RUST_WS),$(ROS_RUST_WS),/ws)
ENDPOINT_ARGS ?=
```

Keep `cargo build --workspace` in all environments. When `IN_CONTAINER=1`,
append a colcon command that sources `/opt/ros/$(ROS_DISTRO)/setup.bash`, uses
`ros2_ws/src` as its base path, writes `build/install/log` beneath
`$(ROS_BUILD_ROOT)`, and enables `--symlink-install`.

- [ ] **Step 3: Add endpoint dispatch without adding phony targets**

Replace the endpoint placeholder with an in-container dispatch:

```text
DEVICE_TYPE=mock-exec   -> ros2 run mock_exec_layer mock_exec_layer_node
DEVICE_TYPE=mock-sensor -> ros2 run physio_mock_publisher physio_mock_publisher_node
```

Before `ros2 run`, source `/opt/ros/$(ROS_DISTRO)/setup.bash` and
`$(ROS_BUILD_ROOT)/install/setup.bash`. Append `--ros-args $(ENDPOINT_ARGS)`
only when `ENDPOINT_ARGS` is non-empty. Missing install setup and unknown
device types must exit non-zero with actionable messages.

- [ ] **Step 4: Ensure Python test tooling exists in both images**

Add `python3-pytest` to the Docker apt packages. Do not install project Python
dependencies globally with pip.

- [ ] **Step 5: Verify build and endpoint discovery in Jazzy**

```bash
make image jazzy
docker compose -f docker/dev/compose.yaml run --rm dev make build
docker compose -f docker/dev/compose.yaml run --rm dev \
  bash -lc 'source /opt/ros/$ROS_DISTRO/setup.bash && source /ws/install/setup.bash && ros2 pkg executables mock_exec_layer && ros2 pkg executables physio_mock_publisher'
```

Expected: build succeeds and both executables are listed.

- [ ] **Step 6: Verify Make errors**

Inside the container:

```bash
make run endpoint DEVICE_TYPE=unknown
```

Expected: exit code 2 with supported `DEVICE_TYPE` values in the message.

- [ ] **Step 7: Commit**

```bash
git add Makefile docker/dev/Dockerfile
git commit -m "build: integrate ROS mocks with container workflow"
```

---

### Task 5: Contract and Workflow Documentation

**Files:**
- Create: `docs/guide/ros-mocks.md`
- Modify: `README.md`
- Modify: `docs/architecture/build-and-run.md`
- Modify: `docs/contracts/domain.md`
- Modify: `docs/contracts/execution.md`
- Modify: `docs/contracts/sensor.md`
- Modify: `docs/contracts/event.md`
- Modify: `docs/contracts/task.md`
- Modify: `TODO.md`

**Interfaces:**
- Consumes: exact ROS interface fields and runtime commands from Tasks 1-4; current Rust types under `services/platform/src`.
- Produces: authoritative canonical field documentation and an explicitly scoped RFC mock guide.

- [ ] **Step 1: Document the ROS mock workflow and RFC subset**

Create `docs/guide/ros-mocks.md` with:

- workspace/package tree;
- exact contracts used: RFC 003 `ExecTask`, RFC 004 `Constraints` and feedback
  rules, RFC 009 `PhysioSample`;
- explicit exclusions: `PlanPath.srv`, `Segment.msg`, diagnosis and AprilTag;
- warning that these RFC interfaces are not yet the canonical `platform` ROS
  mapper;
- image, in-container build, exec run, sensor run, action CLI, cancellation,
  and topic echo commands;
- parameter tables for both nodes.

- [ ] **Step 2: Add exact canonical fields to contract pages**

Document the current Rust source, without inventing fields:

```text
domain.md:
  TaskId(String), DeviceId(String), SensorId(String)
  DeviceDescriptor{id,name,capabilities,primitives,sensors}
  DeviceState{device_id,healthy,message,updated_at_ms}

execution.md:
  ExecutionCommand{task_id,primitive,payload,deadline_ms}
  ExecutionFeedback{task_id,progress,phase}
  ExecutionResult{task_id,state,error_code,message}
  ExecutionError variants and Executor/ExecutionHandlePort signatures

sensor.md:
  SensorDescriptor{id,kind,unit}
  SensorSample{sensor_id,value,timestamp_ms}
  SensorFilter{ids}
  SensorProvider method signatures

event.md:
  EventSequence(u64)
  TaskStateChanged, DeviceStateChanged, SensorUpdated payloads
```

Add a short cross-reference in `task.md` and each affected page explaining
that the RFC ROS mock fields require a future explicit mapper.

- [ ] **Step 3: Update entry-point documentation and backlog status**

Update README and `build-and-run.md` with the exact `make image`, container
`make build`, and `make run endpoint` examples. In `TODO.md`, mark only these
delivered pieces:

- minimal ROS workspace and interface generation entry;
- RFC-compatible exec and sensor mocks for development validation.

Keep production ROS mapper, endpoint runtime, Orchestration connection, and
real adapters unchecked.

- [ ] **Step 4: Verify documentation commands and links**

```bash
grep -R "PlanPath.srv" ros2_ws/src && exit 1 || true
grep -R "Segment.msg" ros2_ws/src && exit 1 || true
test -f docs/guide/ros-mocks.md
git diff --check
```

Expected: no excluded interfaces in `ros2_ws/src`, guide exists, and no
whitespace errors.

- [ ] **Step 5: Commit**

```bash
git add README.md TODO.md docs
git commit -m "docs: document ROS mock workflow and contracts"
```

---

### Task 6: End-to-End Container Verification

**Files:**
- Modify only files from Tasks 1-5 if verification exposes a defect.

**Interfaces:**
- Consumes: complete ROS workspace and Make workflow.
- Produces: verified Jazzy build, action, cancellation, sensor topic, formatting, and Rust regression status.

- [ ] **Step 1: Run repository checks**

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
git diff --check
```

Expected: all commands exit zero.

- [ ] **Step 2: Build all ROS packages in the Jazzy image**

```bash
make image jazzy
docker compose -f docker/dev/compose.yaml run --rm dev make build
```

Expected: Rust and all four ROS packages build successfully.

- [ ] **Step 3: Run package tests**

```bash
docker compose -f docker/dev/compose.yaml run --rm dev bash -lc '
  source /opt/ros/$ROS_DISTRO/setup.bash
  source /ws/install/setup.bash
  colcon test \
    --build-base /ws/build \
    --install-base /ws/install \
    --packages-select mock_exec_layer physio_mock_publisher
  colcon test-result --test-result-base /ws/build --verbose
'
```

Expected: both Python test suites pass with no failures.

- [ ] **Step 4: Verify execution action success and cancellation**

In one container shell, start:

```bash
make run endpoint DEVICE_TYPE=mock-exec ENDPOINT_ARGS="-p step_delay_s:=0.2"
```

In another sourced container shell, run:

```bash
ros2 action list -t
ros2 action send_goal /mock_exec_task ros_interfaces/action/ExecTask \
  "{type: hold}" --feedback
ros2 action send_goal /mock_exec_task ros_interfaces/action/ExecTask \
  "{type: go_to_tag, target_tags: [42]}" --feedback
ros2 action send_goal /mock_exec_task ros_interfaces/action/ExecTask \
  "{type: patrol_route, target_tags: [1, 2, 3, 4, 5]}" --feedback
```

For the patrol command, press `Ctrl-C` after its first feedback to request
cancellation, then inspect the server log for `final_state=canceled`. Expected
terminal states are `succeeded`, `succeeded`, and `canceled`; feedback progress
remains in `[0,1]`, and final `next_tag` is `-1`.

- [ ] **Step 5: Verify normal and anomaly samples**

Start the sensor mock:

```bash
make run endpoint DEVICE_TYPE=mock-sensor ENDPOINT_ARGS="-p scenario:=anomaly -p rate_hz:=5.0 -p random_seed:=0"
```

Then run:

```bash
ros2 topic echo --once /physio/mock_spo2 physio_interfaces/msg/PhysioSample
```

Expected: `data_type: spo2`, `valid: true`, and `data < 90.0`.

- [ ] **Step 6: Record final status**

```bash
git status --short
git log --oneline -6
```

Expected: no unintended files such as `ros2_ws/build`, `ros2_ws/install`, or
`ros2_ws/log`; commits correspond to the tasks above.
