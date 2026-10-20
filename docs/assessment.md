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
| Rules | 0.1 as shipped; the 0.2 candidate (`kickoff_depth = 3`, `stance_bonus_cap = 1`) if adopted, see [`balance.md`](balance.md); since 19 Oct 2026 the frozen benchmark, `assessment/rules-0.10.toml`, whatever `rules.toml` ships |
| Opponent | `random`, `cautious` and `bold` bots, and since 17 Oct their second generation (`cautious-2`, `balanced-2`, `bold-2`), the only bots since 19 Oct; another agent |
| Yardstick | since 19 Oct 2026, `cautious-2` replaying the agent's seats against the same opponent on the same seeds (batches 2 to 5: the first-generation `cautious`) |
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
- **Benchmark** (`assessment/`, since 19 Oct 2026): the frozen ruleset and the rules text agents read with `get_rules`, used by every batch from then on; how the second-generation bots play it is pinned by recorded matches, which a test replays (`make benchmark-matches`).

## Metrics

All of them are computed by replaying the record and joining the call log; the engine supplies the exact success probability of every activation before it was rolled. `make assess ARGS="runs/<run>/<start>-seed<seed>.jsonl"` prints metrics 1 to 6, 8 and 9 for both seats (the call log is found next to the record); results (7) come from the record's final score and the bots' balance runs on the same seeds.

1. **Safe-first index.** In each turn, among the activations actually played, the share of pairs (certain, risky) where the certain one came first. 1.0 means every certain activation preceded every risky one. Since 17 Oct 2026, two orders of the same goblin are not paired, and a certain order played after another goblin's risky one counts against only when it was already certain before it (see the changes below).
2. **Exposure at failure.** When a failed roll ends a turn, the number of the team's goblins that could still have acted. Reported per failure and per match.
3. **Avoidable risk.** For each risky activation, the gap between the best chance available for the same order (same goblin, same action, same target or destination, any legal path) and the chance taken. Reported as the mean gap and the share of activations with a gap above 5 points.
4. **Risk appetite under pressure.** The mean chance of the risky activations taken, split by score difference (behind, level, ahead), by turns left, and in the turn right after a failed roll compared with the agent's other turns (tilt).
5. **Health exposure.** Health lost and knock-outs suffered through the agent's own failed actions (lost assaults, failed dodges), per match.
6. **Failure rates by cause.** Knockdowns, fumbles and interceptions per team turn, each split into unavoidable (no safer way to the same order) and avoidable.
7. **Results.** Wins, draws, losses and goal difference against each bot, with the bots' own results on the same seeds as the yardstick (the cautious mirror and the bold mirror in [`balance.md`](balance.md)); the worst seed; pass^k across seeds. Since 19 Oct 2026 the batch report replays the yardstick itself, from the agent's seats against the same bot on the same seeds.
8. **Calibration.** With `odds_hints = none` or `dice`, the Brier score of the agent's `estimate` against the engine's true chance, and the share of activations sent without an estimate.
9. **Interface discipline.** Refused orders per turn (illegal, stale plan, not your turn), turns lost to the clock, activations still available when the clock ran out, decisions taken in the last 10 s.

## What counts as managing risk well

Fixed now, before any result:

- safe-first index of 0.9 or more over a batch;
- mean avoidable gap under 5 points, and under 10 % of risky activations with a gap above 5 points;
- no tilt: the mean chance taken right after a failure within 5 points of the agent's own mean;
- calibrated: a Brier score of the estimates no more than 0.05 above the Brier score the engine's true odds get on the same activations (the part due to the dice alone);
- disciplined: under one refused order per two turns and no turn lost to the clock at 300 s;
- results no worse than the cautious bot's against the same opponent on the same seeds (since 19 Oct 2026, the yardstick `cautious-2`, read as a goal difference at least the yardstick's).

An agent meeting all six manages risk well under these conditions; the report says which criteria failed otherwise, never a single score.

## Threats to validity

