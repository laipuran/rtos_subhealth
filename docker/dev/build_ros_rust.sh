#!/usr/bin/env bash
# Build the Rust ROS nodes (e.g. `adapter`) inside the Jazzy dev image.
#
# This assembles the rosidl_rust toolchain, builds the message packages and
# their Rust crates, registers them for colcon-ros-cargo, then builds the node
# crates. Run it inside the `ros-dev:jazzy` container (or a Jazzy CI job with
# Rust + colcon-cargo installed).
#
#   docker run --rm -v "$PWD":/workspace -w /workspace ros-dev:jazzy \
#     bash docker/dev/build_ros_rust.sh
#
# NOTE: no `set -u`; ROS setup scripts reference unbound variables.
set -eo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WS="${ROS_RUST_WS:-/tmp/ros_rust_ws}"

# shellcheck disable=SC1091
source /opt/ros/jazzy/setup.bash

mkdir -p "$WS/src"
clone() { [ -d "$WS/src/$2" ] || git clone --quiet --depth 1 -b "$1" "$3" "$WS/src/$2"; }

[ -d "$WS/src/rosidl_rust" ] || \
  git clone --quiet --depth 1 https://github.com/ros2-rust/rosidl_rust.git "$WS/src/rosidl_rust"
clone jazzy unique_identifier_msgs https://github.com/ros2/unique_identifier_msgs.git
clone jazzy rcl_interfaces https://github.com/ros2/rcl_interfaces.git
clone jazzy rosidl_core https://github.com/ros2/rosidl_core.git
clone jazzy rosidl_defaults https://github.com/ros2/rosidl_defaults.git
clone jazzy common_interfaces https://github.com/ros2/common_interfaces.git

rm -rf "$WS/src"/device_interfaces "$WS/src"/task_interfaces \
       "$WS/src"/perception_interfaces "$WS/src"/diagnosis_interfaces
cp -r "$REPO_ROOT"/ros2_ws/src/robot/interfaces/* "$WS/src/"

cd "$WS"
colcon build --merge-install --packages-up-to \
  rosidl_generator_rs geometry_msgs \
  device_interfaces task_interfaces perception_interfaces diagnosis_interfaces

# shellcheck disable=SC1091
source install/setup.bash
bash "$REPO_ROOT/ros2_ws/scripts/register_rust_packages.sh" "$WS/install"

# Cargo reads `.cargo/config.toml` from the manifest's ancestors, so run the
# node build from the repo root and clean the generated config afterwards.
trap 'rm -rf "$REPO_ROOT/.cargo"' EXIT
cd "$REPO_ROOT"
colcon build --merge-install \
  --base-paths \
    "$REPO_ROOT/ros2_ws/src/robot/adapter" \
    "$REPO_ROOT/ros2_ws/src/robot/orchestrator" \
    "$REPO_ROOT/ros2_ws/src/robot/physio_mock" \
    "$REPO_ROOT/ros2_ws/src/robot/diagnosis_node" \
    "$REPO_ROOT/ros2_ws/src/robot/perception_sim" \
    "$REPO_ROOT/ros2_ws/src/robot/gateway_bridge" \
  --build-base "$WS/build_nodes" --install-base "$WS/install_nodes"

echo "[OK] Rust ROS nodes built into $WS/install_nodes"
