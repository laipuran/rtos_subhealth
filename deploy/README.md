# Packaging and deployment

The system is delivered as installable units. Nothing is compiled on the robot.

## Build artifacts

| Artifact | Tool | Source |
|---|---|---|
| ROS interface packages (`.deb`) | `bloom-generate rosdebian` + `fakeroot debian/rules binary` | each package under `ros2_ws/src/robot/interfaces/` |
| Rust services (`.deb`) | `cargo deb -p <crate>` | `services/gateway`, `services/diagnosis`, and the Phase-3 node crates |
| WebUI | `pnpm build`, then install `webui/dist` to `/usr/share/ros-webui` | `webui/` |

Build everything inside the pinned Jazzy container so artifacts match the
target runtime:

```bash
docker build -t ros-dev:jazzy docker/dev
docker run --rm -v "$PWD":/workspace -w /workspace ros-dev:jazzy bash -lc '
  source /opt/ros/jazzy/setup.bash
  colcon build --base-paths ros2_ws/src/robot
  cargo deb -p gateway -p diagnosis
'
```

## Interface package release

```bash
cd ros2_ws/src/robot/interfaces/task_interfaces
bloom-generate rosdebian --os-name ubuntu --os-version jammy --ros-distro jazzy
fakeroot debian/rules binary
```

Interface packages are versioned independently. Any field addition/removal is
gated by the interface-compat CI job.

## Distribution

Publish `.deb`s to the signed apt repository with `deploy/apt/publish.sh`, then
pin exact versions per robot via `/etc/apt/preferences.d/ros`.

## Target endpoint install

```bash
curl -fsSL https://apt.example.com/ros-key.gpg \
  | sudo tee /etc/apt/keyrings/ros.gpg >/dev/null
echo "deb [signed-by=/etc/apt/keyrings/ros.gpg] https://apt.example.com/ros jazzy main" \
  | sudo tee /etc/apt/sources.list.d/ros.list
sudo apt update
sudo apt install gateway
sudo systemctl enable --now gateway
```

## Configuration and secrets

- Non-secret defaults: `deploy/config/gateway.env` -> `/etc/ros/gateway.env`.
- Secrets: systemd credentials, e.g.
  `systemd-creds encrypt /etc/ros/api-token /etc/ros/api-token.cred`.
- Mutable state lives under `/var/lib/ros` (separate partition) so A/B rootfs
  updates never touch it.
