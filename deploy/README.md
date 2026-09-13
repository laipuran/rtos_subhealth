# Packaging and deployment

The system is delivered as installable units. Nothing is compiled on the robot.
All packaging targets are wrapped by the root `Makefile` (`make help`).

## Build artifacts

| Artifact | Command | Output |
| --- | --- | --- |
| Rust services (`.deb`) | `make deb` (`SERVICE_DEBS='gateway'`) | `target/debian/*.deb` |
| ROS nodes + interfaces (`.deb`) | `make ros-deb` (after `make ros`) | `dist/ros-subhealth-nodes_<ver>_amd64.deb` |
| ROS interface packages (`.deb`) | bloom, see below | per package |
| WebUI | `make webui` | `webui/dist` (install to `/usr/share/ros-webui`) |

`make deb` uses `cargo-deb` (pinned in the dev image) and runs inside the
container, so the artifacts match the target runtime. Currently only
`services/gateway` carries `[package.metadata.deb]`; add the same stanza to a
service before including it in `SERVICE_DEBS`.

`make ros-deb` packages the already-built ROS workspace (`/ws/install` +
`/ws/install_nodes`), so run `make ros` first.

## Interface package release (advanced)

Interface packages are versioned independently:

```bash
cd ros2_ws/src/robot/interfaces/task_interfaces
bloom-generate rosdebian --os-name ubuntu --os-version noble --ros-distro jazzy
fakeroot debian/rules binary
```

`bloom-generate` / `fakeroot` are not installed in the dev image by default;
install them in a packaging environment (`pip install bloom`, `apt install
fakeroot`) when cutting interface releases.

## Distribution

Publish `.deb`s to the signed apt repository, then pin exact versions per robot
via `/etc/apt/preferences.d/ros`:

```bash
make publish DEBS='dist/*.deb target/debian/*.deb' \
  APTLY_REPO=ros APTLY_DIST=jazzy GPG_KEY=<key-id>
```

This requires `aptly` and a signing GPG key on the packaging host (never on the
robot).

## Target endpoint install

```bash
curl -fsSL https://apt.example.com/ros-key.gpg \
  | sudo tee /etc/apt/keyrings/ros.gpg >/dev/null
echo "deb [signed-by=/etc/apt/keyrings/ros.gpg] https://apt.example.com/ros jazzy main" \
  | sudo tee /etc/apt/sources.list.d/ros.list
sudo apt update
sudo apt install ros-subhealth-nodes gateway
sudo systemctl enable --now orchestrator adapter@mock gateway_bridge
```

## Configuration and secrets

- Non-secret defaults: `deploy/config/*.env` -> `/etc/ros/*.env`
  (`gateway.env`, `adapter-mock.env`, `adapter-tonypi.env`, `ros-subhealth.env`).
- Secrets: systemd credentials, e.g.
  `systemd-creds encrypt /etc/ros/api-token /etc/ros/api-token.cred`.
- Node binaries run through `/usr/bin/ros-subhealth-run`, which sources ROS and
  the installed workspace before `exec`.
- Mutable state lives under `/var/lib/ros` (separate partition) so A/B rootfs
  updates never touch it.
