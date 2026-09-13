# Phase 0 spike: GO2 DDS compatibility on ROS 2 Jazzy

**Purpose.** Prove that a control stack on ROS 2 Jazzy / Ubuntu 24.04 can talk to
the GO2 over its DDS interface. This is the gate for Phase 3 (Rust `robot_driver`
hardware backend). All ROS tooling runs **inside Docker**; nothing is installed
on the host.

**Why this is the gate.** The GO2 couples at the CycloneDDS 0.10.2 + Unitree
IDL/topic layer, not at the ROS-distro layer
(`docs/tech/tech-platform-migration-research.md` §A). Unitree officially tests
Foxy and Humble, not Jazzy. We must confirm Jazzy's `rmw_cyclonedds_cpp` and
message types interoperate before investing in the Rust backend.

## Pass criteria

1. `unitree_go` / `unitree_api` message packages build on Jazzy.
2. A node on Jazzy can **publish `rt/lowcmd`** and the simulated GO2 reacts.
3. The same node **subscribes `rt/sportmodestate`** and receives state.
4. Domain id and network interface match the GO2 (default domain `0`; control
   subnet `192.168.123.0/24` for a real robot).

## Failure path

If Jazzy cannot interoperate, do **not** block Phases 1/2/4. Isolate the failure
to one place: implement a thin C++ `unitree_sdk2` shim behind the `RobotBackend`
trait and keep the rest of the system on Jazzy. Record the decision in an ADR.

## Steps

### 1. Build the Jazzy + Rust image

```bash
docker build -t ros-dev:jazzy docker/dev
```

### 2. Vendor Unitree interfaces (pinned)

```bash
mkdir -p ros2_ws/src/robot/vendor && cd ros2_ws/src/robot/vendor
git clone https://github.com/unitreerobotics/unitree_ros2.git
cd unitree_ros2 && git checkout v0.3.0
# Keep only the interface packages; drop the C++ examples.
```

Record the exact commit in this file after cloning.

### 3. Build the interfaces on Jazzy

```bash
docker run --rm -v "$PWD":/workspace -w /workspace ros-dev:jazzy bash -lc '
  source /opt/ros/jazzy/setup.bash
  colcon build --base-paths ros2_ws/src/robot/vendor/unitree_ros2
'
```

Expected: `unitree_go` and `unitree_api` build without errors.

### 4. Start a MuJoCo GO2 bridge

Follow the Unitree `unitree_mujoco` instructions, pinned to the same
CycloneDDS 0.10.2 and domain/interface as the control side. For a real robot,
put the control PC on `192.168.123.0/24` and point `CYCLONEDDS_URI` at that
interface.

### 5. Run the probe

The Rust probe is part of the Phase 3 `robot_driver` crate. Until it exists, use
the upstream C++ example as the reference signal:

```bash
# Inside the container, with the Unitree workspace sourced:
ros2 topic list | grep -E 'lowcmd|sportmodestate'
ros2 topic echo /sportmodestate --once
# Publish a LowCmd from the upstream example and confirm the sim reacts.
```

### 6. Record the result

- PASS: proceed with the Rust `UnitreeBackend`.
- FAIL: open an ADR and implement the C++ shim.
