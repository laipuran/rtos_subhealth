# Industrial Control Plane Design

## Purpose

Turn the Rust/ROS 2 migration into a dependable operator control plane for
mobile robot task execution, map administration, and physiological monitoring.
The primary users are field operators and site engineers. They need an accurate
view of device availability, task state, map safety, and diagnostic alerts;
they must not need to understand ROS transport details.

## Architecture

The gateway is the sole production web entry point and the web-domain boundary.
It validates commands, persists operator-visible state, and exposes a stable
REST/WebSocket protocol. The ROS bridge is an edge adapter that translates
canonical gateway commands and events to ROS actions and topics. It contains no
web compatibility policy.

```text
WebUI -> Gateway API/state projection -> Bridge adapter -> ROS actions/topics
                 ^                         |
                 +---- persisted events ---+
```

The standalone mock gateway remains a development and contract-test executable.
Production deployment runs `gateway_bridge` only.

## Command Model

The canonical task command is the device-oriented shape already represented by
`DeviceTask`: `device_id`, `primitive`, `target`, `params_json`, constraints,
and deadline. The gateway rejects incomplete canonical commands.

The legacy WebUI shape (`goal.type`, `target_tags`) is retained temporarily at
the HTTP edge only. Its conversion is explicit and total: a `go_to_tag` becomes
`move_to_pose` with a tag target, `hold` becomes `hold`, and patrol requires an
explicit route conversion. Unsupported or ambiguous legacy input returns an
API error; no missing field is silently converted to `hold`.

## State and Events

The gateway persists accepted, running, terminal, and cancellation transitions.
Each WebSocket event carries a monotonic per-process sequence number. Clients
use one multiplexed socket and resync REST resources after reconnect or a
sequence gap. Task results and lifecycle state are durable; telemetry remains
best-effort.

## Security and Operations

Map scene identifiers use a restricted identifier grammar and maps are written
with a temporary file plus rename. Browser authentication uses the existing API
token through an explicit REST header and WebSocket authentication message.
Hardware RPC has bounded connect, read, and write operations. Safety health is
fed by device/backend communication rather than a self-refreshing timer.

## Operator Experience

The WebUI submits canonical commands, presents device availability and task
failure reasons, maintains a single event connection, and preserves named map
routes. The current task, map, and diagnosis workflows remain available while
their data source becomes the canonical gateway model.

## Delivery Phases

1. Secure and normalize the gateway command, map, and event boundaries.
2. Complete task lifecycle forwarding and durable state projection in the ROS
   bridge.
3. Connect diagnosis and vitals topics to gateway persistence and events.
4. Upgrade the WebUI to canonical commands, one authenticated event stream, and
   lossless map editing.
5. Add Rust contract tests, ROS integration tests, and CI execution for the
   critical production paths.

## Acceptance Criteria

- A navigation request reaches ROS as `move_to_pose` with its tag target.
- Cancelled tasks request ROS cancellation and reach a terminal persisted state.
- Scene traversal is rejected and map route definitions survive editing.
- Authenticated WebUI REST and WebSocket traffic works with a configured token.
- Diagnosis and vitals events are persisted or delivered through the gateway.
- Gateway, WebUI, and ROS integration tests exercise these paths in CI.
