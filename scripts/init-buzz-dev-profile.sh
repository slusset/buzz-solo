#!/usr/bin/env bash
# Create the isolated XDG development profile without touching the live node.
set -euo pipefail

umask 077

config_home="${XDG_CONFIG_HOME:-$HOME/.config}"
data_home="${XDG_DATA_HOME:-$HOME/.local/share}"
profile_path="$config_home/buzz/profiles/dev.toml"
data_root="${BUZZ_DEV_DATA_ROOT:-$data_home/buzz-local-relay/dev}"

if [ -e "$profile_path" ] || [ -L "$profile_path" ]; then
  echo "development profile already exists: $profile_path" >&2
  exit 0
fi

mkdir -p "$(dirname "$profile_path")" "$data_root/artifacts" "$data_root/cursors"

quote_toml() {
  printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
}

quoted_data_root="$(quote_toml "$data_root")"
temporary="$(mktemp "${profile_path}.tmp.XXXXXX")"
trap 'rm -f "$temporary"' EXIT

cat >"$temporary" <<EOF
schema_version = 1
data_root = "$quoted_data_root"

[node]
label = "dev"

[relays]
local = "http://127.0.0.1:3100"

[paths]
journal = "sovereign.ndjson"
artifacts = "artifacts"
cursors = "cursors"

[identities.journal_author]
provider = "environment"
reference = "BUZZ_DEV_PRIVATE_KEY"
label = "dev"
EOF

mv "$temporary" "$profile_path"
trap - EXIT

echo "created development profile: $profile_path"
echo "development data root: $data_root"
echo "set BUZZ_DEV_PRIVATE_KEY before using context write commands"
