# Balance runs (M4), 10 Oct 2026

How rules 0.1 play out between scripted coaches, measured with `make balance`: 1,000 seeded matches per line (seeds 1 to 1,000), the same seeds for every line.

```bash
make balance ARGS="--games 1000 --home cautious --away bold --rules rules.toml"
```

## The coaches

- **random** plays legal commands at random: the sparring partner of the property tests.
- **cautious** and **bold** are the same scripted coach after the Bot Bowl recipe, with two risk appetites. In order of priority they score, assault the enemy carrier, pick up the loose ball, hit the carrier's markers, run Sprinters ahead into open space, pass along a clear line, advance the carrier and brace it, guard it without standing in its lane, hit, close in on the ball without dice, and brace any goblin in contact. A risk is taken only above the coach's appetite:

| Lowest chance played for | Score | Sack the carrier | Pick up | Pass | Free the carrier, advance, hit |
|---|---|---|---|---|---|
| cautious | 70 % | 50 % | 50 % | 50 % | 75 % |
| bold | 50 % | 33 % | 33 % | 33 % | 50 % |

## Rules 0.1

| Home vs away | Home wins | Away wins | Draws | Goals per match | Stances per team turn | Knock-outs per match |
|---|---|---|---|---|---|---|
| cautious vs cautious | 0.7 % | 0.4 % | 98.9 % | 0.01 | 4.99 | 0.62 |
| bold vs bold | 11.3 % | 11.2 % | 77.5 % | 0.25 | 3.25 | 5.50 |
| cautious vs bold | 7.1 % | 3.7 % | 89.2 % | 0.12 | 4.04 | 3.67 |
| bold vs cautious | 5.3 % | 7.0 % | 87.7 % | 0.13 | 4.33 | 2.37 |
| cautious vs random | 76.4 % | 0 % | 23.6 % | 1.15 | 1.87 | 2.83 |
| bold vs random | 97.2 % | 0 % | 2.8 % | 2.15 | 1.19 | 2.47 |

What the numbers say:

- **Defence wins.** Two competent coaches score 0.01 to 0.25 goals per match; the cautious mirror ends 0-0 in 98.9 % of matches. Both scripted coaches beat the random one and never concede, so the coaches can score: the rules stop them.
- **Three causes, seen in replays.** The kick-off drops the ball between two front lines standing two squares apart, so whoever picks it up is marked at once. Stance stacks with every braced neighbour and lasts through the opponent's turn: half of each team braces every turn, a marker stands at d4+2 or more, and dodges through a line almost vanish (0.0 to 1.3 per match). Without a push, a defender standing in front of the carrier blocks it for good.
- **A bigger risk appetite does not pay against a careful opponent.** Cautious beats bold slightly from both seats (7.1 % against 3.7 %, 7.0 % against 5.3 %), with more draws than either mirror.
- **No side bias**, and receiving the opening kick-off gives no edge (the receiver wins as often as the other team).
- **Passing is rare and never intercepted**: 0.0 to 0.3 throws per match between scripted coaches, 50 to 63 % caught.

## Two knobs, measured

`rules.toml` now has `kickoff_depth` (squares the kick-off drops the ball into the receiving half) and `stance_bonus_cap` (most a Stance bonus adds to a roll). Their defaults, 0 and 5, keep rules 0.1 unchanged.

| Variant | Cautious mirror: draws, goals per match | Bold mirror: draws, goals per match | Bold mirror: knock-outs per match |
|---|---|---|---|
| rules 0.1 | 98.9 %, 0.01 | 77.5 %, 0.25 | 5.50 |
| `kickoff_depth = 3` | 90.1 %, 0.10 | 59.0 %, 0.56 | 7.60 |
| `stance_bonus_cap = 1` | 87.2 %, 0.13 | 56.8 %, 0.58 | 9.40 |
| both | 77.2 %, 0.29 | 44.0 %, 0.86 | 9.63 |

