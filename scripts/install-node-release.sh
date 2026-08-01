#!/usr/bin/env bash
# Install a previously built node runtime and atomically advance current.
set -euo pipefail

die() {
  echo "install-node-release: $*" >&2
  exit 1
}

command -v jq >/dev/null 2>&1 || die "jq is required"

sha256_of() {
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    sha256sum "$1" | awk '{print $1}'
  fi
}

PACKAGE="${1:-}"
[ -n "$PACKAGE" ] || die "usage: install-node-release.sh PACKAGE_DIR"
[ -d "$PACKAGE" ] || die "package directory does not exist: $PACKAGE"
[ -f "$PACKAGE/release.json" ] || die "package has no release.json: $PACKAGE"
[ -x "$PACKAGE/bin/buzz" ] || die "package has no buzz executable: $PACKAGE"

VERSION="$(jq -er '.version' "$PACKAGE/release.json")" || die "invalid release manifest"
GIT_SHA="$(jq -er '.git_revision' "$PACKAGE/release.json")" || die "invalid release manifest"
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+([-.+][A-Za-z0-9._-]+)?$ ]] \
  || die "release version is not a safe semantic version: $VERSION"
EXPECTED_SHA256="$(jq -er '.sha256' "$PACKAGE/release.json")" \
  || die "release manifest has no executable hash"
ACTUAL_SHA256="$(sha256_of "$PACKAGE/bin/buzz")"
[ "$ACTUAL_SHA256" = "$EXPECTED_SHA256" ] \
  || die "package executable hash does not match release.json"
ROOT="${BUZZ_RUNTIME_ROOT:-$HOME/.local/lib/buzz}"
RELEASES="$ROOT/releases"
RELEASE_DIR="$RELEASES/$VERSION"

case "$ROOT" in
  /*) ;;
  *) die "runtime root must be absolute: $ROOT" ;;
esac
[ ! -e "$RELEASE_DIR" ] || die "release already installed: $RELEASE_DIR"

mkdir -p "$RELEASES"
STAGING="$(mktemp -d "$ROOT/.node-install.XXXXXX")"
trap 'rm -rf "$STAGING"' EXIT
cp -R "$PACKAGE/." "$STAGING/"
mv "$STAGING" "$RELEASE_DIR"
trap - EXIT

if [ -e "$ROOT/current" ] && [ ! -L "$ROOT/current" ]; then
  die "runtime current path is not a symlink: $ROOT/current"
fi
NEXT_LINK="$ROOT/.current.$$"
ln -s "releases/$VERSION" "$NEXT_LINK"
mv -f "$NEXT_LINK" "$ROOT/current"

echo "installed node runtime: $RELEASE_DIR"
echo "source revision: $GIT_SHA"
echo "current: $ROOT/current"
echo "restart supervised services explicitly to adopt this release"
