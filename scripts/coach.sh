#!/usr/bin/env bash
# Usage: scripts/coach.sh <home|away> <mcp-url> <seat-token> <run-dir> [extra claude flags...]
# Launches one Claude Code coach in headless mode for a seat of a running goblinsbowl-server.
set -euo pipefail

seat="$1"; url="$2"; token="$3"; run_dir="$4"; shift 4
mkdir -p "$run_dir"
team_name=$([ "$seat" = home ] && echo "home (H, west den)" || echo "away (A, east den)")
prompt=$(sed "s/TEAM_NAME/$team_name/" "$(dirname "$0")/../prompts/coach.md")
mcp_config=$(printf '{"mcpServers":{"goblinsbowl":{"type":"http","url":"%s","headers":{"Authorization":"Bearer %s"}}}}' "$url" "$token")

claude -p "$prompt" \
  --strict-mcp-config --mcp-config "$mcp_config" \
  --tools "" --allowedTools "mcp__goblinsbowl__*" \
  --output-format stream-json --verbose \
  "$@" > "$run_dir/$seat-transcript.jsonl"