Both help and they add up, but the game stays low-scoring, and a capped Stance turns bold matches bloody (half a team knocked out per match). The block that remains is structural: nothing moves a defender out of the carrier's lane.

## Options for rules 0.2

1. **Adopt both knobs now** (`kickoff_depth = 3`, `stance_bonus_cap = 1`): two numbers in `rules.toml`, goals multiplied by 3 to 30 depending on the coaches.
2. **Pushes**, already planned after 0.1: a won assault pushes the defender one square away from the attacker, out of bounds meaning a KO. It opens lanes the way blocks do in Blood Bowl, and needs destination rules (occupied squares, chain pushes or none).
3. **Space the kick-off formation**, for example front lines four squares apart instead of two.
4. **Make Stance cost something**, for example no Stance after moving, or a Stance bonus for neighbours only when the braced goblin did not move.

For the agent assessment this matters too: under rules 0.1 agents will rarely score, so risk shows mostly in fights and pick-ups. Running the assessment on the 0.2 candidate would give more decisions with real stakes.

## Rules 0.2, 12 Oct 2026

Rules 0.2 as shipped in `rules.toml`: the 21 × 21 bowl with 11-square dens, 12 turns per half, the scrimmage kick-off formation, goblins that keep moving after their action, and turns that wait for their coach; `kickoff_depth` and `stance_bonus_cap` keep their 0.1 values. Same coaches, seeds 1 to 1,000.

| Home vs away | Home wins | Away wins | Draws | Goals per match | Stances per team turn | Knock-outs per match |
|---|---|---|---|---|---|---|
| cautious vs cautious | 2.8 % | 1.8 % | 95.4 % | 0.05 | 3.71 | 0.23 |
| bold vs bold | 30.8 % | 18.3 % | 50.9 % | 0.70 | 2.35 | 5.86 |
| cautious vs bold | 15.6 % | 7.8 % | 76.6 % | 0.26 | 3.19 | 4.04 |
| bold vs cautious | 11.1 % | 9.9 % | 79.0 % | 0.23 | 3.04 | 3.66 |
| cautious vs random | 93.4 % | 0 % | 6.6 % | 1.84 | 1.73 | 2.24 |
| bold vs random | 99.1 % | 0 % | 0.9 % | 2.75 | 1.13 | 1.77 |

What the numbers say:

- **More goals, but careful coaches still wall up.** Goals per match rise in every pairing, from 0.25 to 0.70 in the bold mirror, whose draws fall from 77.5 % to 50.9 %. Two cautious coaches still draw 95.4 % of their matches with 3.7 stances per team turn: room to run does not break a Stance wall on its own.
- **A bigger risk appetite still does not pay against a careful opponent.** Over both seats, cautious wins 12.8 % of its 2,000 matches against bold, bold 9.5 % against cautious.
- **A home edge appears** (30.8 % against 18.3 % in the bold mirror, home ahead in both mixed pairings) where rules 0.1 showed none, while receiving the opening kick-off still gives nothing. The rules are symmetric under the east-west mirror, the scripted coaches are not: they break ties by square order, which picks different moves for a team attacking east than for one attacking west. Worth checking before reading it as a bias of the rules.
- **Fights rather than passes.** The bold mirror makes 35.6 assaults and 1.0 throw per match, and no pass is ever intercepted.

The two knobs on top of rules 0.2 (`kickoff_depth = 3`, `stance_bonus_cap = 1`), same seeds:

| Variant | Cautious mirror: draws, goals per match | Bold mirror: draws, goals per match | Bold mirror: knock-outs per match |
|---|---|---|---|
| rules 0.2 | 95.4 %, 0.05 | 50.9 %, 0.70 | 5.86 |
| rules 0.2 with both knobs | 53.5 %, 0.72 | 35.4 %, 1.27 | 8.05 |

With both knobs the cautious mirror ends level in only half of its matches and scores fourteen times as often, the largest shift measured so far, while bold matches get bloodier (8 knock-outs per match).
