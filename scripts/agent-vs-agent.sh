#!/usr/bin/env bash
# Usage: scripts/agent-vs-agent.sh [budget-usd-per-coach] [extra goblinsbowl-server flags...]
# Starts a headless match between two Claude Code coaches and stores everything in runs/<timestamp>/.
set -euo pipefail

budget="${1:-4}"; shift || true
root="$(cd "$(dirname "$0")/.." && pwd)"
run_dir="$root/runs/$(date +%Y%m%d-%H%M%S)"
port="${GB_PORT:-7878}"
home_token="home-$RANDOM$RANDOM$RANDOM"
away_token="away-$RANDOM$RANDOM$RANDOM"
mkdir -p "$run_dir"

cargo build -q -p goblins_server --manifest-path "$root/Cargo.toml"
GB_HOME_TOKEN="$home_token" GB_AWAY_TOKEN="$away_token" \
  "$root/target/debug/goblinsbowl-server" --home agent --away agent --port "$port" \
  --record-dir "$run_dir" "$@" > "$run_dir/server.log" 2>&1 &
server_pid=$!
trap 'kill "$server_pid" 2>/dev/null || true' EXIT
sleep 1

url="http://127.0.0.1:$port/mcp"
"$root/scripts/coach.sh" home "$url" "$home_token" "$run_dir" --max-budget-usd "$budget" &
home_pid=$!
"$root/scripts/coach.sh" away "$url" "$away_token" "$run_dir" --max-budget-usd "$budget" &
away_pid=$!
wait "$home_pid" "$away_pid"

grep "Final whistle" "$run_dir/server.log" || echo "The match did not reach the final whistle; see $run_dir"
echo "Run stored in $run_dir"
