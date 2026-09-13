# ADR-002: Device contract and single-task-first orchestration

- **Status:** Accepted
- **Date:** 2026-09-13

## Context

The legacy execution layer assumed a Unitree GO2: `stand_up`/`damp`/
`recovery_stand`, a `moving → approaching → aligning → stabilizing` state
machine, and tag-graph Dijkstra. It does not generalize to a wheeled cart, a
servo humanoid, or future devices. The first target device (TonyPi) exposes only
named action groups and raw servo pulses — no velocity, no odometry, no
`cmd_vel`.

## Decision

1. **Capability-based device contract.** Every adapter advertises a
   `DeviceDescriptor` (kind, capabilities, supported primitives, limits, frames,
   sensors) and periodic `DeviceState`. The orchestrator routes to devices by
   advertised capability, never by assumed hardware.
2. **Primitives are optional.** Common primitives are `move_to_pose`,
   `set_velocity`, `hold`, `stop`. Device-specific behavior uses the generic
   `execute_primitive` (name + `params_json`) so new devices do not need new ROS
   messages. A servo humanoid implements only `execute_primitive`/`hold`/`stop`.
3. **Targets are device-agnostic.** `TaskTarget` supports `tag`, `pose`,
   `waypoint`, and `action`. Tag chaining becomes one localization provider, not
   the core model.
4. **Single-task-first.** The public task contract is a single `DeviceTask`
   (one goal, one device, one primitive). The workflow/step superset is
   deliberately deferred.
5. **Preserved extension points** for a future workflow engine:
   - `TaskTarget` and primitives are reusable as workflow step payloads.
   - Persistence stores the task spec as JSON, so a step list can be added
     without a schema change.
   - `Orchestrator` validation/lifecycle logic is independent of the transport
     and can be driven by a future multi-step scheduler.
   - The HTTP entry point (`POST /api/v1/tasks`) is unchanged.

## Consequences

- A wheeled base and a servo humanoid are handled by the same orchestrator via
  different capability sets.
- `MoveToPose` is not universally available; planners must check capabilities
  before emitting pose-based tasks.
- Deferring the workflow engine avoids speculative DAG complexity while keeping
  a clean seam to add it later.
- The legacy `ExecTask` interface is kept temporarily for compatibility and will
  be retired once the new node stack replaces the Python packages.
