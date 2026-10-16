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

## Rules 0.3, 13 Oct 2026

Rules 0.3 as shipped in `rules.toml`: the kick-off drops the ball 4 to 7 squares into the receiving half and up to 5 rows to either side, then bounces three times; teams are 3 Bruisers, 4 Sprinters and 2 Throwers; a Stance bonus counts at most +1 for the goblin itself and +1 for its braced neighbours; a dodge is an agility test, minus 1 per adjacent enemy and 2 per braced one, against 2. Same coaches, seeds 1 to 1,000.

| Home vs away | Home wins | Away wins | Draws | Goals per match | Stances per team turn | Knock-outs per match |
|---|---|---|---|---|---|---|
| cautious vs cautious | 10.7 % | 3.7 % | 85.6 % | 0.16 | 4.64 | 0.99 |
| bold vs bold | 37.1 % | 21.3 % | 41.6 % | 0.95 | 1.79 | 6.29 |
| cautious vs bold | 24.7 % | 8.4 % | 66.9 % | 0.44 | 3.21 | 4.09 |
| bold vs cautious | 20.2 % | 13.2 % | 66.6 % | 0.44 | 3.28 | 3.76 |
| cautious vs random | 99.8 % | 0 % | 0.2 % | 3.32 | 1.44 | 3.35 |
| bold vs random | 99.8 % | 0 % | 0.2 % | 3.20 | 1.01 | 3.03 |

What the numbers say:

- **More goals, but less open than the two knobs were.** The cautious mirror scores three times as often as under 0.2 (0.16 goals per match) and still draws 85.6 % of its matches; the bold mirror scores 0.95 goals and draws 41.6 %. Rules 0.2 with `kickoff_depth = 3` and `stance_bonus_cap = 1` had gone further (53.5 % and 35.4 % draws).
- **Careful coaches no longer dodge.** An order with a dodge now hardly ever clears the cautious coach's thresholds (a Sprinter leaving one enemy has 50 %, a Bruiser none), so the cautious mirror makes no dodge at all and braces more than ever (4.6 stances per team turn, 3.7 under 0.2): a marked goblin stays marked. The bold mirror tries 2.0 dodges per match and makes half of them; matches against the random coach, which dodges at any odds, see 10 or 11 dodges, 12 to 21 % of them made.
- **A bigger risk appetite still does not pay against a careful opponent.** Over both seats, cautious wins 19.0 % of its 2,000 matches against bold, bold 14.3 % against cautious.
- **The home edge grows**: home wins more than away in every pairing of scripted coaches (37.1 % against 21.3 % in the bold mirror, 10.7 % against 3.7 % in the cautious one), while receiving the opening kick-off still gives nothing (the receiver wins about half of the decided matches). The scripted coaches' tie-breaks by square order remain the suspect, still unchecked.
- **Still fights rather than passes.** The bold mirror makes 44.0 assaults and 3.5 throws per match, against 35.6 and 1.0 under 0.2, and no pass is ever intercepted.

Two of the 0.3 choices undone one at a time, same seeds:

| Variant | Cautious mirror: draws, goals per match | Bold mirror: draws, goals per match | Bold mirror: knock-outs per match |
|---|---|---|---|
| rules 0.3 | 85.6 %, 0.16 | 41.6 %, 0.95 | 6.29 |
| rules 0.3 with contest dodges (`dodges_test_agility = false`) | 79.6 %, 0.25 | 37.3 %, 1.21 | 5.50 |
| rules 0.3 with a Stance bonus of +1 at most (`stance_bonus_cap = 1`) | 70.0 %, 0.38 | 34.1 %, 1.22 | 7.62 |
| rules 0.2 with both knobs, for reference | 53.5 %, 0.72 | 35.4 %, 1.27 | 8.05 |

Both choices hold the game back, the Stance cap more than the dodge: with +1 at most, the cautious mirror scores more than twice as often, while bold matches get bloodier (54.5 assaults and 7.6 knock-outs per match). With contest dodges, careful coaches dodge again (0.6 per match, 85 % made). Neither variant opens the game as far as rules 0.2 with both knobs did; the kick-off, which now drops the ball deep in the receiving half rather than three squares in front of its line, may account for the rest, not measured.

