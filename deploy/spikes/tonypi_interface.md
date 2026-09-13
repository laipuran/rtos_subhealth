# Spike: interfacing an external ROS 2 control stack with Hiwonder TonyPi

**Purpose.** Determine how a separate ROS 2 Jazzy control stack (x86_64, this
repo) can command a Hiwonder TonyPi humanoid. This spike decides whether the
`RobotBackend` trait can get a real `TonyPiBackend`, and if so, where it runs.

**Method / reproducibility.** Findings below come from primary sources: Hiwonder's
official docs and product page, and Hiwonder's official GitHub repositories,
inspected at pinned revisions.

- `Hiwonder/TonyPi` @ `dc452eaac9991516600860f706f4b889f3305d1d` (2026-01-19)
  — README reports software version **2025-8-26**; `.typerc` reports **V1.1 /
  2025-04-21**.
- `Hiwonder/MentorPi` (branch `MentorPi-M1`) @
  `fb6d9969e935eb0e31966185158e33347951e761` (2026-01-16) — used only as the
  reference for what a *ROS-native* Hiwonder robot looks like.
- Hiwonder docs `docs.hiwonder.com` (TonyPi / TonyPi Pro v2.0, TonyPi v1.0) and
  product page `hiwonder.com/products/tonypi`.

**Bottom line.** **TonyPi is not a ROS robot.** It ships no ROS 1 or ROS 2
distribution, exposes no ROS graph, and has no `cmd_vel`/`odom`. Its public robot
interface is a **Python SDK + a JSON-RPC server on TCP :9030 + an MJPEG server on
TCP :8080**, layered over a private **STM32 serial-bus-servo protocol** on
`/dev/ttyAMA0`. Any ROS 2 integration must be an *adapter* around those, not a
DDS peer of the robot.

---

## 1. Hardware, OS image, ROS distribution

### Processor / control architecture
- Current TonyPi and TonyPi Pro are both advertised **"Powered by Raspberry Pi
  5"** (`hiwonder.com/products/tonypi`, `hiwonder.com/products/tonypi-pro`, and
  docs §1.1: *"TonyPi is an AI-powered humanoid robot developed by Hiwonder,
  based on the Raspberry Pi 5"*).
- **Dual-brain architecture**: the Raspberry Pi 5 is the high-level compute/vision
  brain; a separate MCU (STM32) on the expansion board does real-time servo,
  motor, IMU, buzzer, RGB and button I/O — docs call it a *"dual-core control
  system"*, and the board SDK is literally commented `# stm32 python sdk`.
- Actuators: **16 high-voltage intelligent serial bus servos** (RPC validates bus
  servo IDs `1..16`), plus **2 PWM servos** for the 2-DOF camera head (PWM IDs
  `1..2`). TonyPi Pro adds open/close hands documented as bus-servo IDs **17/18**.
- Networking: Wi-Fi AP/STA. Default AP SSID begins with `HW`, password
  `hiwonder`, robot IP `192.168.149.1` (docs §14, §3.2).

### OS image
- Hiwonder ships a **custom Raspberry Pi Linux image** with a **LXDE desktop**,
  Terminator, VNC, the `TonyPi` app tree and `hiwonder-toolbox`. Login is
  **`pi` / `raspberrypi`** on the Pi 5 version (`pi` / `raspberry` on older
  units). Evidence of Raspberry Pi OS (Debian) rather than Ubuntu: the `pi` user,
  `/home/pi/TonyPi`, `/boot/config.txt`, `raspi-config`, `rpi-eeprom-config`, and
  `apt` (`expand_rootfs.sh`, `Camera.py`, docs §3.2, §14).
- **The exact base release (Bullseye vs Bookworm, 32- vs 64-bit) is not
  documented.** Hiwonder does not publish the image download: *"System Image &
  Source Code: ... please email us at support@hiwonder.com, and share your order
  number"* (docs Appendix). I could **not verify** the Debian codename.

### ROS 2 distribution on the device
- **None found.** The shipped `Hiwonder/TonyPi` tree contains no `rclpy`,
  `rclcpp`, `ament`/`colcon` package, no `package.xml`, and no ROS launch files.
  A full-tree search found exactly one file whose *name* contains "ros":
  `HiwonderSDK/hiwonder/ros_robot_controller_sdk.py`, and that file is the serial
  board SDK, not ROS code.
