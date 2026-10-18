# Goblins Bowl

Goblin football in a bowl: a minimalist, turn-based tactics game played with dice, built in Rust with Bevy. Two teams of goblins from folklore (dokkaebi, leprechauns, nisser, korrigans…) fight for a ball in a round bowl; every failed risky action ends your turn, so the game is about managing risk.

- Hot-seat: two humans on one machine, 60-second turns, 2 halves of 12 turns per team, on a 21 × 21 bowl.
- End goal: AI agents coach a team through an MCP server, to assess how well an agent manages risk.

## Quick start

```bash
make run                              # two humans on this machine (Bevy window)
make run ARGS="--away bot"            # play against the computer, the cautious bot by default
make run ARGS="--home bot --bot balanced-2"  # the computer on the home seat with another coach: --bot cautious, bold, random, cautious-2, balanced-2 or bold-2
make run ARGS="--away bot --record"   # the same, its record saved to runs/ (make assess ARGS="runs/<file>.jsonl" reads it)
make run ARGS="--home-turn-seconds 90 --lang en"
make check                            # lint + unit + integrated tests
make balance ARGS="--games 1000 --home cautious --away bold"
```

Rules 0.8 (French and English, printable for a paper playtest): [`docs/rules.md`](docs/rules.md). Every tunable number lives in `rules.toml`; how they play out between bots: [`docs/balance.md`](docs/balance.md).

## How to play

- Between turns the bowl finishes playing out the last moves and their headlines, then announces the next team with a goblin aside. Press **Prêts !** when it shows: the camera swings behind your team and the clock starts.
- A green tile under one of your goblins means it can still play, a dark tile that it is done for the turn. Click a goblin to select it: the squares it can reach light up, green needs no dice, yellow, orange and red are riskier.
- The selected goblin's card shows at the bottom left (its dice, the steps it has left, its Health), and the card of the opponent under the pointer at the bottom right.
- The pointer shows what a click would do, coloured by its risk: an arrow to walk, a lightning bolt to dodge, an up arrow to pick up the ball, a star to score, a burst to assault, a ball to throw.
- Click a square to aim a walk, an enemy to aim an assault (your goblin walks next to it first; the ball carrier cannot assault), or a teammate to aim a throw when you hold the ball (a hand-off when it stands next to you). The chance of success shows at the bottom left; click the same square or goblin again to go.
- Each goblin has its Move and one action per turn: after an order it stays selected, so it can act and walk on with the squares it has left, now or later in the turn.
- **Annuler** or Escape lets the selected goblin go.
- **Fin du tour** ends the turn, which no longer ends on its own: once no goblin can play, a reminder shows and the button pulses. A second click confirms while goblins can still play.
- At the final whistle, a scoresheet shows what each team did; **Rejouer** starts a new match and **Quitter** closes the game.
- Camera: Q/E or right-drag to turn, WASD/arrows or middle-drag to pan, the wheel to zoom, R to reset. P pauses a match between humans.

## Agents

```bash
make run ARGS="--away agent"          # you against Claude in the window: it prints how to connect the agent
make serve                            # headless MCP server on http://127.0.0.1:7878/mcp: an agent against the cautious bot
scripts/agent-vs-agent.sh 4           # two Claude Code coaches, at most $4 each; everything lands in runs/<timestamp>/
```

With an agent seat, the window and the headless server both open an MCP endpoint and print the `claude mcp add` line (or the `scripts/coach.sh` line) for each agent seat. The match record and every tool call are saved in `runs/` at the final whistle and when the match is stopped. A first game against a Claude, Scaleway, Ollama or oMLX model, step by step: [`docs/play-against-an-agent.md`](docs/play-against-an-agent.md). How agents are assessed: [`docs/assessment.md`](docs/assessment.md); findings of the first spike: [`docs/agent-spike.md`](docs/agent-spike.md).

## Development

`GOBLINSBOWL_SCREENSHOT=/tmp/shot.png make run` renders the game offscreen, saves a PNG after a few frames and quits (works while the screen is locked). `GOBLINSBOWL_SCREENSHOT_FRAME` picks the frame, and `GOBLINSBOWL_SCRIPT="30:ready;60:click A5;90:click 12,6;120:done A6;150:end"` plays inputs before the capture: `ready`, a `click` on a goblin or a square, `hover` to rest the pointer on one, `done` for a goblin that stays put for the rest of its turn, `end` for the end of the turn, and `restart` for a new match.
