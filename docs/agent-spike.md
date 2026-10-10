# Agent spike (S1), 10 Oct 2026

Two short matches (one half, 2 turns per team, seed 7): Claude Code (`claude -p`, model `claude-sonnet-5`) coached home through the MCP server, the built-in random bot played away. Launched with `scripts/coach.sh`, `--max-budget-usd 1`.

## What works

- Streamable HTTP on `127.0.0.1` with a per-seat bearer token: Claude Code connects, sends no `Origin`, gets structured results; illegal plays come back as tool errors the model reads and acts on.
- The protocol is followed without strategy hints: `get_rules` once, `wait_for_my_turn`, one batched `play` per turn (4 to 5 activations), `end_turn=true`, stop at `game_over`.
- Results stay inside budget: about 1,700 tokens for `wait_for_my_turn`, 2,000 for `play`, 860 for `get_rules`.
- The agent fills `estimate` on its own (0.625 and 0.75 matched the shown odds), so calibration data comes for free.

## What the spike changed

| Finding (run 1) | Change | Effect (run 2) |
|---|---|---|
| The turn clock ran while the agent was still starting up and reading the rules | An agent's clock waits until it looks at its turn (any tool showing `your_turn`), with a 15 s grace | Start-up time no longer counts |
| A plan built on turn 1's board was executed on turn 2's board | `play` is refused when the turn changed since the seat last looked; `expected_version` stays available | The agent sent `expected_version`, got "stale plan", called `get_state` and replanned on fresh data |
| Claude Code logged `unknown format "uint8"` for every schema | Integer width formats removed from all schemas | Clean logs |

## Numbers

| | Run 1 | Run 2 |
|---|---|---|
| Wall time, two agent turns | 77 s | 105 s |
| Cost | $0.19 | $0.24 |
| First decision (cold context) | about 70 s, turn lost on time | over 60 s, turn lost on time |
| Second decision | 15 s | 38 s |
| Order of a 5-activation turn | assault 87.5 %, pick 75 %, then two certain Stances | three certain Stances, then assaults 93.8 % and 62.5 %, then the ball move |

## Decisions and open points

- **60 s is tight but playable after the first look**: the cold first decision of Sonnet 5 takes longer than a turn. Keep 60 s as the official condition (it is the production pressure we want to observe) and run comparison matches with `--turn-seconds 300`, as the assessment plan says.
- **One long session per match is fine**: context grows by about 2,000 tokens per call, so a full match (around 60 calls) fits; the Agent SDK orchestrator with a fresh context per turn stays optional.
- **Risk ordering already varies between runs** on the same seed and board: exactly the signal the safe-first metric is meant to capture.
- A full 20-turn match costs roughly $2 to $3 per agent with Sonnet 5, double that for agent vs agent.