- TonyPi does **not** appear in Hiwonder's "ROS Robot" product collection or in
  Hiwonder's ROS-first GitHub repos (`MentorPi`, `ROSpider`, `ROSOrin`,
  `LanderPi`, `ArmPi-Ultra`, …); the official repo describes it only as *"A smart
  AI Robot"* with Python as the language.
- **Hiwonder's actual ROS line is separate and is ROS 2 Humble on Ubuntu
  (`/home/ubuntu/ros2_ws`)** — e.g. MentorPi product listings say
  "ROS2-HUMBLE", and the MentorPi tree ships `ament` packages and `cpython-310`
  bytecode (Ubuntu 22.04 / Humble). Hiwonder's ROS *humanoid* is **AiNex**, which
  is **ROS 1 (Noetic)** / `catkin_make`; there is no official ROS 2 humanoid from
  Hiwonder.

> Practical consequence: "same DDS domain" and "Humble ↔ Jazzy" questions are
> moot for a stock TonyPi. They only become relevant if *you* install ROS 2 on
> the Pi yourself.

---

## 2. SDK / control stack

**Repository:** <https://github.com/Hiwonder/TonyPi> · **current version
2025-8-26** · Python only.

### Python SDK
- `HiwonderSDK/` is a Python package named **`hiwonder`, version 1.0**
  (`setup.py` / `hiwonder.egg-info/PKG-INFO`), installed locally with
  `sudo python3 setup.py install`. It is **not published to PyPI** and has no
  tagged releases.
- Modules: `ros_robot_controller_sdk` (`Board` — the STM32 serial protocol),
  `Controller` (high-level servo/motor/IMU wrapper), `ActionGroupControl`,
  `Camera`, `ASR`, `TTS`, `MP3`, `Sonar`, `PID`, `apriltag`, `common`, `fps`.
- Main APIs:
  - `Board.bus_servo_set_position(duration, [[id, pos], ...])`,
    `bus_servo_read_position(id)`, `bus_servo_read_vin/temp(...)`,
    `bus_servo_enable_torque`, `bus_servo_set_offset/save_offset`,
    `bus_servo_stop(ids)`;
  - `Board.pwm_servo_set_position(...)`;
  - `Board.get_imu()`, `get_battery()`, `get_button()`, `get_gamepad()`,
    `get_sbus()`, `set_led()`, `set_buzzer()`, `set_rgb()`, `set_oled_text()`;
  - `ActionGroupControl.runActionGroup(name, times=1, with_stand=False)` and
    `stopActionGroup()`.

### Wire protocol to the STM32 board (private, but simple)
`ros_robot_controller_sdk.py` opens **`/dev/ttyAMA0` @ 1,000,000 baud** and frames
packets as:

```
0xAA 0x55 <function> <length> <data...> <crc8>
```

Functions: `SYS(0)`, `LED(1)`, `BUZZER(2)`, `MOTOR(3)`, `PWM_SERVO(4)`,
`BUS_SERVO(5)`, `KEY(6)`, `IMU(7)`, `GAMEPAD(8)`, `SBUS(9)`, `OLED(10)`,
`RGB(11)`. CRC-8 with a 256-entry table. Incoming reports are parsed by the same
function IDs. This is the same protocol as Hiwonder's ROS-native products, whose
`ros_robot_controller` package embeds a nearly identical SDK (diff shows only
comment/whitespace changes).

### JSON-RPC robot API (the easiest remote entry point)
`RPCServer.py` runs a **JSON-RPC over HTTP server on port 9030** (Werkzeug +
`jsonrpc`), started by `TonyPi.py` via systemd unit
`/etc/systemd/system/tonypi.service`. Methods include:

