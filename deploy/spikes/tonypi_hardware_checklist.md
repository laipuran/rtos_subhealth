# Hardware validation: TonyPi adapter

**Purpose.** Confirm the `adapter` node drives a real Hiwonder TonyPi over its
JSON-RPC interface, and that safety/watchdog behave as designed.

**Interface reminder.** TonyPi is not a ROS robot: it exposes a Python SDK and a
JSON-RPC server on TCP `:9030` with discrete action groups. The adapter maps
`execute_primitive` -> `RunAction`.

## Prerequisites

- TonyPi powered on and connected to the same LAN (or its AP), host reachable.
- You know the robot IP (replace `<ip>` below).
- The `ros-dev:jazzy` image with the built workspace (`docker/dev/build_ros_rust.sh`).

## 1. Verify the JSON-RPC endpoint

```bash
curl -s -X POST "http://<ip>:9030/" -H 'content-type: application/json' \
  -d '{"jsonrpc":"2.0","method":"RunAction","params":["stand",1],"id":1}'
```

Expect a JSON result (HTTP 200). If this fails, the adapter cannot work: fix the
network or the robot's RPC service first.

## 2. Start the adapter

```bash
DEVICE_ID=tonypi DEVICE_TYPE=tonypi TONYPI_RPC_URL=http://<ip>:9030/ \
  ros2 run adapter adapter
```

Verify discovery:

```bash
ros2 topic echo /device_descriptors --once   # kind=legged, primitives=[execute_primitive,hold,stop]
ros2 topic echo /device_states --once        # healthy=true, pose empty
```

## 3. Dispatch actions through the orchestrator

```bash
# in another shell
ros2 run orchestrator orchestrator   # MAP_PATH optional here

ros2 action send_goal /orchestrator/device_task task_interfaces/action/DeviceTask \
"{goal_id: 'h1', device_id: tonypi, primitive: execute_primitive,
  target: {kind: action, tag_id: 0, waypoint_id: '', pose: {header: {frame_id: ''},
    pose: {position: {x: 0.0, y: 0.0, z: 0.0}, orientation: {x: 0.0, y: 0.0, z: 0.0, w: 1.0}}},
    action_id: wave, position_tolerance_m: 0.0, yaw_tolerance_rad: 0.0},
  params_json: '{\"action\":\"wave\"}', constraints: {max_speed_mps: 0.0, min_clearance_m: 0.0, avoid_tags: []},
  deadline_ms: 0}"
```

Expected: the robot performs the action; the goal result is `succeeded`.

## 4. Safety checks

- Set `SAFETY_WATCHDOG_S=2` and stop the state timer (kill the process feeding
  heartbeats). Subsequent commands must be rejected as stale.
- `Stop` must issue `RunAction(["0", 1])` and stop motion.

## 5. Record the result

| Check | Expected | Observed |
|---|---|---|
| RPC reachable | HTTP 200 | |
| Descriptor published | kind=legged, action primitives | |
| `stand`/action group runs | robot moves | |
| `stop` halts | robot stops | |
| Watchdog rejects stale commands | rejected | |

If the RPC schema differs from the documented JSON-RPC shape, adjust
`HttpActionRunner` in `ros2_ws/src/robot/adapter/src/main.rs` accordingly.
