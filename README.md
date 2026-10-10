# Goblins Bowl

Goblin football in a bowl: a minimalist, turn-based tactics game played with dice, built in Rust with Bevy. Two teams of goblins from folklore (dokkaebi, leprechauns, nisser, korrigans…) fight for a ball in a cross-shaped arena; every failed risky action ends your turn, so the game is about managing risk.

- Hot-seat: two humans on one machine, 60-second turns, 2 halves of 10 turns per team.
- End goal: AI agents coach a team through an MCP server, to assess how well an agent manages risk.

## Quick start

```bash
make run                              # two humans on this machine (Bevy window)
make run ARGS="--away bot"            # play against the computer (--bot cautious, bold or random)
make run ARGS="--home-turn-seconds 90 --lang en"
make check                            # lint + unit + integrated tests
make balance ARGS="--games 1000 --home cautious --away bold"
```

Rules 0.1 (French and English, printable for a paper playtest): [`docs/rules.md`](docs/rules.md). Every tunable number lives in `rules.toml`; how they play out between bots: [`docs/balance.md`](docs/balance.md).

## How to play

- Press **Prêts !** when your turn comes: the camera swings behind your team and the clock starts.
- Click one of your goblins (green ring: it can still act). The squares it can reach light up: green needs no dice, yellow, orange and red are riskier.
- Click a square to aim a walk, an enemy to aim an assault (your goblin walks next to it first), or a teammate to aim a throw when you hold the ball. The chance of success shows at the bottom left; click the same square or goblin again to go.
- **Garde** braces in Stance after the aimed walk, or in place. **Annuler** or Escape lets the goblin go.
- **Fin du tour** ends the turn early (a second click confirms while goblins can still act).
- Camera: Q/E or right-drag to turn, WASD/arrows or middle-drag to pan, the wheel to zoom, R to reset. P pauses a match between humans.

## Agents

```bash
make run ARGS="--away agent"          # you against Claude in the window: it prints how to connect the agent
make serve                            # headless MCP server on http://127.0.0.1:7878/mcp: an agent against the cautious bot
scripts/agent-vs-agent.sh 3           # two Claude Code coaches, at most $3 each; everything lands in runs/<timestamp>/
```

With an agent seat, the window and the headless server both open an MCP endpoint and print the `claude mcp add` line (or the `scripts/coach.sh` line) for each agent seat. The match record and every tool call are saved in `runs/` at the final whistle and when the match is stopped. How agents are assessed: [`docs/assessment.md`](docs/assessment.md); findings of the first spike: [`docs/agent-spike.md`](docs/agent-spike.md).

## Development

`GOBLINSBOWL_SCREENSHOT=/tmp/shot.png make run` renders the game offscreen, saves a PNG after a few frames and quits (works while the screen is locked). `GOBLINSBOWL_SCREENSHOT_FRAME` picks the frame, and `GOBLINSBOWL_SCRIPT="30:ready;60:click A6;90:click 7,6"` plays inputs before the capture.
