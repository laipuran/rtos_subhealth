# ADR-001: Language scope (Rust / C++ / Python)

- **Status:** Accepted
- **Date:** 2026-09-13

## Context

The system is migrating from a Python-only ROS 2 Foxy stack to a multi-device
robot control platform (wheeled bases, biped/humanoid, others). Devices may be
servo-based with no ROS graph at all (e.g. TonyPi exposes a Python SDK + HTTP
JSON-RPC and discrete action groups), while others may offer C++ or Python
vendor SDKs or native `rclcpp` drivers.

We need one rule that keeps the platform maintainable across devices whose SDKs
are written in different languages.

## Decision

**Language is an implementation detail behind language-neutral ROS 2 interfaces.
There is no cross-language in-process FFI.**

| Layer | Language |
|---|---|
| WebUI | TypeScript |
| Gateway, diagnosis | Rust |
| Orchestration, planning, execution core, safety policy | Rust |
| ROS nodes (rclrs) | Rust |
| Device adapters | Rust by default; C++ or Python when the vendor SDK or hard real-time requires it |
| Real-time control loops / `ros2_control` / Nav2 components | C++ |
| Tests, simulation, data and CI tooling | Rust preferred; Python allowed |

Hard rules:

1. The device contract is ROS 2 messages/actions/services. An adapter in any
   language satisfies it.
2. Adapters run as separate processes and communicate over DDS; no shared
   headers, no FFI.
3. One deployable artifact uses one language.
4. New ROS core code defaults to Rust. A C++/Python adapter must record its
   justification (vendor SDK / real-time / required `rclcpp` ecosystem) in an ADR.
5. Python must not implement core request-path logic (orchestration, planning,
   execution, safety, persistence). It may implement adapters and tooling, and
   must be replaceable without changing the contract.
6. No language or process assumptions may leak into message fields.

## Consequences

- A Python-only vendor SDK becomes a thin edge adapter, replaceable later.
- Core logic stays in one toolchain (Cargo workspace), unit-testable off-robot.
- Cross-distro support is possible because only messages cross process/ROS
  boundaries: the control PC runs Jazzy; devices keep their vendor-supported ROS.
- We accept maintaining two ROS client libraries (rclrs for core, rclcpp for
  C++ adapters) where necessary; they interoperate over DDS.
- Rust ROS (`rclrs`) is pre-1.0, so the version is pinned and covered by CI.