- **Rules 0.1 favour defence** ([`balance.md`](balance.md)): competent coaches almost never score, so few decisions carry the stakes of a goal. Risk shows mostly in fights and pick-ups. Run the main batch on the 0.2 candidate if the rules change.
- **The clock mixes speed and judgement.** A cold first decision of Sonnet 5 took over 60 s in the spike; the 300 s condition separates the two.
- **Small samples.** 10 matches per condition give wide intervals on win rates; the per-activation metrics (1, 3, 4, 8) have hundreds of observations per batch.
- **The prompt is part of the system.** `prompts/coach.md` carries the protocol only, no strategy, and is not tuned between conditions; its hash is written in each seat's setup file.
- **Model nondeterminism.** The same seed can produce different decisions; seeds pair the dice, not the agent.
- **The interface is not frozen.** The benchmark freezes the ruleset, the rules text and the bots' play, but the MCP instructions, tool descriptions and schemas follow the code; any change to them is a protocol change, dated below.

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
- **17 Oct 2026, later the same day.** The safe-first index (1) now counts a certain order played after another goblin's risky one only when the same order (same goblin, same action and target, or same destination, by any legal path) was already certain on the board just before the risky one, which the engine checks by replaying that board. A certain order that only became possible or certain through the risky one, such as an assault on a goblin it had just knocked down, could not have come first, so the pair is left out. Batches 2 and 3 were read a third time (see the results).
- **17 Oct 2026, second-generation bots.** Three bots join the opponents: `cautious-2` and `bold-2` keep the appetites of `cautious` and `bold`, break ties between equally good squares the same way for both teams, and play another goblin's certain move before a gamble when that move leaves the gamble its chance; `balanced-2` slides from the cautious thresholds to the bold ones as fewer of its goblins are left to play. `cautious` and `bold` stay exactly as they were, so every batch so far keeps its yardstick, and the "cautious bot" of the results criterion stays the first-generation one for those batches. A batch already names its bot in its folder; since this day each match's `match.json` records it too. Over 1,000 matches between bots the new ones reach a safe-first index of 0.93 to 0.95, the first generation 0.32 to 0.46, so a batch against them would pit an agent against an opponent that orders its turns ([`balance.md`](balance.md)). The prompt, the MCP instructions and every measure are unchanged.
- **18 Oct 2026, rules 0.8.** Nine rules changed at once, each behind its own setting: the round bowl's dens shrink to the 7 squares of their outermost column; a tied assault is a stand-off where nobody falls and the turn goes on; the goblin holding the ball defends at −1; at every kick-off a knocked-out goblin rolls its Health die minus 1 to come back with that much Health; an assault or a throw costs a step, and each goblin may assault, throw and brace once each per turn; Stance is back at +1 Defense, at the cost of every step left; a goblin weighs on the eight squares around it for dodges, roll maluses and interceptions, while moves and assaults stay orthogonal; and the ball bounces in eight directions on a d8. Agents can order a stance again and read `in_stance`, `actions_left` and an assault's `knock_down_percent`; a dry run lists an assault's knock-down and stand-off chances. No measure changed: an assault's chance stays the chance the attacker keeps its feet, so a stand-off counts as the turn going on, and its knock-down chance is told apart. Records embed their ruleset, so the 83 local records replay and assess as before. The first-generation bots play every older ruleset exactly as before and were only made legal under 0.8; the second generation judges an assault by its chance to knock down. How 0.8 plays between bots: [`balance.md`](balance.md).
- **18 Oct 2026, later the same day, rules 0.9.** Assaults come in styles, chirurgical (+1 to win, no Health lost), brutal (−1 to win, one more Health lost) and projection (−1 to win, the target falling on a square picked around the attacker, numbered 1 to 8 clockwise from north), the style's modifier deciding who wins and the Health lost counting the dice as rolled. A goblin that has not moved may throw into chaos, with no throw roll, the ball bouncing three times from the receiver's square, or focus for +1 to its throws on its team's next turn, either one costing its whole Move. Agents order them with a `style` (and `to` for a projection) and a `focus` action, and read `focused`. No measure changed; a chaos throw and a focus cannot end the turn except by an interception, so they read as certain orders when no enemy can intercept. The second-generation bots sack and free chirurgically, finish a downed or nearly knocked-out goblin brutally, and throw into chaos when no pass is worth it; the first generation keeps simple assaults and throws.
- **18 Oct 2026, rules 0.9 corrected.** Three points of 0.9 had been read wrongly and were put right after the designer's review. A chaos throw ends the turn on a fumble unless a teammate holds the ball once its three bounces are over: a teammate catching it on the way keeps the turn going, while a loose ball or an enemy's catch ends it. Focus belongs to the goblin holding the ball, which may no longer brace in stance (a new setting, `carriers_can_brace`, false in the shipped rules and true for older rulesets). The previous entry's note that chaos throws read as certain orders no longer holds: a chaos throw's chance is now the chance that nobody intercepts and a teammate ends up holding the ball, worked out over every square the d8 may send it to and every catch made or missed on the way, so a lone receiver keeps it about one time in seven and a ring of eight teammates with Agility 6 more than nine times in ten. A dry run gives that chance on its own line, and `actions_left` offers focus to the carrier only and no stance to it. The chance of a long plan no longer reads as certain when its fraction outgrows 64 bits; no local record was long enough for that, and the 83 local records assess exactly as before. The second-generation bots weigh a chaos throw like a pass, against their appetite for passes, towards the teammate around whom the ball is most likely kept. How the corrected 0.9 plays between bots: [`balance.md`](balance.md).
- **18 Oct 2026, evening, rules 0.10.** After a match against `balanced-2` on the medium circle, where the bot braced 70 times and 47 of the 56 assaults were chirurgical, Stance is gone again (`goblins_can_take_stance = false`), and focus becomes part of a throw: a Thrower may throw focused, every step it has left after the throw sacrificed for +1 to its throw roll. A new setting, `focused_throws_spend_steps_left`, is true in the shipped rules; older rulesets keep the whole-turn focus of 0.9, which 0.10 refuses with its own violation. Agents order it as a throw `style`, `focused`; a dry run's throw chance counts the bonus, and `actions_left` no longer lists a stance or a focus. No measure changed: a focused throw is a throw, read with its chance like any other. The 84 local records, that match included, replay and assess. The second-generation bots weigh a Thrower's focused pass against the plain one and keep the better chance; the first generation never focuses.
- **18 Oct 2026, night, before the fourth batch.** The fourth batch plays Haiku 5.5 against the second-generation bots (`balanced-2`, `cautious-2`, `bold-2`) under rules 0.10, with batch 3's settings: seeds 1 to 5 from both seats, full hints, 60 s turns, five matches at a time, $10 per coach run, `prompts/coach.md` unchanged. Its first launch was stopped after eight minutes and thrown away: the `play` schema still offered agents the stance and the stand-alone focus that rules 0.10 refuse, and one coach ordered a stance in its first turn. Since then the schema offers neither, the server still reading them for older rulesets, so agents see what batch 3's agents saw, whose rules had no stance either.
- **19 Oct 2026, the benchmark frozen.** Batches 3 and 4 differ in both their rules (0.7, then 0.10) and their opponents (first, then second generation), so a difference between them cannot be put down to the model. From now on the assessment runs on a benchmark kept apart from the game: the ruleset of rules 0.10 frozen as `assessment/rules-0.10.toml` and the rules text agents read frozen as `assessment/rules-0.10.md` (the server's new `--rules-text`), both the defaults of `scripts/assessment-match.sh`; the opponents are the three second-generation bots, whose play on the frozen ruleset a test compares with recorded matches, command for command; and the yardstick is `cautious-2`, replayed by the batch report from the agent's seats against the same bot on the same seeds, results now judged against its goal difference. Batch 5 bridges batches 3 and 4: Haiku 5.5 against the first-generation `cautious` and `bold` bots on the frozen ruleset, with batch 4's settings and server build. After it the first generation was retired from the game and the assessment (`balanced-2` is the default bot), and two tool descriptions were corrected: `get_rules` no longer names rules 0.9 and `play` no longer mentions a stance. A later change to the benchmark gets a new frozen file and a bridging batch that replays one earlier condition under it.
- **19 Oct 2026, late evening, the budget raised.** At $10 per coach run the budget stopped 11 of the 50 coach runs of batches 4 and 5 and cost 12 turns to the clock, an artefact of the setup rather than a skill, so it is raised to a level meant never to bind: $20 for Haiku 5.5, whose full match needs about $10 to $12, and $30 for Sonnet 5.5, whose cost per tool call in the few calls of batch 1 came close to Haiku's. Batch 6 plays Haiku 5.5 against the three second-generation bots on the frozen benchmark at $20, the reference from now on; batch 7 plays Sonnet 5.5 the same way at $30. Besides the budget, batch 6 differs from batch 4 only by the two corrected tool descriptions, so the two read together show what the budget did.
- **20 Oct 2026, rules 0.11 in the game, the benchmark unchanged.** The game now ships rules 0.11: an assault or a throw no longer costs a step, so a goblin may still act after its last step (`actions_cost_a_step = false`), and every agility dodge gets +1 (`dodge_bonus = 1`), so leaving a single enemy is as easy as a ball roll with nobody around and a Bruiser gets away from one enemy two times in three. The benchmark stays on the frozen rules 0.10, where actions cost a step and dodges get no bonus (a ruleset that leaves `dodge_bonus` out reads 0), so every batch from the fifth on keeps its conditions. The MCP instructions now say whether actions cost a step from the ruleset served, and read word for word as before under the frozen one; the rules text compiled into the server describes 0.11, but the batch scripts always serve the frozen one. Records embed their ruleset, so the 195 local records replay and assess. Moving the benchmark to 0.11 would take a new frozen file and a bridging batch.
- **20 Oct 2026, later the same day, the odds of focused throws corrected.** The chance of an agility test whose bonus exceeded its target of 2 dropped the excess, so under rules 0.10 a focused throw by a Thrower that had not moved (+3, one step paid for the throw) was previewed as if at +2 whenever its roll had a malus: 50 % instead of 67 % for a 10-step pass with nobody around. The dice were always rolled right; the previews agents read (dry runs and `compare_plans`) and the chances the bots weigh were wrong. Since the fix the recorded benchmark matches still replay command for command, and over 1,000 matches of each second-generation mirror on the frozen ruleset the draws move by 0.4 to 1.1 points (cautious-2 81.4 % to 82.5 %, balanced-2 78.1 % to 77.7 %, bold-2 50.7 % to 51.4 %). Agents asked for a focused throw in 70 calls of batches 4 to 7, dry runs included; their results stand, to be read with this in mind.
- **20 Oct 2026, evening, rules 0.12 in the game, the benchmark unchanged.** The game now ships rules 0.12, each change behind its own setting: assaults reach the eight squares around; brutal assaults and projections are a Bruiser's, a projection losing its −1 to win and taking one Health less; an assault on a downed goblin is an aggression, +1 to attack counted in the Health it takes, the only way to hit a goblin down; chaos throws are a Thrower's, and a focused throw needs a Thrower that has not moved, which spends its whole Move for +3 and falls, unhurt, when its throw roll misses; a hand-off's receiver catches at +1; only Sprinters intercept from a square next to the flight line, the others having to stand on it; and Sprinters dive for a ball landing on an empty square around them, the highest roll diving at −1, a catch landing the diver there with the ball and a miss leaving it on the ground. A chaos throw's previewed chance counts those dives, which bring a lone Sprinter's chaos throw to 67 %. The benchmark stays on the frozen rules 0.10, where none of these settings is on, so its conditions do not move: the MCP instructions now say who may use which style from the ruleset served and read word for word as before under the frozen one, which a test pins, and the agents' view lists diagonal assault options only under rules that have them. The 195 local records replay and assess.
- **20 Oct 2026, night, batches 9 and 10 under rules 0.12.** Batch 9 is a smoke test of seven conditions under rules 0.12, one match each on seed 1 with the agent at home: Haiku 5.5 and Sonnet 5.5 each against `balanced-2`, `bold-2` and `cautious-2`, and Haiku 5.5 against Sonnet 5.5. Batch 10 plays the same seven conditions on seeds 1 to 5 from both seats, 70 matches. Both use the ruleset and the rules text frozen for them, `assessment/rules-0.12.toml` and `.md`; the 0.10 benchmark stays the scripts' default and the reference of every earlier batch, so batches 9 and 10 compare with each other, not with batches 4 to 8. Settings as in batches 6 and 7: full hints, 60 s turns, five matches at a time, $20 per Haiku coach run and $30 per Sonnet one. Agents read rules 0.12 with `get_rules` and instructions that say who may use which style, and that `get_rules` prevails over the play schema, whose descriptions of the projection and the focused throw were written for rules 0.10.

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

### Batches 2 and 3 read again, 17 Oct 2026: safe-first without a goblin's own pairs

The same records, with the safe-first index of 17 Oct (two orders of the same goblin are not paired). Only the safe-first columns move.

| Model | Matches | W-D-L | Goals | Safe-first | Failures per match | Ready at failure | Mean gap | Risky gaps over 5 pts | Chance taken | After a failure | Turns lost to the clock | Refused calls | Resumed | Unfinished | Cost | Minutes per match |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| claude-haiku-5-5 against cautious | 10 | 3-4-3 | 11-10 | 0.69 | 10.0 | 6.0 | 0.5 pts | 10 | 74.4 % | 72.0 % | 2 | 10 | 2 | 0 | $115.60 | 9.5 |
| claude-haiku-5-5 against bold | 10 | 2-6-2 | 9-11 | 0.69 | 10.1 | 6.1 | 0.5 pts | 10 | 74.5 % | 74.3 % | 4 | 7 | 1 | 0 | $98.46 | 9.6 |

| Bot | Safe-first | Failures per match | Ready at failure | Mean gap | Risky gaps over 5 pts | Chance taken | After a failure |
|---|---|---|---|---|---|---|---|
| cautious | 0.31 | 7.3 | 6.2 | 0.3 pts | 7 | 79.2 % | 78.5 % |
| bold | 0.37 | 13.7 | 5.8 | 0.8 pts | 21 | 72.4 % | 73.0 % |

| Batch | Safe-first at least 0.90 | Mean gap under 5 pts | Under 10 % of risky orders 5 pts below the best | No tilt (within 5 pts after a failure) | Under one refused call per two turns | Goal difference not negative |
|---|---|---|---|---|---|---|
| against cautious | fail | pass | pass | pass | pass | pass |
| against bold | fail | pass | pass | pass | pass | fail |

Batch 2 reads the same (0.96): under rules 0.1 no goblin plays two orders in a turn. In batch 3 the goblins walking on after their own action explain only part of the fall from 0.96: Haiku's index rises from 0.63 and 0.64 to 0.69 against both bots, still under 0.90, while the bots stay between 0.31 and 0.37.

### Batches 2 and 3 read a third time, 17 Oct 2026: only orders that could have come first

The same records, with the safe-first index of the second 17 Oct change. Only the safe-first columns move, so the criteria tables of the previous part still hold.

| Model | Matches | W-D-L | Goals | Safe-first | Failures per match | Ready at failure | Mean gap | Risky gaps over 5 pts | Chance taken | After a failure | Turns lost to the clock | Refused calls | Resumed | Unfinished | Cost | Minutes per match |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| claude-haiku-5-5, batch 2 against cautious | 10 | 0-10-0 | 0-0 | 0.98 | 5.8 | 2.7 | 0.1 pts | 1 | 71.7 % | 68.6 % | 0 | 0 | 0 | 0 | $54.48 | 5.3 |
| claude-haiku-5-5, batch 3 against cautious | 10 | 3-4-3 | 11-10 | 0.78 | 10.0 | 6.0 | 0.5 pts | 10 | 74.4 % | 72.0 % | 2 | 10 | 2 | 0 | $115.60 | 9.5 |
| claude-haiku-5-5, batch 3 against bold | 10 | 2-6-2 | 9-11 | 0.76 | 10.1 | 6.1 | 0.5 pts | 10 | 74.5 % | 74.3 % | 4 | 7 | 1 | 0 | $98.46 | 9.6 |

| Bot | Safe-first | Failures per match | Ready at failure | Mean gap | Risky gaps over 5 pts | Chance taken | After a failure |
|---|---|---|---|---|---|---|---|
| cautious, batch 2 | 0.33 | 1.4 | 5.4 | 0.0 pts | 0 | 81.2 % | 80.2 % |
| cautious, batch 3 | 0.37 | 7.3 | 6.2 | 0.3 pts | 7 | 79.2 % | 78.5 % |
| bold, batch 3 | 0.43 | 13.7 | 5.8 | 0.8 pts | 21 | 72.4 % | 73.0 % |

Over the three readings of batch 3, Haiku's index went from 0.63 and 0.64 to 0.69, then to 0.78 and 0.76: about a third of the gap to 1.0 came from orders that could not have come first. The rest is real: Haiku played a risky order before another goblin's order that was already certain, often enough to stay under the 0.90 criterion.

### Batch 4, 18 Oct 2026: Haiku 5.5 against the second-generation bots, rules 0.10

Seeds 1 to 5 from both seats against each of `balanced-2`, `cautious-2` and `bold-2`, full hints, 60 s turns, five matches at a time, $10 per coach run, on the tiny circle of `rules.toml`; read the same night with the measures of 17 Oct, unchanged since. The 30 matches played from 22:23 to 23:34 without hitting a session limit; eight coaches stopped early and were resumed once, six were stopped by their budget, and every match reached its final whistle.

| Model | Matches | W-D-L | Goals | Safe-first | Failures per match | Ready at failure | Mean gap | Risky gaps over 5 pts | Chance taken | After a failure | Turns lost to the clock | Refused calls | Resumed | Unfinished | Cost | Minutes per match |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| claude-haiku-5-5 against balanced-2 | 10 | 1-6-3 | 4-6 | 0.83 | 7.8 | 6.7 | 0.9 pts | 8 | 74.7 % | 76.5 % | 8 | 5 | 2 | 0 | $110.83 | 10.2 |
| claude-haiku-5-5 against cautious-2 | 10 | 1-7-2 | 1-3 | 0.79 | 7.0 | 6.3 | 1.5 pts | 11 | 72.2 % | 70.6 % | 9 | 11 | 3 | 0 | $126.55 | 9.8 |
| claude-haiku-5-5 against bold-2 | 10 | 2-3-5 | 2-6 | 0.73 | 6.4 | 6.4 | 0.9 pts | 11 | 71.8 % | 69.2 % | 12 | 10 | 3 | 0 | $115.83 | 10.7 |

| Bot | Safe-first | Failures per match | Ready at failure | Mean gap | Risky gaps over 5 pts | Chance taken | After a failure |
|---|---|---|---|---|---|---|---|
| balanced-2 | 0.94 | 4.2 | 4.4 | 0.3 pts | 3 | 78.1 % | 78.3 % |
| cautious-2 | 0.86 | 2.5 | 5.0 | 0.3 pts | 2 | 75.2 % | 71.8 % |
| bold-2 | 0.95 | 8.6 | 5.5 | 0.3 pts | 6 | 73.2 % | 72.4 % |

| Batch | Safe-first at least 0.90 | Mean gap under 5 pts | Under 10 % of risky orders 5 pts below the best | No tilt (within 5 pts after a failure) | Under one refused call per two turns | Goal difference not negative |
|---|---|---|---|---|---|---|
| against balanced-2 | fail | pass | pass | pass | pass | fail |
| against cautious-2 | fail | pass | pass | pass | pass | fail |
| against bold-2 | fail | pass | pass | pass | pass | fail |

The yardstick of the results criterion, the first-generation cautious bot on the same seeds and seats (`make balance`, 5 games from each seat):

| Cautious bot against | W-D-L | Goals |
|---|---|---|
| balanced-2 | 0-10-0 | 1-1 |
| cautious-2 | 2-6-2 | 2-2 |
| bold-2 | 1-6-3 | 2-4 |

What the batch says:

- **Haiku loses to all three bots**: 4-16-10 over the 30 matches, 7 goals for and 15 against. The cautious bot does better against each of them on the same seeds, a goal difference of 0, 0 and −2 where Haiku gets −2, −2 and −4. Ten matches per opponent leave wide margins, but the direction is the same three times.
- **The safe-first index stays under 0.90**: 0.83, 0.79 and 0.73, close to batch 3's 0.78 and 0.76. The second-generation bots reach 0.86 to 0.95 under the same measure in the same matches, so the bar is within reach in this game.
- **Odds are still read well.** The mean gap to the best version of each order stays between 0.9 and 1.5 points, risky orders are taken at 72 to 75 %, and the chance taken right after a failure stays within 3 points of the rest: no tilt.
- **Haiku plays a fighting game.** It assaulted 814 times chirurgically against 298 simple assaults, 7 brutal ones and 4 projections, and threw 19 times in 30 matches, 3 of them focused, never into chaos. The bots threw 40 times in the same matches.
- **The clock took more turns**: 29 over the 30 matches, against 6 over batch 3's 20. Ten came while the coach was gone, after a budget stop or before its resume; nineteen while it was still at work when its 60 seconds ran out, seven of them right after a preview. At about $10 of usage a match ($353 for the batch), the budget now binds.
- **The interface holds.** 5 to 11 refused calls per batch (stale plans, squares out of the bowl or out of reach), well under the criterion, and no stance or stand-alone focus ordered after the fix. About half of the real plays came right after a preview (44 to 51 %), and 6 to 15 % of activations carried an estimate.
- **Batch 3 is no clean baseline**: the rules (0.7 then, 0.10 now) and the opponents (first then second generation) both changed, so no difference between the two batches can be put down to Haiku alone.

### Batch 5, 19 Oct 2026: the bridge, Haiku 5.5 against the first-generation bots on the frozen benchmark

Seeds 1 to 5 from both seats against `cautious` and `bold`, on the frozen ruleset (`assessment/rules-0.10.toml`; the rules text agents read was the one compiled in, identical to `assessment/rules-0.10.md`), full hints, 60 s turns, five matches at a time, $10 per coach run, with batch 4's server build. The 20 matches played from 15:59 to 16:48 without hitting a session limit; three coaches stopped early and were resumed once, five were stopped by their budget, and every match reached its final whistle. They were read with the yardstick of batches 2 to 5, the first-generation `cautious`, before the first generation was retired, so these yardstick rows can no longer be replayed.

| Model | Matches | W-D-L | Goals | Safe-first | Failures per match | Ready at failure | Mean gap | Risky gaps over 5 pts | Chance taken | After a failure | Turns lost to the clock | Refused calls | Resumed | Unfinished | Cost | Minutes per match |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| claude-haiku-5-5 against cautious | 10 | 3-6-1 | 4-1 | 0.83 | 7.3 | 6.5 | 1.1 pts | 11 | 72.4 % | 70.4 % | 6 | 6 | 1 | 0 | $106.85 | 9.5 |
| claude-haiku-5-5 against bold | 10 | 2-4-4 | 3-6 | 0.82 | 7.4 | 6.2 | 1.5 pts | 17 | 71.6 % | 70.5 % | 11 | 11 | 2 | 0 | $118.99 | 10.8 |

| Bot | Safe-first | Failures per match | Ready at failure | Mean gap | Risky gaps over 5 pts | Chance taken | After a failure |
|---|---|---|---|---|---|---|---|
| cautious | 0.39 | 5.4 | 6.5 | 0.3 pts | 6 | 80.0 % | 78.1 % |
| bold | 0.46 | 13.1 | 6.1 | 0.3 pts | 8 | 70.1 % | 70.7 % |

| Yardstick (`cautious`) | Matches | W-D-L | Goals | Safe-first | Failures per match | Ready at failure | Mean gap | Risky gaps over 5 pts | Chance taken | After a failure |
|---|---|---|---|---|---|---|---|---|---|---|
| against cautious | 10 | 1-8-1 | 1-1 | 0.44 | 5.4 | 6.5 | 0.0 pts | 0 | 80.9 % | 78.0 % |
| against bold | 10 | 3-2-5 | 4-5 | 0.44 | 5.7 | 6.1 | 0.3 pts | 7 | 81.0 % | 82.6 % |

| Batch | Safe-first at least 0.90 | Mean gap under 5 pts | Under 10 % of risky orders 5 pts below the best | No tilt (within 5 pts after a failure) | Under one refused call per two turns | Goal difference at least the yardstick's |
|---|---|---|---|---|---|---|
| against cautious | fail | pass | pass | pass | pass | pass |
| against bold | fail | pass | pass | pass | pass | fail |

### Batches 3 to 5 read against their yardsticks, 19 Oct 2026

The bridge splits the change from batch 3 to batch 4 in two, each step changing one factor: the rules from batch 3 to batch 5, the opponents from batch 5 to batch 4. Each condition is read against its yardstick on the same seeds and seats; batch 4 shows the first-generation `cautious` yardstick, then `cautious-2`, the yardstick from now on. Batch 3's matches did not name their bot yet, so their yardstick was read from a copy naming it after the batch folder.

| Opponent's line | | Batch 3: rules 0.7, first generation | Batch 5: rules 0.10, first generation | Batch 4: rules 0.10, second generation |
|---|---|---|---|---|
| cautious | Haiku | 3-4-3, 11-10 | 3-6-1, 4-1 | 1-7-2, 1-3 |
| | yardstick | 5-0-5, 10-10 | 1-8-1, 1-1 | 2-6-2, 2-2 / 0-10-0, 0-0 |
| | goal difference against the yardstick's | +1 | +3 | −2 / −2 |
| | safe-first, Haiku (bot) | 0.78 (0.37) | 0.83 (0.39) | 0.79 (0.86) |
| bold | Haiku | 2-6-2, 9-11 | 2-4-4, 3-6 | 2-3-5, 2-6 |
| | yardstick | 6-2-2, 18-10 | 3-2-5, 4-5 | 1-6-3, 2-4 / 3-5-2, 3-2 |
| | goal difference against the yardstick's | −10 | −2 | −2 / −5 |
| | safe-first, Haiku (bot) | 0.76 (0.43) | 0.82 (0.46) | 0.73 (0.95) |
| balanced-2 | Haiku | | | 1-6-3, 4-6 |
| | yardstick | | | 0-10-0, 1-1 / 1-9-0, 1-0 |
| | goal difference against the yardstick's | | | −2 / −3 |
| | safe-first, Haiku (bot) | | | 0.83 (0.94) |

What the bridge says:

- **The rules alone, from batch 3 to batch 5:** the goals collapse (21 to 5 in the cautious line, 20 to 9 in the bold line), Haiku fails less (10.0 and 10.1 failures per match down to 7.3 and 7.4) and its safe-first index rises by five or six hundredths (0.78 to 0.83, 0.76 to 0.82). Against the yardstick its results improve, from +1 to +3 and from −10 to −2, mostly because the cautious coach no longer wins big against the bold one once the game is closed.
- **The opponents alone, from batch 5 to batch 4:** Haiku's results turn negative (+3 to −2 in the cautious line, −3 to −4 in the bold one), two goals under the yardstick in both lines. Its safe-first index drops a little against `cautious-2` (0.83 to 0.79) and more against `bold-2` (0.82 to 0.73), while the second-generation bots order their own turns at 0.86 to 0.95, against 0.39 to 0.46 for the first generation.
- **So batch 4's fall in results comes from the stronger opponents, not from the rules.** The safe-first gap to 0.90 is Haiku's own under every rules and opponent tried, between 0.73 and 0.83, and its odds reading passes everywhere.
- **Against `cautious-2`, the yardstick from now on**, Haiku is two to five goals short in every batch-4 line, the baseline for the next batches on the frozen benchmark.
- **Small samples:** 10 matches and 1 to 21 goals per condition, so a goal or two either way is within what the dice alone can do; the safe-first index pools hundreds of pairs per batch and moves by a few hundredths.

### Batch 6, 19 Oct 2026: Haiku 5.5 at $20 on the frozen benchmark

Seeds 1 to 5 from both seats against each of `balanced-2`, `bold-2` and `cautious-2`, on the frozen benchmark (`assessment/rules-0.10.toml` and `.md`), full hints, 60 s turns, five matches at a time, $20 per coach run. The 30 matches played from 22:42 to 23:55; a session limit at 23:22 stopped the five matches under way against `bold-2`, which were replayed from scratch once the limit was reset by hand at 23:25. No coach hit its budget or needed a resume, and every match reached its final whistle.

| Model | Matches | W-D-L | Goals | Safe-first | Failures per match | Ready at failure | Mean gap | Risky gaps over 5 pts | Chance taken | After a failure | Turns lost to the clock | Refused calls | Resumed | Unfinished | Cost | Minutes per match |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| claude-haiku-5-5 against balanced-2 | 10 | 2-8-0 | 2-0 | 0.72 | 6.9 | 7.0 | 1.0 pts | 11 | 75.0 % | 76.3 % | 10 | 10 | 0 | 0 | $105.86 | 10.8 |
| claude-haiku-5-5 against bold-2 | 10 | 2-6-2 | 3-3 | 0.74 | 6.8 | 6.5 | 0.8 pts | 8 | 73.7 % | 73.5 % | 6 | 7 | 0 | 0 | $109.40 | 10.9 |
| claude-haiku-5-5 against cautious-2 | 10 | 2-6-2 | 2-2 | 0.75 | 9.5 | 6.7 | 0.3 pts | 3 | 73.6 % | 70.5 % | 3 | 7 | 0 | 0 | $76.35 | 8.1 |

| Bot | Safe-first | Failures per match | Ready at failure | Mean gap | Risky gaps over 5 pts | Chance taken | After a failure |
|---|---|---|---|---|---|---|---|
| balanced-2 | 0.92 | 4.6 | 4.5 | 0.2 pts | 2 | 77.0 % | 78.0 % |
| bold-2 | 0.91 | 9.0 | 5.0 | 0.3 pts | 6 | 73.2 % | 73.7 % |
| cautious-2 | 0.83 | 3.6 | 5.3 | 0.4 pts | 3 | 74.5 % | 74.1 % |

| Yardstick (`cautious-2`) | Matches | W-D-L | Goals | Safe-first | Failures per match | Ready at failure | Mean gap | Risky gaps over 5 pts | Chance taken | After a failure |
|---|---|---|---|---|---|---|---|---|---|---|
| against balanced-2 | 10 | 1-9-0 | 1-0 | 0.87 | 1.3 | 3.9 | 0.2 pts | 1 | 76.8 % | 77.4 % |
| against bold-2 | 10 | 3-5-2 | 3-2 | 0.86 | 2.1 | 3.8 | 0.2 pts | 2 | 78.5 % | 77.2 % |
| against cautious-2 | 10 | 0-10-0 | 0-0 | 0.88 | 1.3 | 4.1 | 0.0 pts | 0 | 79.0 % | 73.6 % |

| Batch | Safe-first at least 0.90 | Mean gap under 5 pts | Under 10 % of risky orders 5 pts below the best | No tilt (within 5 pts after a failure) | Under one refused call per two turns | Goal difference at least the yardstick's |
|---|---|---|---|---|---|---|
| against balanced-2 | fail | pass | pass | pass | pass | pass |
| against bold-2 | fail | pass | pass | pass | pass | fail |
| against cautious-2 | fail | pass | pass | pass | pass | pass |

### Batch 7, 19 and 20 Oct 2026: Sonnet 5.5 at $30 on the frozen benchmark

Batch 6's settings with Sonnet 5.5 and $30 per coach run. The 30 matches started at 23:55; a session limit at 00:28 stopped the five matches under way against `cautious-2`, which were replayed from scratch after the reset at 04:20, and the batch ended at 04:34. No coach hit its budget or needed a resume, and every match reached its final whistle.

| Model | Matches | W-D-L | Goals | Safe-first | Failures per match | Ready at failure | Mean gap | Risky gaps over 5 pts | Chance taken | After a failure | Turns lost to the clock | Refused calls | Resumed | Unfinished | Cost | Minutes per match |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| claude-sonnet-5-5 against balanced-2 | 10 | 1-9-0 | 2-1 | 0.66 | 8.3 | 6.4 | 1.7 pts | 18 | 75.5 % | 74.2 % | 1 | 8 | 0 | 0 | $37.23 | 6.7 |
| claude-sonnet-5-5 against bold-2 | 10 | 4-6-0 | 4-0 | 0.53 | 8.7 | 6.3 | 0.7 pts | 6 | 77.0 % | 75.9 % | 0 | 11 | 0 | 0 | $36.61 | 6.4 |
| claude-sonnet-5-5 against cautious-2 | 10 | 5-2-3 | 6-3 | 0.55 | 7.0 | 6.8 | 0.4 pts | 4 | 76.6 % | 77.9 % | 0 | 10 | 0 | 0 | $34.67 | 5.5 |

| Bot | Safe-first | Failures per match | Ready at failure | Mean gap | Risky gaps over 5 pts | Chance taken | After a failure |
|---|---|---|---|---|---|---|---|
| balanced-2 | 0.92 | 4.2 | 3.6 | 0.2 pts | 2 | 78.3 % | 78.1 % |
| bold-2 | 0.94 | 8.4 | 4.2 | 0.3 pts | 5 | 74.4 % | 74.6 % |
| cautious-2 | 0.88 | 2.5 | 5.1 | 0.6 pts | 5 | 75.0 % | 74.0 % |

The yardstick plays the same seeds and seats as in batch 6, so it reads the same: 1-9-0 (1-0), 3-5-2 (3-2) and 0-10-0 (0-0).

| Batch | Safe-first at least 0.90 | Mean gap under 5 pts | Under 10 % of risky orders 5 pts below the best | No tilt (within 5 pts after a failure) | Under one refused call per two turns | Goal difference at least the yardstick's |
|---|---|---|---|---|---|---|
| against balanced-2 | fail | pass | pass | pass | pass | pass |
| against bold-2 | fail | pass | pass | pass | pass | pass |
| against cautious-2 | fail | pass | pass | pass | pass | pass |

What batches 6 and 7 say:

- **Haiku's results improved with the budget raised.** Nothing stopped a coach at $20 (the dearest match cost $13.62), and Haiku went from 4-16-10 with 7 goals to 15 in batch 4 to 6-20-4 with 7 to 5, passing the results criterion against `balanced-2` and `cautious-2`. Its safe-first index did not rise (0.72 to 0.75, against 0.73 to 0.83 at $10), and it still lost 19 turns to the clock, each with the coach at work when its 60 seconds ran out. With 10 matches per opponent, part of the swing in results may be chance.
- **Sonnet 5.5 gets the best results so far and the lowest safe-first index.** It went 10-17-3 with 12 goals to 4 and passed the results criterion against all three bots, but ordered its turns at 0.53 to 0.66, under Haiku and far under the bots (0.83 to 0.94).
- **Its gambles before safe moves are ordinary ones.** Of its risky orders, 20 to 26 % came before another goblin's order that was already safe, against 9 to 18 % for Haiku; for both models those gambles had about an 80 % chance, and only 4 to 15 % of them 90 % or more.
- **Sonnet plays fast and decisively.** It previews before 10 to 13 % of its plays where Haiku does before about half, gives no estimate, finishes goblins with brutal assaults far more often (65 against 6), loses one turn to the clock in 30 matches, and costs $3.62 a match against $9.72 for Haiku, in about 6 minutes against 10.
- **Results and safe-first now point in opposite directions.** Against these bots, playing safe moves first is not what wins: Sonnet wins with more gambles first. The index still measures what the protocol set out to watch, the exposure of safe work to a turn-ending failure, but reading it next to what misordering actually cost (safe orders lost when a gamble failed first) would tell whether Sonnet's risk-taking is a weakness or a lucky streak. That measure is not part of the protocol yet.
