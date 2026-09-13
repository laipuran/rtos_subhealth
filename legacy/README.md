# Legacy (retired) implementation

The original Python / ROS 2 Foxy implementation has been **removed from the
tree** now that the Rust-first architecture replaces it. The code is still
available in git history (it was deleted in a commit on
`feat.duckran.migration`).

This file is retained as historical documentation of what was replaced.

| Legacy | Replacement |
|---|---|
| `desc_layer` (Flask HTTP/WS + SQLite) | `services/gateway` + `ros2_ws/src/robot/gateway_bridge` |
| `diagnosis_layer` (Python aggregation/RAG/LLM) | `services/diagnosis` + `ros2_ws/src/robot/diagnosis_node` |
| `exec_layer` (FSM + planner + robot backends) | `services/orchestrator-core`, `services/world-model`, `ros2_ws/src/robot/orchestrator`, `ros2_ws/src/robot/adapter` |
| `mock_exec_layer` | `ros2_ws/src/robot/adapter` (`DEVICE_TYPE=mock`) |
| `physio_mock_publisher` | `ros2_ws/src/robot/physio_mock` |
| `apriltag_perception` / `camera_test_publisher` | `ros2_ws/src/robot/perception_sim` (sim) and `perception_camera` (real) |
| `ros_interfaces` / `apriltag_interfaces` / `physio_interfaces` | `ros2_ws/src/robot/interfaces/*` |
| `run.sh`, `setup.sh`, `Makefile`, `config-params` | `deploy/`, `docker/`, systemd units |
| `map_to_mjmodel.py`, `test/*` | not ported; simulation is not on the critical path |

See `docs/tech/adr-001-language-scope.md` and
`docs/tech/tech-current-architecture.md` for the current design.