## Rules 0.4, 13 Oct 2026

Rules 0.3 with the 22-point archetypes: Bruiser Attack 4, Defense 4, Agility 3, Health 7, Move 4; Sprinter 3, 2, 6, 5, 6; Thrower 3, 3, 6, 6, 4. Same coaches, seeds 1 to 1,000.

| Home vs away | Home wins | Away wins | Draws | Goals per match | Stances per team turn | Knock-outs per match |
|---|---|---|---|---|---|---|
| cautious vs cautious | 13.9 % | 13.9 % | 72.2 % | 0.36 | 4.29 | 0.73 |
| bold vs bold | 38.6 % | 29.7 % | 31.7 % | 1.61 | 1.58 | 2.97 |
| cautious vs bold | 38.1 % | 16.7 % | 45.2 % | 0.97 | 2.63 | 2.38 |
| bold vs cautious | 22.8 % | 33.3 % | 43.9 % | 0.95 | 2.65 | 2.26 |
| cautious vs random | 99.9 % | 0 % | 0.1 % | 4.34 | 1.37 | 1.35 |
| bold vs random | 100.0 % | 0 % | 0 % | 4.62 | 1.10 | 1.22 |

What the numbers say:

- **Goals double, knock-outs halve.** The cautious mirror scores 0.36 goals per match (0.16 under 0.3) and draws 72.2 % of its matches; the bold mirror scores 1.61 and draws 31.7 %. A single won assault can no longer knock a goblin out, so the bold mirror loses 3.0 goblins per match instead of 6.3.
- **The ball moves.** With d6 agility, an open throw and its catch succeed 69 % of the time (56 % before): the bold mirror throws 6.5 times per match (3.5 under 0.3), the cautious mirror 2.2 (0.5). No pass is ever intercepted yet, although a Sprinter or a Thrower under the flight line now snatches it two times in three.
- **A bigger risk appetite pays even less against a careful opponent.** Over both seats, cautious wins 35.7 % of its 2,000 matches against bold, bold 19.8 % against cautious.
- **The home edge shrinks**: none in the cautious mirror (13.9 % each), 38.6 % against 29.7 % in the bold mirror (37.1 % against 21.3 % under 0.3).
- **Still more closed than 0.2 with both knobs** (53.5 % draws in the cautious mirror): careful coaches keep bracing (4.3 stances per team turn) and hardly dodge (0.2 per match).

## Rules 0.5, 13 Oct 2026

Rules 0.4 without Stance: no goblin braces, an action is an assault or a throw. Same coaches, seeds 1 to 1,000.

| Home vs away | Home wins | Away wins | Draws | Goals per match | Knock-outs per match |
|---|---|---|---|---|---|
| cautious vs cautious | 40.0 % | 32.2 % | 27.8 % | 1.71 | 6.59 |
| bold vs bold | 41.2 % | 31.1 % | 27.7 % | 2.10 | 4.99 |
| cautious vs bold | 43.2 % | 29.0 % | 27.8 % | 1.86 | 5.89 |
| bold vs cautious | 38.0 % | 35.2 % | 26.8 % | 1.99 | 5.71 |
| cautious vs random | 99.8 % | 0 % | 0.2 % | 4.43 | 1.66 |
| bold vs random | 100.0 % | 0 % | 0 % | 4.26 | 1.64 |

What the numbers say:

