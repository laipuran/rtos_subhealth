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
rm -rf "$WS/repo"  # clean any earlier in-workspace node copy
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
  rosidl_generator_rs geometry_msgs sensor_msgs \
  device_interfaces task_interfaces perception_interfaces diagnosis_interfaces

# shellcheck disable=SC1091
source install/setup.bash
bash "$REPO_ROOT/ros2_ws/scripts/register_rust_packages.sh" "$WS/install"

# colcon-ros-cargo writes a generated `.cargo/config.toml` next to the build
# cwd. Build the nodes from a throwaway copy under the workspace volume so that
# file never lands in the mounted repo (a stray ROS patch config breaks
# host-side tools such as rust-analyzer).
NODE_ROOT="${ROS_RUST_NODES:-/tmp/ros_rust_nodes}"
NODE_SRC="$NODE_ROOT/ros2_ws/src/robot"
rm -rf "$NODE_ROOT"
mkdir -p "$NODE_SRC"
# The service crates inherit from the workspace root, so copy it as well.
cp "$REPO_ROOT/Cargo.toml" "$NODE_ROOT/Cargo.toml"
[ -f "$REPO_ROOT/Cargo.lock" ] && cp "$REPO_ROOT/Cargo.lock" "$NODE_ROOT/Cargo.lock"
[ -f "$REPO_ROOT/rust-toolchain.toml" ] && cp "$REPO_ROOT/rust-toolchain.toml" "$NODE_ROOT/rust-toolchain.toml"
cp -r "$REPO_ROOT/services" "$NODE_ROOT/services"
for pkg in adapter orchestrator physio_mock diagnosis_node perception_sim perception_camera gateway_bridge; do
  cp -r "$REPO_ROOT/ros2_ws/src/robot/$pkg" "$NODE_SRC/$pkg"
done

cd "$NODE_SRC"
colcon build --merge-install \
  --build-base "$WS/build_nodes" --install-base "$WS/install_nodes"

echo "[OK] Rust ROS nodes built into $WS/install_nodes"