| Method | Args (as strings/arrays) | Purpose |
|---|---|---|
| `RunAction` | `['<action-or-id>', times]` | Run an action group; `'0'` stops |
| `StopActionGroup` | `'stopActionGroup'` | Stop current action group |
| `StopBusServo` | `'stopAction'` | Stop current action |
| `StandUp` | — | Fall/stand recovery |
| `SetBusServoPulse` | `[ms, n, id, pulse, ...]` | Move bus servos (IDs 1–16) |
| `SetPWMServo` / `set_pwm_servo` | `[ms, n, id, pulse, ...]` | Move PWM head servos (IDs 1–2) |
| `GetBusServosPulse` | `'angularReadback'` | Read positions |
| `GetBusServosDeviation` / `SaveBusServosDeviation` | `'readDeviation'` / `'downloadDeviation'` | Servo offsets |
| `UnloadBusServo` | `'servoPowerDown'` | Relax torque |
| `LoadFunc`/`StartFunc`/`StopFunc`/`FinishFunc`/`GetRunningFunc`/`Heartbeat` | `0..12` | Start/stop pre-coded AI behaviors |
| `SetLABValue`/`GetLABValue`/`SaveLABValue` | color thresholds | Vision tuning |
| `SetPoint`/`SetThreshold`/`GetRGBValue` | per-game params | Vision games |

Action-group numbers are mapped in `ActionGroupDict.py` (e.g. `1=go_forward`,
`7=turn_left`, `8=turn_right`, `11=squat`, `24=stepping`). The call graph:
`RPCServer` → `AGC.runActionGroup` → `runAction` opens a **SQLite `.d6a` file** in
`/home/pi/TonyPi/ActionGroups/`, reads table `ActionGroup` (each row = duration +
16 servo positions), and streams `Controller.set_bus_servo_pulse()` calls over
the serial port.

### Video
`MjpgServer.py` serves an HTTP **MJPEG stream on port 8080**; `GET /?action=snapshot`
returns a single JPEG. Used by the PC app and the mobile **WonderPi** app.

### App
Android/iOS **WonderPi** app for direct (AP) and LAN control; also a Qt PC
action editor.

---

## 3. ROS interface (topics/services/actions)

**On TonyPi: none.** There is no ROS 2 graph, no message packages, no
`geometry_msgs/Twist` (`cmd_vel`), and no `nav_msgs/Odometry`. Motion is
**action-group / discrete-servo** based, not velocity based.

For contrast, the ROS-native Hiwonder **MentorPi** platform does expose ROS 2
interfaces (Humble), which is what a TonyPi port would have to imitate. From
`driver/ros_robot_controller/ros_robot_controller/ros_robot_controller_node.py`
(private `~/` names relative to the node):

- Publishes: `~/imu_raw` (`sensor_msgs/Imu`), `~/joy` (`sensor_msgs/Joy`),
  `~/sbus` (`ros_robot_controller_msgs/Sbus`), `~/button`
  (`ButtonState`), `~/battery` (`std_msgs/UInt16`).
- Subscribes: `~/set_led`, `~/set_buzzer`, `~/set_oled`, `~/set_motor`
  (`MotorsState`), `~/enable_reception` (`std_msgs/Bool`),
  `~/bus_servo/set_state` (`SetBusServoState`),
  `~/bus_servo/set_position` (`ServosPosition`),
  `~/pwm_servo/set_state` (`SetPWMServoState`), `~/set_rgb` (`RGBStates`).
- Services: `~/bus_servo/get_state` (`GetBusServoState`),
  `~/pwm_servo/get_state` (`GetPWMServoState`), `~/init_finish`
  (`std_srvs/Trigger`).
- Chassis/nav (MentorPi only): `/controller/cmd_vel` (`geometry_msgs/Twist`)
  consumed by `driver/controller/.../odom_publisher_node.py`, which publishes
  `odom_raw` (`nav_msgs/Odometry`); the nav stack uses `/odom`. **TonyPi has no
  wheels, no encoder motors, and none of this.**
- Hiwonder's ROS humanoid, **AiNex**, is ROS 1 (Noetic) and 24-DOF, with
  `ainex_kinematics` for IK/gait — again, not part of TonyPi.

---

## 4. Distributed communication (external x86_64 ↔ TonyPi)

- **A stock TonyPi has no DDS participant.** There is no RMW to match and no ROS
  domain to join. The external stack talks to it with ordinary TCP:
  JSON-RPC on `:9030` and MJPEG on `:8080`, over the `HW...` AP or the site LAN.
  Both servers bind to `''` (all interfaces) with **no authentication in code**.
