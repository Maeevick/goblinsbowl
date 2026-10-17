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

1. **Safe-first index.** In each turn, among the activations actually played, the share of pairs (certain, risky) where the certain one came first. 1.0 means every certain activation preceded every risky one. Since 17 Oct 2026, two orders of the same goblin are not paired (see the changes below).
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

- **10 Oct 2026, before the first batch.** Coaches now start in their own folder outside this repository, with skills disabled, so no auto-memory, CLAUDE.md or skill text of the machine reaches them (the spike's coaches ran from the repository and could have seen its memory notes). The first batch plays seeds 1 to 5 from each seat (`scripts/assessment-batch.sh`), which is how "seats alternating" is applied. A coach that stops before the final whistle is resumed once with its context, never after hitting its budget.
- **10 Oct 2026, between the first and second batches.** The first batch (Sonnet 5, Sonnet 5.5, Haiku 5.5, Haiku 4.5 and Sonnet 4.6) is void: the subscription's session limit stopped coaches in the middle of matches, so its records mix play with outages; they are kept apart in `runs/assessment-1-invalid-session-limit`. Since then, a match whose coach hits the account's session or usage limit before the final whistle is abandoned without a result and replayed from scratch on the same seed once the limit resets, while the batch waits (this happened once in the second batch, on seed 4 away). The second batch plays Haiku 5.5 alone, one match at a time, with a budget of $25 per coach run: a whole match counts $4 to $7 of usage, so the $6 default would stop coaches before the final whistle.
- **12 Oct 2026, after the second batch.** Rules 0.2 now ship in `rules.toml`: a 21 × 21 bowl with 11-square dens, 12 turns per half, a scrimmage kick-off formation, goblins that keep moving after their action (their Move in steps and one action per turn, spent over several orders), and turns that end only with `end_turn`, a failed roll, a score or the clock. This 0.2 is not the candidate of the Conditions table: `kickoff_depth` and `stance_bonus_cap` keep their 0.1 values; how it plays between bots is in [`balance.md`](balance.md). The MCP instructions, the tool descriptions and `get_rules` now describe 0.2, and the view shows each goblin's `steps_left` and `has_action_left`; `prompts/coach.md` is unchanged. Records embed their ruleset and every new setting defaults to rules 0.1, so batches 1 and 2 still replay and assess as before (all 61 local records checked). Three measures read differently under 0.2 and need a decision before the next batch, none applied yet: the safe-first index pairs orders, so a goblin's certain walk after its own risky action counts as out of order even when it could not have come first; exposure at failure counts goblins with steps left as still ready; and turns lost to the clock now include turns a coach forgot to end.
- **13 Oct 2026, before the next batch.** The agents' interface changed: a dry run now previews its orders in sequence, each from where the previous ones leave the board if their rolls succeed (before, each from the board as it stood), and returns the plan's chance; the new `compare_plans` tool previews alternative plans side by side. Neither changes what a played order does nor any measure: both are previews, kept only in the call log. The batch scripts now default to $10 per coach run and to a match timeout read from the rules and the turn length, sized for 24-turn matches.
- **13 Oct 2026, rules 0.3.** Rules 0.3 now ship in `rules.toml`: the kick-off drops the ball 4 to 7 squares into the receiving half and up to 5 rows to either side, then bounces three times; the default team is 3 Bruisers, 4 Sprinters and 2 Throwers; a Stance bonus counts at most +1 for the goblin itself and +1 for its braced neighbours; and a dodge is an agility test, d(Agility) minus 1 per adjacent standing enemy and 2 per braced one, which must reach 2, with no critical result. `get_rules` and the tool descriptions describe 0.3, and an agent now reads a dodge's preview as `dodge out of [x, y]: d4-2 needs 2`. Every new setting still defaults to rules 0.1 when a record leaves it out, so the 61 local records replay and assess as before. No measure changed, but health exposure (5) reads differently: a failed dodge costs its shortfall minus 1 in Health, so leaving a crowd can knock a goblin out, and a Bruiser next to an enemy can no longer dodge at all. How 0.3 plays between bots is in [`balance.md`](balance.md).
- **13 Oct 2026, rules 0.4.** The archetypes grew for the 21 × 21 bowl, 22 points each: Bruiser Attack 4, Defense 4, Agility 3, Health 7, Move 4; Sprinter 3, 2, 6, 5, 6; Thrower 3, 3, 6, 6, 4. Agility tests now mostly roll a d6, so a Sprinter leaves one enemy two times in three and a Bruiser one time in three, and no goblin can be knocked out by a single won assault any more. `get_rules` describes 0.4. Records embed their ruleset, so the 61 local records replay and assess as before. No measure changed; health exposure (5) and the bots' yardsticks for results (7) read differently, see [`balance.md`](balance.md).
- **13 Oct 2026, rules 0.5.** Stance is gone: no goblin can brace any more, so no Defense bonus, no braced enemy weighing 2 on a dodge, and an action is an assault or a throw. Agents can no longer order a Stance (the `play` schema has no `stance` action) and their view no longer carries `in_stance`; the window shows no Garde button and a single defences line on its scoresheet. The engine still knows Stance for records whose ruleset has one, so the 61 local records replay and assess as before. No measure changed, but the safe-first index (1) loses a whole kind of certain order: a Stance never failed.
- **14 Oct 2026, rules 0.6.** Three rules changed: a goblin of the team on turn missing a bouncing ball (after an assault knocked the carrier down) no longer ends the turn, as in Blood Bowl; the goblin holding the ball can no longer assault; and a throw to an adjacent teammate is a hand-off, with no throw roll and only the receiver's catch, reported as a new `handed_off` event. Agents' assault options leave out the carrier, and `get_rules` describes 0.6. Records embed their ruleset, so the 61 local records replay and assess as before. Exposure at failure (2) and failure rates by cause (6) read slightly differently: a fumble now needs a failed pick-up, a failed throw, the team's own throw or hand-off not caught, or an interception.
- **16 Oct 2026, rules 0.7.** The bowl is round: a pixelated disc 21 squares across (349 squares), each den the 11-square arc at its west or east edge, and an agent's view now gives the columns a den spans. Health lasts the whole match: nobody heals at a kick-off and a knocked-out goblin stays out, the others sharing the formation's squares. `get_rules` describes 0.7. Records embed their ruleset, so the 61 local records replay and assess as before. No measure changed, but health exposure (5) now weighs more, since every point lost is lost for the match. The window also holds each headline before showing what came next, and its bot waits for the bowl to play out; neither touches a record or a measure.
- **17 Oct 2026, after the third batch.** The safe-first index (1) no longer pairs two orders of the same goblin. Since rules 0.2 a goblin can walk on after its action, and that walk counted as a certain order played after a risky one, although it could not have come before the action it follows; on 12 Oct this was flagged as an open question, and on 17 Oct the user decided to leave such pairs out. Orders of different goblins are paired as before. Under rules 0.1 every goblin plays one order a turn, so batches 1 and 2 read the same; batch 3 was recorded under the old measure first, then read again (see the results). The two other measures flagged on 12 Oct, exposure at failure and turns lost to the clock, are unchanged.

## Results

Each batch as `make assess ARGS="--batch runs/<batch>"` reported it, read on 17 Oct 2026 with the measures of that day. A batch read again after a measure changes gets its own dated part below; these tables stay as they are. Calibration (8) is not reported: every batch so far ran with full odds hints, where agents see the true chances.

### Batch 2, 10 Oct 2026: Haiku 5.5 against the cautious bot, rules 0.1

Seeds 1 to 5 from both seats, full hints, 60 s turns, one match at a time, $25 per coach run.

| Model | Matches | W-D-L | Goals | Safe-first | Failures per match | Ready at failure | Mean gap | Risky gaps over 5 pts | Chance taken | After a failure | Turns lost to the clock | Refused calls | Resumed | Unfinished | Cost | Minutes per match |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| claude-haiku-5-5 | 10 | 0-10-0 | 0-0 | 0.96 | 5.8 | 2.7 | 0.1 pts | 1 | 71.7 % | 68.6 % | 0 | 0 | 0 | 0 | $54.48 | 5.3 |

| Bot | Safe-first | Failures per match | Ready at failure | Mean gap | Risky gaps over 5 pts | Chance taken | After a failure |
|---|---|---|---|---|---|---|---|
| cautious | 0.32 | 1.4 | 5.4 | 0.0 pts | 0 | 81.2 % | 80.2 % |

All six criteria pass, in matches where neither side scored.

### Batch 3, 16 and 17 Oct 2026: Haiku 5.5 against the cautious and the bold bots, rules 0.7

Seeds 1 to 5 from both seats against each bot, full hints, 60 s turns, five matches at a time, $10 per coach run. No session limit was hit; three coaches stopped early and were resumed once, and every match reached its final whistle.

| Model | Matches | W-D-L | Goals | Safe-first | Failures per match | Ready at failure | Mean gap | Risky gaps over 5 pts | Chance taken | After a failure | Turns lost to the clock | Refused calls | Resumed | Unfinished | Cost | Minutes per match |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| claude-haiku-5-5 against cautious | 10 | 3-4-3 | 11-10 | 0.63 | 10.0 | 6.0 | 0.5 pts | 10 | 74.4 % | 72.0 % | 2 | 10 | 2 | 0 | $115.60 | 9.5 |
| claude-haiku-5-5 against bold | 10 | 2-6-2 | 9-11 | 0.64 | 10.1 | 6.1 | 0.5 pts | 10 | 74.5 % | 74.3 % | 4 | 7 | 1 | 0 | $98.46 | 9.6 |

| Bot | Safe-first | Failures per match | Ready at failure | Mean gap | Risky gaps over 5 pts | Chance taken | After a failure |
|---|---|---|---|---|---|---|---|
| cautious | 0.31 | 7.3 | 6.2 | 0.3 pts | 7 | 79.2 % | 78.5 % |
| bold | 0.38 | 13.7 | 5.8 | 0.8 pts | 21 | 72.4 % | 73.0 % |

| Batch | Safe-first at least 0.90 | Mean gap under 5 pts | Under 10 % of risky orders 5 pts below the best | No tilt (within 5 pts after a failure) | Under one refused call per two turns | Goal difference not negative |
|---|---|---|---|---|---|---|
| against cautious | fail | pass | pass | pass | pass | pass |
| against bold | fail | pass | pass | pass | pass | fail |

The safe-first index still pairs every order of a turn, so a goblin's certain walk after its own risky action counts as out of order even though it could not have come first (the measure question of 12 Oct, still open on the day of this reading).

