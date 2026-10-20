# Goblins Bowl

Goblin football in a bowl: a minimalist, turn-based tactics game played with dice, built in Rust with Bevy. Two teams of goblins from folklore (dokkaebi, leprechauns, nisser, korrigans…) fight for a ball in a round bowl; every failed risky action ends your turn, so the game is about managing risk.

- Hot-seat: two humans on one machine, 60-second turns, 2 halves of 12 turns per team, on a 21 × 21 bowl.
- End goal: AI agents coach a team through an MCP server, to assess how well an agent manages risk.

## Quick start

```bash
make run                              # two humans on this machine (Bevy window), opening on a menu to pick the arena
make run ARGS="--no-menu"             # the same, straight into the match on the rules file's arena
make run ARGS="--away bot"            # play against the computer, the balanced-2 bot by default
make run ARGS="--home bot --bot cautious-2"  # the computer on the home seat with another coach: --bot cautious-2, balanced-2, bold-2 or random
make run ARGS="--away bot --record"   # the same, its record saved to runs/ (make assess ARGS="runs/<file>.jsonl" reads it)
make run ARGS="--replay runs/<file>.jsonl"   # watch any recorded match again, played by humans, bots or agents
make run ARGS="--arena rectangle --arena-size medium"  # another bowl: circle, cross or rectangle; tiny (21), medium (31) or huge (51)
make run ARGS="--home-turn-seconds 90 --lang en"
make check                            # lint + unit + integrated tests
make balance ARGS="--games 1000 --home cautious-2 --away bold-2"
```

Rules 0.11 (French and English, printable for a paper playtest): [`docs/rules.md`](docs/rules.md). Every tunable number lives in `rules.toml`; how they play out between bots: [`docs/balance.md`](docs/balance.md).

## How to play

- A match against a person or a bot opens on a start menu: pick the arena, a circle, a cross or a rectangle, and its size (21, 31 or 51 squares from den to den), then **Jouer**. A match against an agent starts straight away on the arena given with `--arena` and `--arena-size`.
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
- Replay (`--replay`): the bar at the bottom left tells each order with its chance, in the colour of its risk, and in orange which goblins had a safe order that could have been played first. Its buttons go to the previous or the next turn, play or pause (Space), pick the speed ×1, ×2 or ×3 (keys 1, 2 and 3) and quit; Play again on the final sheet replays from the kick-off.

## Agents

```bash
make run ARGS="--away agent"          # you against Claude in the window: it prints how to connect the agent
make serve                            # headless MCP server on http://127.0.0.1:7878/mcp: an agent against the balanced-2 bot
make serve ARGS="--arena cross"       # the same in another bowl (--arena and --arena-size, as in the window)
make serve ARGS="--rules assessment/rules-0.10.toml --rules-text assessment/rules-0.10.md"   # the frozen assessment benchmark: its ruleset, and the rules agents read
scripts/agent-vs-agent.sh 4           # two Claude Code coaches, at most $4 each; everything lands in runs/<timestamp>/
GB_OPPONENT_MODEL=claude-sonnet-5-5 scripts/assessment-batch.sh duel claude-haiku-5-5   # an assessment batch between two models, seats swapped on each seed
```

With an agent seat, the window and the headless server both open an MCP endpoint and print the `claude mcp add` line (or the `scripts/coach.sh` line) for each agent seat. The match record and every tool call are saved in `runs/` at the final whistle and when the match is stopped. A first game against a Claude, Scaleway, Ollama or oMLX model, step by step: [`docs/play-against-an-agent.md`](docs/play-against-an-agent.md). How agents are assessed: [`docs/assessment.md`](docs/assessment.md); findings of the first spike: [`docs/agent-spike.md`](docs/agent-spike.md).

## Development

`make benchmark-matches` records how the second-generation bots play the frozen assessment ruleset (`assessment/`); a test fails as soon as they play it differently, since that would move the benchmark: run it again only after a bridging batch ([`docs/assessment.md`](docs/assessment.md)).

`GOBLINSBOWL_SCREENSHOT=/tmp/shot.png make run` renders the game offscreen, saves a PNG after a few frames and quits (works while the screen is locked), the start menu included. `GOBLINSBOWL_SCREENSHOT_FRAME` picks the frame, and `GOBLINSBOWL_SCRIPT="30:ready;60:click A5;90:click 12,6;120:done A6;150:end"` plays inputs before the capture: `ready`, a `click` on a goblin or a square, `hover` to rest the pointer on one, `done` for a goblin that stays put for the rest of its turn, `end` for the end of the turn, and `restart` for a new match; a script starts the match without the start menu.
