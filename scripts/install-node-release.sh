#!/usr/bin/env bash
# Install a previously built node runtime and safely advance current.
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

CURRENT="$ROOT/current"
if [ -e "$CURRENT" ] && [ ! -L "$CURRENT" ]; then
  die "runtime current path is not a symlink: $ROOT/current"
fi
NEXT_LINK="$ROOT/.current.$$"
PREVIOUS_LINK="$ROOT/.current.previous.$$"
current_moved=0
restore_current() {
  rm -f "$NEXT_LINK"
  if [ "$current_moved" -eq 1 ] \
    && [ ! -e "$CURRENT" ] \
    && [ -L "$PREVIOUS_LINK" ]; then
    mv "$PREVIOUS_LINK" "$CURRENT" || true
  fi
  rm -f "$PREVIOUS_LINK"
}
trap restore_current EXIT

ln -s "releases/$VERSION" "$NEXT_LINK"
if [ -e "$CURRENT" ] || [ -L "$CURRENT" ]; then
  mv "$CURRENT" "$PREVIOUS_LINK"
  current_moved=1
fi
mv "$NEXT_LINK" "$CURRENT"
current_moved=0
rm -f "$PREVIOUS_LINK"
trap - EXIT

echo "installed node runtime: $RELEASE_DIR"
echo "source revision: $GIT_SHA"
echo "current: $ROOT/current"
echo "restart supervised services explicitly to adopt this release"
