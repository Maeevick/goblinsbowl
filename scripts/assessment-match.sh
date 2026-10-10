#!/usr/bin/env bash
# Usage: scripts/assessment-match.sh <batch-dir> <model> <seed> <seat> <port>
# Plays one assessment match: a Claude Code coach (<model>) on <seat> against the built-in bot, on its
# own server, until the final whistle. A coach that stops early is resumed once with its context,
# unless it ran out of budget. Everything lands in <batch-dir>/<model>/seed<seed>-<seat>/.
# Settings: GB_BOT (cautious), GB_TURN_SECONDS (60), GB_HINTS (full), GB_RULES (rules.toml),
# GB_BUDGET (6 USD per coach run), GB_RESUMES (1), GB_MATCH_TIMEOUT (3600 s after the coach ends).
set -uo pipefail

batch_dir="$1"; model="$2"; seed="$3"; seat="$4"; port="$5"
root="$(cd "$(dirname "$0")/.." && pwd)"
dir="$batch_dir/$model/seed$seed-$seat"
[ -f "$dir/match.json" ] && exit 0
rm -rf "$dir"
mkdir -p "$dir"
agent_token="$seat-$RANDOM$RANDOM$RANDOM"
other_token="unused-$RANDOM$RANDOM$RANDOM"
if [ "$seat" = home ]; then
  seat_flags=(--home agent --away bot); home_token="$agent_token"; away_token="$other_token"
else
  seat_flags=(--home bot --away agent); home_token="$other_token"; away_token="$agent_token"
fi
"$root/target/debug/goblinsbowl-server" "${seat_flags[@]}" --bot "${GB_BOT:-cautious}" --seed "$seed" --port "$port" \
  --turn-seconds "${GB_TURN_SECONDS:-60}" --odds-hints "${GB_HINTS:-full}" --rules "${GB_RULES:-$root/rules.toml}" \
  --home-token "$home_token" --away-token "$away_token" --record-dir "$dir" > "$dir/server.log" 2>&1 &
server=$!
trap 'kill "$server" 2>/dev/null' EXIT
for _ in $(seq 1 100); do nc -z 127.0.0.1 "$port" 2>/dev/null && break; sleep 0.2; done

url="http://127.0.0.1:$port/mcp"
started=$(date +%s)
coach_runs=1
"$root/scripts/coach.sh" "$seat" "$url" "$agent_token" "$dir" --model "$model" --max-budget-usd "${GB_BUDGET:-6}"
while ! grep -q "Final whistle" "$dir/server.log" && [ "$coach_runs" -le "${GB_RESUMES:-1}" ]; do
  grep -q '"subtype":"error_max_budget_usd"' "$dir/$seat-transcript.jsonl" && break
  session=$(grep -o '"session_id":"[^"]*"' "$dir/$seat-transcript.jsonl" | tail -1 | cut -d'"' -f4)
  [ -n "$session" ] || break
  coach_runs=$((coach_runs + 1))
  GB_RESUME_SESSION="$session" "$root/scripts/coach.sh" "$seat" "$url" "$agent_token" "$dir" \
    --model "$model" --max-budget-usd "${GB_BUDGET:-6}"
done
coach_seconds=$(( $(date +%s) - started ))
deadline=$(( $(date +%s) + ${GB_MATCH_TIMEOUT:-3600} ))
until grep -q "Final whistle" "$dir/server.log" || [ "$(date +%s)" -ge "$deadline" ]; do sleep 5; done
finished=$(grep -q "Final whistle" "$dir/server.log" && echo true || echo false)
kill "$server" 2>/dev/null; wait "$server" 2>/dev/null
cost=$(grep -o '"total_cost_usd":[0-9.]*' "$dir/$seat-transcript.jsonl" | cut -d: -f2 | awk '{ total += $1 } END { printf "%.4f", total }')
printf '{"model":"%s","seed":%s,"seat":"%s","finished":%s,"coach_runs":%s,"coach_seconds":%s,"match_seconds":%s,"cost_usd":%s}\n' \
  "$model" "$seed" "$seat" "$finished" "$coach_runs" "$coach_seconds" "$(( $(date +%s) - started ))" "$cost" > "$dir/match.json"
