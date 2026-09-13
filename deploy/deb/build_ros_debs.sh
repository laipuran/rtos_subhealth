#!/usr/bin/env bash
# Build a Debian package from a built ROS workspace (Jazzy).
#
# Usage:
#   deploy/deb/build_ros_debs.sh <nodes_install_prefix> <interfaces_install_prefix> [out_dir]
#
# Example (inside the dev container, after docker/dev/build_ros_rust.sh):
#   deploy/deb/build_ros_debs.sh /tmp/ros_rust_ws/install_nodes \
#                                /tmp/ros_rust_ws/install dist
set -eo pipefail

NODES_PREFIX="${1:?usage: $0 <nodes_prefix> <interfaces_prefix> [out_dir]}"
IFACES_PREFIX="${2:?usage: $0 <nodes_prefix> <interfaces_prefix> [out_dir]}"
OUT_DIR="${3:-dist}"
VERSION="${VERSION:-0.1.0}"
PKG="ros-subhealth-nodes"
ARCH="${ARCH:-amd64}"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

mkdir -p "$STAGE/opt/ros-subhealth/nodes" "$STAGE/opt/ros-subhealth/interfaces" \
         "$STAGE/lib/systemd/system" "$STAGE/etc/ros" "$STAGE/usr/bin" "$STAGE/DEBIAN"

cp -r "$NODES_PREFIX/." "$STAGE/opt/ros-subhealth/nodes/"
cp -r "$IFACES_PREFIX/." "$STAGE/opt/ros-subhealth/interfaces/"
# Prune build-time only payload (headers, CMake configs) to keep the package small.
rm -rf "$STAGE/opt/ros-subhealth/interfaces/include" \
       "$STAGE/opt/ros-subhealth/nodes/include"
find "$STAGE/opt/ros-subhealth" -type d -name cmake -prune -exec rm -rf {} + 2>/dev/null || true
cp "$REPO_ROOT"/deploy/systemd/*.service "$STAGE/lib/systemd/system/"
cp "$REPO_ROOT"/deploy/config/*.env "$STAGE/etc/ros/"

# Environment shared by all nodes: ament prefix + shared library paths.
cat > "$STAGE/etc/ros/ros-subhealth.env" <<EOF
AMENT_PREFIX_PATH=/opt/ros-subhealth/interfaces:/opt/ros-subhealth/nodes:/opt/ros/jazzy
LD_LIBRARY_PATH=/opt/ros-subhealth/interfaces/lib:/opt/ros-subhealth/nodes/lib:/opt/ros/jazzy/lib
PYTHONPATH=/opt/ros-subhealth/interfaces/lib/python3.12/site-packages:/opt/ros/jazzy/lib/python3.12/site-packages
EOF

# Wrapper that sources the ROS + workspace environments before running a node.
cat > "$STAGE/usr/bin/ros-subhealth-run" <<'EOF'
#!/usr/bin/env bash
set -e
. /opt/ros/jazzy/setup.bash
if [ -f /opt/ros-subhealth/interfaces/setup.bash ]; then . /opt/ros-subhealth/interfaces/setup.bash; fi
if [ -f /opt/ros-subhealth/nodes/setup.bash ]; then . /opt/ros-subhealth/nodes/setup.bash; fi
exec "$@"
EOF
chmod 755 "$STAGE/usr/bin/ros-subhealth-run"

# Expose node binaries on PATH (link to the installed prefix, not the staging dir).
find "$STAGE/opt/ros-subhealth/nodes/lib" -mindepth 2 -maxdepth 2 -type f -executable | while read -r bin; do
    target="${bin#"$STAGE"}"
    ln -sf "$target" "$STAGE/usr/bin/$(basename "$bin")"
done

cat > "$STAGE/DEBIAN/control" <<EOF
Package: $PKG
Version: $VERSION
Section: misc
Priority: optional
Architecture: $ARCH
Maintainer: ROS Subhealth <dev@ros-subhealth.local>
Depends: ros-jazzy-rmw-cyclonedds-cpp, ros-jazzy-rclcpp
Description: ROS 2 Subhealth nodes, interfaces and systemd units.
EOF

mkdir -p "$OUT_DIR"
dpkg-deb --build --root-owner-group "$STAGE" "$OUT_DIR/${PKG}_${VERSION}_${ARCH}.deb"
echo "[deb] wrote $OUT_DIR/${PKG}_${VERSION}_${ARCH}.deb"
