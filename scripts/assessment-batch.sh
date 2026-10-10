#!/usr/bin/env bash
# Usage: scripts/assessment-batch.sh <batch-name> <model>...
# Plays every model against the built-in bot on each seed, once from each seat (docs/assessment.md),
# GB_PARALLEL matches at a time, each on its own server and port. Re-running the same batch name
# only plays the matches still missing. When the account's session limit is hit, the batch waits
# for the reset time the limit message gives (or GB_LIMIT_WAIT seconds, 1800 by default), then plays
# the missing matches. Settings: GB_SEEDS ("1 2 3 4 5"), GB_PARALLEL (5), GB_FIRST_PORT (7920),
# plus those of scripts/assessment-match.sh.
set -euo pipefail

batch="$1"; shift
root="$(cd "$(dirname "$0")/.." && pwd)"
batch_dir="$root/runs/$batch"
mkdir -p "$batch_dir"
cargo build -q -p goblins_server --manifest-path "$root/Cargo.toml"

port="${GB_FIRST_PORT:-7920}"
jobs=""
for seed in ${GB_SEEDS:-1 2 3 4 5}; do
  for seat in home away; do
    for model in "$@"; do
      jobs="$jobs$batch_dir $model $seed $seat $port
"
      port=$((port + 1))
    done
  done
done
seconds_until_reset() {
  local clock epoch now
  now=$(date +%s)
  epoch=$(grep -o -E '\|[0-9]{10}' "$batch_dir/LIMIT_HIT" | tr -d '|' | head -1 || true)
  if [ -z "$epoch" ]; then
    clock=$(grep -o -i -E 'resets [0-9]{1,2}(:[0-9]{2})? ?[ap]m' "$batch_dir/LIMIT_HIT" | head -1 | sed -E 's/resets //I; s/ //g' | tr '[:lower:]' '[:upper:]' || true)
    case "$clock" in
      *:*) epoch=$(date -j -f "%I:%M%p" "$clock" +%s 2>/dev/null || true) ;;
      ?*) epoch=$(date -j -f "%I%p" "$clock" +%s 2>/dev/null || true) ;;
    esac
    if [ -n "$epoch" ] && [ "$epoch" -le "$now" ]; then epoch=$((epoch + 86400)); fi
  fi
  if [ -n "$epoch" ]; then echo $((epoch - now + 120)); else echo "${GB_LIMIT_WAIT:-1800}"; fi
}

while true; do
  rm -f "$batch_dir/LIMIT_HIT"
  printf '%s' "$jobs" | xargs -P "${GB_PARALLEL:-5}" -n 5 "$root/scripts/assessment-match.sh"
  [ -f "$batch_dir/LIMIT_HIT" ] || break
  wait_seconds=$(seconds_until_reset)
  echo "$(date '+%H:%M') session limit hit ($(cat "$batch_dir/LIMIT_HIT")): waiting ${wait_seconds}s for the reset"
  sleep "$wait_seconds"
done
echo "Batch $batch done: $(find "$batch_dir" -name match.json | wc -l | tr -d ' ') matches in $batch_dir"
