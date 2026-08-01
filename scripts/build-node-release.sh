#!/usr/bin/env bash
# Build a reproducible, immutable node runtime package from a clean checkout.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

die() {
  echo "build-node-release: $*" >&2
  exit 1
}

command -v cargo >/dev/null 2>&1 || die "cargo is required"
command -v git >/dev/null 2>&1 || die "git is required"
command -v jq >/dev/null 2>&1 || die "jq is required"

sha256_of() {
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    sha256sum "$1" | awk '{print $1}'
  fi
}

if [ "${BUZZ_NODE_ALLOW_DIRTY:-0}" != "1" ] \
  && [ -n "$(git status --porcelain --untracked-files=all)" ]; then
  die "working tree has uncommitted changes; commit or set BUZZ_NODE_ALLOW_DIRTY=1"
fi

VERSION="${BUZZ_NODE_VERSION:-$(cargo metadata --no-deps --format-version 1 \
  | jq -r '.packages[] | select(.name == "buzz-cli") | .version' \
  | head -n 1)}"
[ -n "$VERSION" ] || die "could not determine the node version"

GIT_SHA="$(git rev-parse HEAD)"
TARGET="${BUZZ_NODE_TARGET:-$(rustc -vV | sed -n 's/^host: //p')}"
OUTPUT="${BUZZ_NODE_OUTPUT:-$ROOT/dist/node/$VERSION/$TARGET}"
CARGO_TARGET_ROOT="${CARGO_TARGET_DIR:-$ROOT/target}"
case "$CARGO_TARGET_ROOT" in
  /*) ;;
  *) CARGO_TARGET_ROOT="$ROOT/$CARGO_TARGET_ROOT" ;;
esac

case "$OUTPUT" in
  "$ROOT"/*) ;;
  *) die "output must be inside the repository: $OUTPUT" ;;
esac
[ ! -e "$OUTPUT" ] || die "output already exists: $OUTPUT"

if [ -n "${BUZZ_NODE_TARGET:-}" ]; then
  BUZZ_NODE_VERSION="$VERSION" cargo build --locked --release --target "$TARGET" \
    -p buzz-cli -p buzz-local-relay --bins
  BIN_DIR="$CARGO_TARGET_ROOT/$TARGET/release"
else
  BUZZ_NODE_VERSION="$VERSION" cargo build --locked --release -p buzz-cli -p buzz-local-relay --bins
  BIN_DIR="$CARGO_TARGET_ROOT/release"
fi

STAGING="$(mktemp -d "${TMPDIR:-/tmp}/buzz-node-release.XXXXXX")"
trap 'rm -rf "$STAGING"' EXIT
mkdir -p "$STAGING/bin" "$STAGING/scripts"

for binary in \
  buzz \
  buzz-local-relay \
  buzz-artifact-sync \
  buzz-handoff-state \
  buzz-pulse-watch \
  buzz-relay-pull \
  buzz-relay-push; do
  [ -x "$BIN_DIR/$binary" ] || die "missing release binary: $BIN_DIR/$binary"
  cp "$BIN_DIR/$binary" "$STAGING/bin/$binary"
done

for script in buzz-ctx buzz-ctx-graph buzz-ctx-graph.jq buzz-drain; do
  [ -f "$ROOT/scripts/$script" ] || die "missing runtime script: scripts/$script"
  cp "$ROOT/scripts/$script" "$STAGING/scripts/$script"
done
chmod 0755 "$STAGING/scripts/buzz-ctx" "$STAGING/scripts/buzz-ctx-graph" "$STAGING/scripts/buzz-drain"

mkdir -p "$STAGING/ops/macos/launchd"
cp "$ROOT"/ops/macos/launchd/*.template "$STAGING/ops/macos/launchd/"

BUZZ_SHA256="$(sha256_of "$STAGING/bin/buzz")"
jq -n \
  --arg version "$VERSION" \
  --arg git_revision "$GIT_SHA" \
  --arg sha256 "$BUZZ_SHA256" \
  '{version: $version, git_revision: $git_revision, sha256: $sha256}' \
  >"$STAGING/release.json"

mkdir -p "$(dirname "$OUTPUT")"
mv "$STAGING" "$OUTPUT"
trap - EXIT

echo "built node runtime: $OUTPUT"
echo "release revision: $GIT_SHA"
echo "install with: just node-install $OUTPUT"