- **The walls are gone.** The cautious mirror scores 1.71 goals per match, almost five times as often as under 0.4, and draws 27.8 % of its matches instead of 72.2 %; the bold mirror scores 2.10 and draws 27.7 %. For the first time, both mirrors play the same open game.
- **Careful coaches fight instead of bracing.** With nothing certain left to do in contact, the cautious mirror makes 55.5 assaults per match (7.7 under 0.4), wins 84 % of them, and knocks out 6.6 goblins per match. Bold coaches knock out fewer (5.0).
- **The ball changes hands.** 17 to 21 pick-ups and 7 to 11 throws per match between scripted coaches, just over half of the throws caught; still no interception.
- **A bigger risk appetite still does not pay against a careful opponent**, though by less: over both seats, cautious wins 39.2 % of its 2,000 matches against bold, bold 33.5 % against cautious.
- **The home edge is back in every pairing** (40.0 % against 32.2 % in the cautious mirror, 41.2 % against 31.1 % in the bold one), while receiving the opening kick-off still gives nothing. Now that matches are decided, checking the scripted coaches' tie-breaks is worth doing before trusting home and away results.

## Rules 0.6, 14 Oct 2026

Rules 0.5 with hand-offs, carriers that cannot assault, and missed bouncing balls that no longer end the turn. Same coaches, seeds 1 to 1,000.

| Home vs away | Home wins | Away wins | Draws | Goals per match | Knock-outs per match |
|---|---|---|---|---|---|
| cautious vs cautious | 41.7 % | 30.0 % | 28.3 % | 1.77 | 5.88 |
| bold vs bold | 41.4 % | 32.8 % | 25.8 % | 2.22 | 4.42 |
| cautious vs bold | 40.4 % | 33.2 % | 26.4 % | 1.98 | 5.13 |
| bold vs cautious | 41.2 % | 32.0 % | 26.8 % | 2.09 | 5.04 |
| cautious vs random | 99.5 % | 0 % | 0.5 % | 4.32 | 1.56 |
| bold vs random | 100.0 % | 0 % | 0 % | 4.27 | 1.58 |

What the numbers say:

- **A little more flow.** Fumbles per team turn fall from 0.171 to 0.143 in the cautious mirror and from 0.246 to 0.224 in the bold one, now that a missed bouncing ball no longer ends the turn; goals rise slightly (1.77 and 2.22 per match) and throws too (7.2 and 11.5), hand-offs included.
- **Fewer knock-outs** (5.9 and 4.4 per match instead of 6.6 and 5.0): carriers no longer fight.
- **For the first time, risk appetite does not cost against a careful opponent**: over both seats, bold wins 37.2 % of its 2,000 matches against cautious, cautious 36.2 % against bold.
- **The home edge stays** at 7 to 12 points in every pairing between scripted coaches; their tie-breaks remain unchecked.

## Rules 0.7, 16 Oct 2026

Rules 0.6 on the round bowl, with Health that lasts the whole match. Same coaches, seeds 1 to 1,000.

| Home vs away | Home wins | Away wins | Draws | Goals per match | Knock-outs per match |
|---|---|---|---|---|---|
| cautious vs cautious | 40.0 % | 36.2 % | 23.8 % | 2.06 | 6.05 |
| bold vs bold | 43.6 % | 36.9 % | 19.5 % | 2.56 | 5.84 |
| cautious vs bold | 38.2 % | 38.6 % | 23.2 % | 2.36 | 5.96 |
| bold vs cautious | 41.8 % | 37.5 % | 20.7 % | 2.41 | 5.96 |
| cautious vs random | 100.0 % | 0 % | 0 % | 4.76 | 2.09 |
| bold vs random | 100.0 % | 0 % | 0 % | 4.59 | 2.60 |

What the numbers say:

- **More goals, fewer draws.** The cautious mirror scores 2.06 goals per match (1.77 under 0.6) and draws 23.8 % of its matches; the bold mirror scores 2.56 and draws 19.5 %. In most pairings the second half scores a little more than the first: goblins knocked out early no longer come back.
- **The home edge shrinks**: 3.8 points in the cautious mirror (11.7 under 0.6), 6.7 in the bold one, and the away team even wins slightly more when cautious hosts bold. The cross shape played a part in the edge; the scripted coaches' tie-breaks are still unchecked.
- **Bold now edges cautious**: over both seats, bold wins 40.2 % of its 2,000 matches against cautious, cautious 37.9 % against bold.
- Both changes came together, so these runs do not say how much each one weighs.

