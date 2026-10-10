# Goblins Bowl

Goblin football in a bowl: a minimalist, turn-based tactics game played with dice, built in Rust with Bevy. Two teams of goblins from folklore (dokkaebi, leprechauns, nisser, korrigans…) fight for a ball in a cross-shaped arena; every failed risky action ends your turn, so the game is about managing risk.

- Hot-seat: two humans on one machine, 60-second turns, 2 halves of 10 turns per team.
- End goal: AI agents coach a team through an MCP server, to assess how well an agent manages risk.

## Quick start

```bash
make run          # play (Bevy window)
make check        # lint + unit + integrated tests
```

Rules 0.1 (French and English, printable for a paper playtest): [`docs/rules.md`](docs/rules.md). Every tunable number lives in `rules.toml`.

Development: `GOBLINSBOWL_SCREENSHOT=/tmp/shot.png make run` renders the game offscreen, saves a PNG after a few frames and quits (works while the screen is locked).
