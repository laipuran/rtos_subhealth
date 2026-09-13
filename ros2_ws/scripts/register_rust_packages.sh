#!/usr/bin/env bash
# Prepare Rust message-crate discovery for colcon-ros-cargo.
#
# rosidl_generator_rs currently does not publish generated crates into the
# `rust_packages` ament index, so colcon-ros-cargo cannot resolve them as
# Cargo patches. This script registers the generated crates after a build.
#
# Usage: source the ROS install first, then:
#   scripts/register_rust_packages.sh <install-prefix>
set -euo pipefail

PREFIX="${1:?usage: $0 <install-prefix>}"
RIDX="$PREFIX/share/ament_index/resource_index/rust_packages"
mkdir -p "$RIDX"

for pkgdir in "$PREFIX"/share/*/rust; do
  [ -f "$pkgdir/Cargo.toml" ] || continue
  pkg=$(basename "$(dirname "$pkgdir")")
  touch "$RIDX/$pkg"
  echo "registered rust package: $pkg"
done
