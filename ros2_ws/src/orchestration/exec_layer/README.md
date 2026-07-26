# exec_layer

Execution layer action server skeleton for RFC003/004, with integrated state machine.

## Scope
- Action server for the single-entry task flow (`ExecTask`).
- Calls planner service (`PlanPath`) and executes returned segments.
- Publishes feedback and final result per RFC field semantics.
- Manages task lifecycle via `ExecFSM` (`exec_layer/fsm.py`).

## State Machine

States: `idle → accepted → running → succeeded/failed/canceled`, with `paused` reserved.

Powered by the [transitions](https://github.com/pytransitions/transitions) library.

## Dependencies
- Python: `transitions`, `rclpy`, `ros_interfaces`, `builtin_interfaces`

## Non-Goals
- Does not implement actual motion control or perception.
- Does not implement planner logic.
