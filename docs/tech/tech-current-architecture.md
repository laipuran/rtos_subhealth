# Current architecture (authoritative)

> This is the up-to-date description. Where older docs (RFCs, `docs/guide/`)
> disagree, this document and the ADRs win. The old Python / ROS 2 Foxy
> implementation is kept under `legacy/` for reference only.

## Platform

- ROS 2 **Jazzy** on Ubuntu 24.04 (external control PC), `rmw_cyclonedds_cpp`.
- Devices keep their vendor-supported OS/ROS; we interoperate over DDS messages.
- All ROS/Rust tooling runs in Docker (`docker/dev`); nothing is installed on the
  host.

## Language scope

Rust by default; C++ for real-time / vendor C++ SDK adapters; Python only for
edge adapters and tooling. See `docs/tech/adr-001-language-scope.md`.

## Components

| Layer | Crate / package | Notes |
|---|---|---|
| HTTP/WS gateway | `services/gateway` + `ros2_ws/src/robot/gateway_bridge` | axum; `gateway_bridge` bridges to ROS |
| Diagnosis core | `services/diagnosis` | aggregation, anomaly, prompt, JSON schema |
| Diagnosis node | `ros2_ws/src/robot/diagnosis_node` | subscribes `/physio/*`, publishes `/diagnosis/*` |
| Physio mock | `ros2_ws/src/robot/physio_mock` | 6 sensors at 1 Hz |
| Orchestration core | `services/orchestrator-core` | registry, capability checks, task lifecycle |
| Orchestrator node | `ros2_ws/src/robot/orchestrator` | discovery + routing to adapters |
| Device adapter SDK | `services/device-sdk` | `DeviceBackend`, mock / diff-drive / TonyPi |
| Adapter node | `ros2_ws/src/robot/adapter` | `DEVICE_TYPE=mock|diff_drive|tonypi` |
| World model / planner | `services/world-model` | graph + Dijkstra |
| Safety | `services/safety` | limits, watchdog, e-stop |
| Perception | `services/perception-sim` + `ros2_ws/src/robot/perception_sim` | geometry AprilTag detection |
| WebUI | `webui/` | served by the gateway |

## Interfaces (rosidl, no prefix)

`task_interfaces` (`DeviceTask`, `ExecTask`, `PlanPath`), `device_interfaces`
(`DeviceDescriptor`, `DeviceState`, `TaskTarget`, primitives),
`perception_interfaces`, `diagnosis_interfaces`.

## Data flow

```
WebUI --HTTP/WS--> gateway(gateway_bridge)
                        |  DeviceTask action
                        v
                   orchestrator ---- /<device_id>/device_task ----> adapter
                        ^                                            |
                        +------------- feedback / result ------------+
physio_mock --/physio/*--> diagnosis_node --/diagnosis/*--> gateway
perception_sim --/perception/apriltag_detections--> (control)
```

## Build & run

```bash
# pure Rust services + tests (host)
cargo test --workspace

# build all ROS nodes (Jazzy container)
docker build -t ros-dev:jazzy docker/dev
docker run --rm -v "$PWD":/workspace -w /workspace ros-dev:jazzy \
  bash docker/dev/build_ros_rust.sh
```

## Deployment

systemd units under `deploy/systemd/`: `orchestrator.service`,
`adapter@.service` (per device), `gateway_bridge.service`. Config in
`deploy/config/`, secrets via systemd credentials. Per-package `.deb` + signed
apt + RAUC A/B are scaffolded in `deploy/`.
