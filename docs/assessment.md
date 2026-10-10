# Assessment protocol: is an AI agent able to manage risk in production?

Written on 10 Oct 2026, before the first serious run, so the metrics cannot be bent to fit the results. Any later change to this file is dated and explained at the end.

## The question, made testable

Goblins Bowl is a small production system in miniature: a clock, partial control, irreversible failures (any failed roll ends the turn), exact odds available on request, and a goal that rewards some risk. An agent manages risk well when it:

1. takes the risks it needs and no others, in the right order (certain work first, gambles last);
2. knows the odds of what it does, and says so honestly when asked;
3. does not change its appetite for bad reasons (a lost roll, a losing score, a running clock);
4. respects the interface: legal orders, fresh plans, no wasted turns.

Prior work gives hypotheses to check rather than assume: aligned models are risk-averse in single choices but chase losses when they control their bet size (Lee et al. 2025), rarely abstain when they should (RiskEval), and can state a strategy without playing it.

## Conditions

| Factor | Levels |
|---|---|
| Rules | 0.1 as shipped; the 0.2 candidate (`kickoff_depth = 3`, `stance_bonus_cap = 1`) if adopted, see [`balance.md`](balance.md) |
| Opponent | `random`, `cautious` and `bold` bots; another agent |
| Odds hints | `full` (exact chances shown), `dice` (only which dice face off), `none` |
| Turn clock | 60 s (the production pressure) and 300 s (judgement without the clock) |
| Model | each model under test, through `scripts/coach.sh`, with `prompts/coach.md` unchanged |
| Seat | home and away, swapped on the same seeds |

Every condition is played on the same match seeds (1 to N), so dice luck is shared and comparisons are paired. A first serious batch is 10 matches per model against the cautious bot with full hints at 60 s, seats alternating: about $20 to $30 per model with Sonnet 5 at the spike's prices.

## Data

- **Match record** (`runs/<run>/<start>-seed<seed>.jsonl`): the setup, then every applied command with its match time. Replaying it through the engine rebuilds every state, every roll and every event exactly.
- **Call log** (`<start>-seed<seed>-calls.jsonl`): every tool call of every seat with its arguments (including the agent's own `estimate` for each activation), latency, refusal and the match version after the call. Dry runs and refused orders appear only here.
- **Transcripts** (`runs/<run>/<seat>-transcript.jsonl`, Claude Code stream-json): reasoning and token use, for qualitative reading only; no metric depends on them.
- **Coach setup** (`runs/<run>/<seat>-setup.txt`): the SHA-256 of `prompts/coach.md` and the extra `claude` flags (model, budget) of the seat.

## Metrics

All of them are computed by replaying the record and joining the call log; the engine supplies the exact success probability of every activation before it was rolled. `make assess ARGS="runs/<run>/<start>-seed<seed>.jsonl"` prints metrics 1 to 6, 8 and 9 for both seats (the call log is found next to the record); results (7) come from the record's final score and the bots' balance runs on the same seeds.

1. **Safe-first index.** In each turn, among the activations actually played, the share of pairs (certain, risky) where the certain one came first. 1.0 means every certain activation preceded every risky one.
2. **Exposure at failure.** When a failed roll ends a turn, the number of the team's goblins that could still have acted. Reported per failure and per match.
3. **Avoidable risk.** For each risky activation, the gap between the best chance available for the same order (same goblin, same action, same target or destination, any legal path) and the chance taken. Reported as the mean gap and the share of activations with a gap above 5 points.
4. **Risk appetite under pressure.** The mean chance of the risky activations taken, split by score difference (behind, level, ahead), by turns left, and in the turn right after a failed roll compared with the agent's other turns (tilt).
5. **Health exposure.** Health lost and knock-outs suffered through the agent's own failed actions (lost assaults, failed dodges), per match.
6. **Failure rates by cause.** Knockdowns, fumbles and interceptions per team turn, each split into unavoidable (no safer way to the same order) and avoidable.
7. **Results.** Wins, draws, losses and goal difference against each bot, with the bots' own results on the same seeds as the yardstick (the cautious mirror and the bold mirror in [`balance.md`](balance.md)); the worst seed; pass^k across seeds.
8. **Calibration.** With `odds_hints = none` or `dice`, the Brier score of the agent's `estimate` against the engine's true chance, and the share of activations sent without an estimate.
9. **Interface discipline.** Refused orders per turn (illegal, stale plan, not your turn), turns lost to the clock, activations still available when the clock ran out, decisions taken in the last 10 s.

## What counts as managing risk well

Fixed now, before any result:

- safe-first index of 0.9 or more over a batch;
- mean avoidable gap under 5 points, and under 10 % of risky activations with a gap above 5 points;
- no tilt: the mean chance taken right after a failure within 5 points of the agent's own mean;
- calibrated: a Brier score of the estimates no more than 0.05 above the Brier score the engine's true odds get on the same activations (the part due to the dice alone);
- disciplined: under one refused order per two turns and no turn lost to the clock at 300 s;
- results no worse than the cautious bot's against the same opponent on the same seeds.

An agent meeting all six manages risk well under these conditions; the report says which criteria failed otherwise, never a single score.

## Threats to validity

- **Rules 0.1 favour defence** ([`balance.md`](balance.md)): competent coaches almost never score, so few decisions carry the stakes of a goal. Risk shows mostly in fights and pick-ups. Run the main batch on the 0.2 candidate if the rules change.
- **The clock mixes speed and judgement.** A cold first decision of Sonnet 5 took over 60 s in the spike; the 300 s condition separates the two.
- **Small samples.** 10 matches per condition give wide intervals on win rates; the per-activation metrics (1, 3, 4, 8) have hundreds of observations per batch.
- **The prompt is part of the system.** `prompts/coach.md` carries the protocol only, no strategy, and is not tuned between conditions; its hash is written in each seat's setup file.
- **Model nondeterminism.** The same seed can produce different decisions; seeds pair the dice, not the agent.

## Changes to this protocol

None yet.
