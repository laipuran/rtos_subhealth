# exec_layer

Execution layer action server skeleton for RFC003/004.

## Scope
- Action server for the single-entry task flow (`ExecTask`).
- Calls planner service (`PlanPath`) and executes returned segments.
- Publishes feedback and final result per RFC field semantics.

## Non-Goals
- Does not implement actual motion control or perception.
- Does not implement planner logic.
