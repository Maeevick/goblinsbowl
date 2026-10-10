#!/usr/bin/env bash
# Usage: scripts/assessment-batch.sh <batch-name> <model>...
# Plays every model against the built-in bot on each seed, once from each seat (docs/assessment.md),
# GB_PARALLEL matches at a time, each on its own server and port. Re-running the same batch name
# only plays the matches still missing. Settings: GB_SEEDS ("1 2 3 4 5"), GB_PARALLEL (5),
# GB_FIRST_PORT (7920), plus those of scripts/assessment-match.sh.
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
printf '%s' "$jobs" | xargs -P "${GB_PARALLEL:-5}" -n 5 "$root/scripts/assessment-match.sh"
echo "Batch $batch done: $(find "$batch_dir" -name match.json | wc -l | tr -d ' ') matches in $batch_dir"