- **No RMW/DDS configuration exists on TonyPi** (no `RMW_IMPLEMENTATION`,
  `ROS_DOMAIN_ID`, or Cyclone/Fast-DDS config). Hiwonder's ROS products likewise
  ship no explicit RMW config; ROS 2 Humble on Ubuntu defaults to
  `rmw_fastrtps_cpp`.
- **Humble ↔ Jazzy (only if you add ROS on the Pi).** DDS is a wire protocol, so
  different ROS distros *can* interoperate when both sides use the same DDS/RMW,
  the same `ROS_DOMAIN_ID`, a shared network, and compatible IDL/type hashes.
  This is not an officially guaranteed compatibility matrix, and
  message-definition/typehash drift between distros is the usual failure mode.
  ROS 2 documents middleware selection and the multiple-RMW workflow
  (<https://docs.ros.org/en/jazzy/Concepts/Intermediate/About-Different-Middleware-Vendors.html>,
  <https://docs.ros.org/en/jazzy/Tutorials/Advanced/Working-with-multiple-RMW-implementations.html>).
  Because TonyPi ships no ROS, this repo's `rmw_cyclonedds_cpp` on Jazzy is only
  relevant to whatever bridge *you* deploy, not to the robot.
- Cross-distro is thus a **build/deployment** risk (you'd install Jazzy on the
  Pi 5, or run Humble in a container on the Pi and bridge), not a robot-provided
  capability.

---

## 5. Minimum viable adapter shape

There is no ROS graph to bridge to, so the adapter wraps the **existing** remote
surface. Three viable shapes:

| Shape | Where it runs | Robot-side changes | Sees low-rate state | Notes |
|---|---|---|---|---|
| **A. Remote RPC bridge** | Control PC (x86_64, Jazzy) | none | `RunAction`, servo pulse readback, MJPEG | Simplest; pure HTTP client; no serial, no IMU stream, no odometry |
| **B. On-Pi ROS 2 node** | Raspberry Pi 5 | install ROS 2 + write node | full SDK: IMU, servo temps, `get_imu` | Needs ROS on Pi OS; heavyweight but gives real telemetry and local control |
| **C. Two-tier** | PC bridge + small on-Pi agent | run a Python agent (systemd) | agent forwards IMU/servo state; PC sends high-level | Best balance; agent reuses `hiwonder` SDK, PC keeps ROS 2 Jazzy |

**Recommendation:** start with **C**, or A if you only need scripted action
groups. The RPC server is at `:9030` with app-oriented, string-typed methods and
no continuous velocity command; the serial SDK (which has the IMU and true servo
telemetry) is only reachable from the Pi.

If you want ROS 2 "natively" on the robot, install ROS 2 on the Pi (Jazzy on
64-bit Pi OS is not a Hiwonder-supported configuration) **or** run a Humble
container on the Pi that bridges the SDK, as the community did for the ROS 1
AiNex (`thinknewdev/ainex-ros2`, a Humble port with a standalone `motiond`
daemon and ROS-free shims; `foundway/AinexPsi` notes Hiwonder *"has no plan for
ROS 2 support"* on AiNex).

---

## 6. High-level motion vs low-level servo/action groups

- **TonyPi provides no high-level "walk to coordinate" API.** The highest-level
  motion primitives are:
  - **named action groups** (`go_forward`, `back`, `turn_left`, `turn_right`,
    `left_move_fast`, `right_move_fast`, `squat`, `stand`, ... ), which are
    pre-recorded 16-servo keyframe sequences in SQLite `.d6a` files; and
  - **raw bus/PWM servo pulse commands** with a duration.
- There is **no velocity command, no odometry, no pose feedback, no map frame.**
  The only feedback is servo pulse/voltage/temperature reads and a raw 6-axis
  IMU (`get_imu`) that is only accessible on the Pi over the serial link.
- Therefore a **`MoveToPose`-style primitive would have to be built from**:
  1. an action-group finite state machine (`go_forward`/`turn_left` with
     time- or vision-based stopping), and/or
  2. a servo-space inverse-kinematics layer (implemented by you; TonyPi ships no
     IK, unlike AiNex's `ainex_kinematics`), plus
  3. IMU and vision feedback (AprilTag detection exists in the SDK) for
     closed-loop correction.
  Robust metric `MoveToPose` is **not realistically achievable** without new
  kinematics and state estimation.

---

## What I could not verify

- **Exact base OS release** (Raspberry Pi OS Bullseye vs Bookworm, 32/64-bit).
  The image is not published; it is obtained by emailing Hiwonder support.
- **Whether any TonyPi revision ships any ROS** — no evidence it does; the repo
  and docs contain none. Stated as a negative finding, not a vendor guarantee.
- **Pi 4B vs Pi 5 across board revisions.** Current listings and docs say Pi 5
  for both TonyPi and TonyPi Pro, but older cached TonyPi Pro listings said
  Raspberry Pi 4B. Verify against the specific unit's serial/order.
- **Servo model numbers** (e.g. LX-15D / LX-824HV). Docs only say "high-voltage
  intelligent bus servos."
- **Official cross-distro ROS 2 guidance from Hiwonder** — none exists for
  TonyPi. The Humble↔Jazzy notes are ROS-community/general, not vendor-tested.
- **ROS Index presence.** `index.ros.org` returned no TonyPi (or Hiwonder
  robot) entry; the ROS packages live only in Hiwonder's GitHub repos and are
  versioned `0.0.0` (`ros_robot_controller`, `ros_robot_controller_msgs`,
  `controller`, `interfaces`). I could not use the index search endpoint
  directly (it returned non-2xx), so treat "not indexed" as strongly likely, not
  proven.
- **RPC security defaults** beyond reading the code (binds all interfaces, no
  auth, no TLS). Whether the shipped image adds firewall rules was not examined.

---

## Adapter recommendation (concise)

- **Where it runs.** Put the adapter **on the control PC** for high-level action
  groups (pure HTTP client to `:9030`, MJPEG from `:8080`), and add a **small
  long-lived Python agent on the Pi** if you need low-rate telemetry (IMU, servo
  voltage/temperature) or tighter timing. Do **not** expect a DDS peer.
- **Can it be Rust?** Yes for the PC side: an HTTP/JSON-RPC client (e.g.
  `reqwest`/`ureq`) plus an MJPEG consumer, behind the existing `RobotBackend`
  trait, is idiomatic and testable. The **on-Pi agent should stay Python** to
  reuse Hiwonder's hardware-validated `hiwonder` SDK; have it expose a tiny
  local HTTP/UDP JSON API to the Rust node. A pure-Rust reimplementation of the
  `0xAA 0x55 … CRC-8` serial protocol is technically possible but re-creates a
  validated driver — do that only if the Python dependency is unacceptable.
- **Realistic primitives.** `RunActionGroup`/stop, bus/PWM servo position writes,
  servo readback (position/voltage/temp), IMU read (via on-Pi agent),
  buzzer/LED/RGB, camera snapshot/stream, and a simple action-group sequencer.
  Also implementable: "walk forward N steps / turn ~90°" as bounded action-group
  compositions with IMU or vision termination.
- **Not realistically implementable without new work.** Metric `MoveToPose`,
  velocity control, odometry, and reliable closed-loop navigation.
- **Main risks.**
  1. No ROS, no odometry, no velocity: this breaks assumptions in a stack built
     for the GO2 (`cmd_vel`/odom/state feedback).
  2. Action groups are **discrete, blocking, and only weakly observable**; there
     is no completion signal surfaced through the RPC.
  3. The high-value sensor/feedback path (IMU, precise servo state) is **serial
     to the Pi**, forcing code onto the robot and into Python.
  4. **1 Mbaud serial + `time.sleep`-driven motion** is timing-sensitive;
     running a Pi-side agent concurrently with Hiwonder's own `tonypi.service`
     risks serial contention (the service must be stopped or the agent must own
     the port).
  5. **No auth/TLS** on `:9030`/`:8080`.
  6. **Vendor/version drift** (Pi 4B→5, undocumented OS image, firmware version
     tool, `servo_config.yaml` offsets) means the adapter must be pinned and
     validated per unit.
