#!/usr/bin/env bash
# Publish signed .deb artifacts to an aptly-managed apt repository.
#
# Usage:
#   APTLY_REPO=ros APTLY_DIST=jazzy GPG_KEY=<key-id> \
#     deploy/apt/publish.sh build/debs/*.deb
#
# Prerequisites (on the CI/packaging host, never on the robot):
#   - aptly installed
#   - a GPG key imported for signing
set -euo pipefail

REPO="${APTLY_REPO:-ros}"
DIST="${APTLY_DIST:-jazzy}"
COMPONENT="${APTLY_COMPONENT:-main}"
GPG_KEY="${GPG_KEY:?set GPG_KEY to the signing key id}"
SNAPSHOT="${APTLY_SNAPSHOT:-${REPO}-$(date -u +%Y%m%dT%H%M%SZ)}"

if [ "$#" -lt 1 ]; then
  echo "usage: $0 <package.deb> [more.deb ...]" >&2
  exit 1
fi

if ! aptly repo show "$REPO" >/dev/null 2>&1; then
  aptly repo create -distribution="$DIST" -component="$COMPONENT" "$REPO"
fi

aptly repo add "$REPO" "$@"
aptly snapshot create "$SNAPSHOT" from repo "$REPO"
aptly publish snapshot -batch -gpg-key="$GPG_KEY" "$SNAPSHOT" "$DIST"

echo "Published snapshot '$SNAPSHOT' to $DIST (component $COMPONENT)."
echo "Robots pin the exact snapshot/version via /etc/apt/preferences.d/ros."
