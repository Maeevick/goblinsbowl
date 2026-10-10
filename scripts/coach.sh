#!/usr/bin/env bash
# Usage: scripts/coach.sh <home|away> <mcp-url> <seat-token> <run-dir> [extra claude flags...]
# Launches one Claude Code coach in headless mode for a seat of a running goblinsbowl-server.
# The coach starts in its own folder outside this repository, with skills disabled and only the
# goblinsbowl MCP server, so no memory, CLAUDE.md or skill text of this machine's projects reaches it.
# GB_RESUME_SESSION=<session id> resumes a coach that stopped before the final whistle, with its context.
set -euo pipefail

seat="$1"; url="$2"; token="$3"; run_dir="$4"; shift 4
root="$(cd "$(dirname "$0")/.." && pwd)"
mkdir -p "$run_dir"
run_dir="$(cd "$run_dir" && pwd)"
work_dir="${TMPDIR:-/tmp}/goblinsbowl-coach/$(printf '%s' "$run_dir/$seat" | shasum | cut -c1-16)"
mkdir -p "$work_dir"
team_name=$([ "$seat" = home ] && echo "home (H, west den)" || echo "away (A, east den)")
if [ -n "${GB_RESUME_SESSION:-}" ]; then
  prompt="The match is not over yet: keep coaching (call wait_for_my_turn) until the status is game_over."
  set -- --resume "$GB_RESUME_SESSION" ${@+"$@"}
  printf 'resumed %s with: claude %s\n' "$GB_RESUME_SESSION" "$*" >> "$run_dir/$seat-setup.txt"
else
  prompt=$(sed "s/TEAM_NAME/$team_name/" "$root/prompts/coach.md")
  printf '%s  prompts/coach.md\nclaude %s\n' "$(shasum -a 256 "$root/prompts/coach.md" | cut -d' ' -f1)" "$*" > "$run_dir/$seat-setup.txt"
fi
mcp_config=$(printf '{"mcpServers":{"goblinsbowl":{"type":"http","url":"%s","headers":{"Authorization":"Bearer %s"}}}}' "$url" "$token")

cd "$work_dir"
claude -p "$prompt" \
  --strict-mcp-config --mcp-config "$mcp_config" \
  --tools "" --allowedTools "mcp__goblinsbowl__*" --disable-slash-commands \
  --output-format stream-json --verbose \
  ${@+"$@"} >> "$run_dir/$seat-transcript.jsonl"
