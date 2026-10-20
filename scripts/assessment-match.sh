#!/usr/bin/env bash
# Usage: scripts/assessment-match.sh <batch-dir> <model> <seed> <seat> <port>
# Plays one assessment match: a Claude Code coach (<model>) on <seat> against the built-in bot, or
# against a second coach (GB_OPPONENT_MODEL) on the other seat, on its own server, until the final
# whistle. A coach that stops early is resumed once with its context, unless it ran out of budget.
# Everything lands in <batch-dir>/<model>/seed<seed>-<seat>/.
# When a coach hits the account's session or usage limit, the match is abandoned without a
# match.json (a later run replays it) and <batch-dir>/LIMIT_HIT tells the other jobs to stand down.
# Settings: GB_BOT (balanced-2), GB_OPPONENT_MODEL (none: the bot plays the other seat),
# GB_OPPONENT_BUDGET (GB_BUDGET), GB_TURN_SECONDS (60), GB_HINTS (full), GB_RULES and GB_RULES_TEXT (the
# frozen ruleset and rules text of the benchmark, assessment/rules-0.10.toml and .md),
# GB_BUDGET (10 USD per coach run, a 24-turn match), GB_RESUMES (1), GB_MATCH_TIMEOUT (seconds
# allowed after the coaches end; by default every turn of the coached teams running out, 15 s of
# readiness grace included, plus 20 minutes, from GB_TURN_SECONDS and GB_RULES).
set -uo pipefail

batch_dir="$1"; model="$2"; seed="$3"; seat="$4"; port="$5"
root="$(cd "$(dirname "$0")/.." && pwd)"
dir="$batch_dir/$model/seed$seed-$seat"
rules="${GB_RULES:-$root/assessment/rules-0.10.toml}"
rules_text="${GB_RULES_TEXT:-$root/assessment/rules-0.10.md}"
opponent_model="${GB_OPPONENT_MODEL:-}"
[ -f "$dir/match.json" ] && exit 0
[ -f "$batch_dir/LIMIT_HIT" ] && exit 0
rm -rf "$dir"
mkdir -p "$dir"
if [ "$seat" = home ]; then other_seat=away; else other_seat=home; fi
if [ -n "$opponent_model" ]; then other_kind=agent; else other_kind=bot; fi
agent_token="$seat-$RANDOM$RANDOM$RANDOM"
other_token="$other_seat-$RANDOM$RANDOM$RANDOM"
if [ "$seat" = home ]; then
  seat_flags=(--home agent --away "$other_kind"); home_token="$agent_token"; away_token="$other_token"
else
  seat_flags=(--home "$other_kind" --away agent); home_token="$other_token"; away_token="$agent_token"
fi
"$root/target/debug/goblinsbowl-server" "${seat_flags[@]}" --bot "${GB_BOT:-balanced-2}" --seed "$seed" --port "$port" \
  --turn-seconds "${GB_TURN_SECONDS:-60}" --odds-hints "${GB_HINTS:-full}" --rules "$rules" --rules-text "$rules_text" \
  --home-token "$home_token" --away-token "$away_token" --record-dir "$dir" > "$dir/server.log" 2>&1 &
server=$!
coaches=""
kill_tree() {
  local child
  for child in $(pgrep -P "$1"); do kill_tree "$child"; done
  kill "$1" 2>/dev/null
}
stop_everything() {
  local coach
  for coach in $coaches; do kill_tree "$coach"; done
  kill "$server" 2>/dev/null; wait "$server" 2>/dev/null
}
trap stop_everything EXIT
for _ in $(seq 1 100); do nc -z 127.0.0.1 "$port" 2>/dev/null && break; sleep 0.2; done

