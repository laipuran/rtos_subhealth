# Legacy (retired) implementation

This directory holds the original Python / ROS 2 Foxy implementation, kept for
reference only. It is outside the build tree (`ros2_ws/src/`) and is not built
by colcon, CI or the deploy tooling.

It is superseded by:

| Legacy | Replacement |
|---|---|
| `ros2_ws/src/orchestration/desc_layer` (Flask) | `services/gateway` (Rust/axum) |
| `ros2_ws/src/orchestration/diagnosis_layer` (Python) | `services/diagnosis` (Rust) |
| `ros2_ws/src/orchestration/exec_layer` (FSM + planner) | `ros2_ws/src/robot/orchestrator` + adapters |
| `ros2_ws/src/orchestration/mock_exec_layer` | `ros2_ws/src/robot/adapter` (`DEVICE_TYPE=mock`) |
| `ros2_ws/src/orchestration/physio_mock_publisher` | not yet ported (planned) |
| `ros2_ws/src/perception/apriltag_perception` | not yet ported (planned) |
| `ros2_ws/config-params`, `run.sh`, `setup.sh`, `Makefile` | `deploy/`, `docker/`, systemd |
| `map_to_mjmodel.py`, `test/` | not yet ported (simulation) |

No new work should target this directory.
