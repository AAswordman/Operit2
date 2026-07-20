#!/usr/bin/env bash
set -euo pipefail

env_file="${OPERIT_CLOUDFLARE_ENV_FILE:?OPERIT_CLOUDFLARE_ENV_FILE must point to assistance_web/.env.local}"
token="$(sed -n 's/^CLOUDFLARE_API_TOKEN=//p' "$env_file" | head -n 1)"
test -n "$token"
export CLOUDFLARE_API_TOKEN="$token"

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec python3 "$script_dir/publish_v86_runtime_r2.py" "$@"