url="http://127.0.0.1:$port/mcp"
started=$(date +%s)
limit_pattern='"result":"[^"]*(hit your [a-z]* ?limit|usage limit)[^"]*"'
limited() {
  grep -q "Final whistle" "$dir/server.log" && return 1
  grep -q -i -E "$limit_pattern" "$dir"/*-transcript.jsonl 2>/dev/null
}
abandon_if_limited() {
  limited || return 0
  grep -h -o -i -E '"result":"[^"]*limit[^"]*"' "$dir"/*-transcript.jsonl | tail -1 > "$batch_dir/LIMIT_HIT"
  [ -s "$batch_dir/LIMIT_HIT" ] || echo "limit hit in $dir" > "$batch_dir/LIMIT_HIT"
  exit 0
}
coach_seat() {
  local coached_seat="$1" coach_model="$2" budget="$3" token="$4" runs=1 session
  "$root/scripts/coach.sh" "$coached_seat" "$url" "$token" "$dir" --model "$coach_model" --max-budget-usd "$budget"
  while ! grep -q "Final whistle" "$dir/server.log" && [ "$runs" -le "${GB_RESUMES:-1}" ] && ! limited; do
    grep -q '"subtype":"error_max_budget_usd"' "$dir/$coached_seat-transcript.jsonl" && break
    session=$(grep -o '"session_id":"[^"]*"' "$dir/$coached_seat-transcript.jsonl" | tail -1 | cut -d'"' -f4)
    [ -n "$session" ] || break
    runs=$((runs + 1))
    GB_RESUME_SESSION="$session" "$root/scripts/coach.sh" "$coached_seat" "$url" "$token" "$dir" \
      --model "$coach_model" --max-budget-usd "$budget"
  done
  echo "$runs" > "$dir/$coached_seat-coach-runs"
}
coaching() {
  local coach
  for coach in $coaches; do kill -0 "$coach" 2>/dev/null && return 0; done
  return 1
}
coach_seat "$seat" "$model" "${GB_BUDGET:-10}" "$agent_token" &
coaches="$!"
if [ -n "$opponent_model" ]; then
  coach_seat "$other_seat" "$opponent_model" "${GB_OPPONENT_BUDGET:-${GB_BUDGET:-10}}" "$other_token" &
  coaches="$coaches $!"
fi
while coaching; do abandon_if_limited; sleep 5; done
abandon_if_limited
coaches=""
coach_seconds=$(( $(date +%s) - started ))
team_turns=$(awk -F= '/^turns_per_half/ { turns = $2 } /^halves/ { halves = $2 } END { print turns * halves }' "$rules")
[ "${team_turns:-0}" -gt 0 ] 2>/dev/null || team_turns=24
if [ -n "$opponent_model" ]; then coached_teams=2; else coached_teams=1; fi
match_timeout=$(( coached_teams * team_turns * (${GB_TURN_SECONDS:-60} + 15) + 1200 ))
deadline=$(( $(date +%s) + ${GB_MATCH_TIMEOUT:-$match_timeout} ))
until grep -q "Final whistle" "$dir/server.log" || [ "$(date +%s)" -ge "$deadline" ]; do sleep 5; done
finished=$(grep -q "Final whistle" "$dir/server.log" && echo true || echo false)
kill "$server" 2>/dev/null; wait "$server" 2>/dev/null
cost_of() {
  grep -o '"total_cost_usd":[0-9.]*' "$dir/$1-transcript.jsonl" 2>/dev/null | cut -d: -f2 \
    | awk '{ total += $1 } END { printf "%.4f", total }'
}
runs_of() { cat "$dir/$1-coach-runs" 2>/dev/null || echo 1; }
if [ -n "$opponent_model" ]; then
  against=$(printf '"opponent_model":"%s","opponent_coach_runs":%s,"opponent_cost_usd":%s' \
    "$opponent_model" "$(runs_of "$other_seat")" "$(cost_of "$other_seat")")
else
  against=$(printf '"bot":"%s"' "${GB_BOT:-balanced-2}")
fi
printf '{"model":"%s",%s,"seed":%s,"seat":"%s","finished":%s,"coach_runs":%s,"coach_seconds":%s,"match_seconds":%s,"cost_usd":%s}\n' \
  "$model" "$against" "$seed" "$seat" "$finished" "$(runs_of "$seat")" "$coach_seconds" \
  "$(( $(date +%s) - started ))" "$(cost_of "$seat")" > "$dir/match.json"
