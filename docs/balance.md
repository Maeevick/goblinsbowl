# Balance runs (M4), 10 Oct 2026

How rules 0.1 play out between scripted coaches, measured with `make balance`: 1,000 seeded matches per line (seeds 1 to 1,000), the same seeds for every line.

```bash
make balance ARGS="--games 1000 --home cautious-2 --away bold-2 --rules rules.toml"
make balance ARGS="--games 1000 --home cautious-2 --away bold-2 --arena rectangle --arena-size medium"
```

## The coaches

- **random** plays legal commands at random: the sparring partner of the property tests.
- **cautious** and **bold** are the same scripted coach after the Bot Bowl recipe, with two risk appetites. In order of priority they score, assault the enemy carrier, pick up the loose ball, hit the carrier's markers, run Sprinters ahead into open space, pass along a clear line, advance the carrier and brace it, guard it without standing in its lane, hit, close in on the ball without dice, and brace any goblin in contact. A risk is taken only above the coach's appetite:

| Lowest chance played for | Score | Sack the carrier | Pick up | Pass | Free the carrier, advance, hit |
|---|---|---|---|---|---|
| cautious | 70 % | 50 % | 50 % | 50 % | 75 % |
| bold | 50 % | 33 % | 33 % | 33 % | 50 % |

- **cautious-2**, **balanced-2** and **bold-2**, since 17 Oct 2026: the second generation, see [the second-generation coaches](#second-generation-coaches-17-oct-2026). On 19 Oct 2026 the first generation, `cautious` and `bold`, was retired from the game and the assessment; the tables below that name them stay as they were measured, but no longer replay.

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

## Second-generation coaches, 17 Oct 2026

The first-generation coaches stay exactly as they were, so the tables above and the assessment batches keep their yardstick; on rules 0.7 their mirrors reproduce the table above to the last decimal. Three coaches join them:

- **cautious-2** and **bold-2** keep the appetites of cautious and bold and change two things. Ties between equally good squares are broken the same way for both teams, the square nearest the coach's own den first, then the northern one, where the first generation followed square order, which picked different squares for the team attacking east and the team attacking west; a property test checks that each new coach chooses the mirrored order on a mirrored bowl. And before a gamble, the coach first plays another goblin's certain move (running a Sprinter ahead, advancing the carrier or closing in on the ball) when that move leaves the gamble at least its chance.
- **balanced-2** blends the two appetites by what a failure would cost: the cautious thresholds while every other goblin can still play, the bold ones once none can, in proportion in between.

With `--risk`, every match is replayed to report each coach's own safe-first index, as [`assessment.md`](assessment.md) reads it for agents:

```bash
make balance ARGS="--games 1000 --home cautious-2 --away bold-2 --rules rules.toml --risk"
```

Rules 0.7, seeds 1 to 1,000:

| Home vs away | Home wins | Away wins | Draws | Goals per match | Knock-outs per match | Safe-first, home and away |
|---|---|---|---|---|---|---|
| cautious-2 vs cautious-2 | 28.8 % | 31.9 % | 39.3 % | 0.88 | 9.56 | 0.94, 0.93 |
| balanced-2 vs balanced-2 | 36.3 % | 32.2 % | 31.5 % | 1.03 | 10.56 | 0.94, 0.94 |
| bold-2 vs bold-2 | 38.5 % | 36.4 % | 25.1 % | 1.36 | 10.53 | 0.94, 0.94 |
| cautious-2 vs bold-2 | 30.0 % | 39.6 % | 30.4 % | 1.13 | 10.31 | 0.93, 0.95 |
| bold-2 vs cautious-2 | 43.1 % | 24.2 % | 32.7 % | 1.11 | 10.30 | 0.95, 0.93 |
| cautious-2 vs balanced-2 | 24.3 % | 37.0 % | 38.7 % | 0.92 | 10.26 | 0.93, 0.94 |
| balanced-2 vs cautious-2 | 38.2 % | 24.2 % | 37.6 % | 0.96 | 10.25 | 0.94, 0.93 |
| balanced-2 vs bold-2 | 37.5 % | 33.7 % | 28.8 % | 1.25 | 10.54 | 0.93, 0.95 |
| bold-2 vs balanced-2 | 36.0 % | 36.5 % | 27.5 % | 1.20 | 10.55 | 0.95, 0.94 |
| cautious-2 vs cautious | 67.5 % | 13.8 % | 18.7 % | 1.98 | 7.69 | 0.94, 0.32 |
| cautious vs cautious-2 | 15.9 % | 52.4 % | 31.7 % | 1.25 | 8.14 | 0.33, 0.94 |
| cautious vs cautious | 40.0 % | 36.2 % | 23.8 % | 2.06 | 6.05 | 0.37, 0.38 |
| bold vs bold | 43.6 % | 36.9 % | 19.5 % | 2.56 | 5.84 | 0.45, 0.46 |

What the numbers say:

- **Certain work first wins matches.** With the same appetite, cautious-2 wins 60.0 % of its 2,000 matches against cautious over both seats, cautious 14.9 %. The planning alone does it: the first-generation coach given the same planning, its tie-breaks untouched, wins 64.3 % against the plain one (16.6 %). The safe-first index climbs with it, from between 0.32 and 0.46 to between 0.93 and 0.95, so the measure tracks something that wins matches, and weighs far more than appetite does.
- **No home edge left.** Home wins minus away wins come to −3.1 points in the cautious-2 mirror, +4.1 in balanced-2's and +2.1 in bold-2's, all inside what chance allows over 1,000 matches (about ±5 points), where the first-generation bold mirror shows +6.7. The home edge seen since rules 0.2 came from the first generation's tie-breaks, not from the rules, at least under 0.7.
- **Appetite pays once turns are ordered.** Over both seats, bold-2 wins 41.4 % of its 2,000 matches against cautious-2, which wins 27.1 %; balanced-2 wins 37.6 % against cautious-2 (24.3 %) and 37.0 % against bold-2 (34.9 %), the last gap within chance. Between first-generation coaches appetite hardly mattered (bold 40.2 % against cautious 37.9 % under 0.7), most likely because their failed gambles also cost the certain moves still unplayed.
- **Tighter, bloodier matches.** The new mirrors score 0.88 to 1.36 goals per match where the first generation scored 2.06 and 2.56, draw 25 to 39 % of their matches, and knock out 9.6 to 10.6 goblins per match instead of about 6. Their safe-first index stops short of 1.0 because a coach holds back a certain move that would cost the gamble its chance, which the measure still counts against it, and looks for certain moves among three kinds only.

Two throwaway builds, measured once and not shipped, split the two changes on the cautious appetite:

| Variant | Mirror: home, away, draws | Goals per match | Knock-outs per match | Against cautious, both seats |
|---|---|---|---|---|
| cautious, first generation | 40.0 %, 36.2 %, 23.8 % | 2.06 | 6.05 | |
| first generation with the certain-first planning | 32.8 %, 36.9 %, 30.3 % | 1.42 | 9.20 | 64.3 % against 16.6 % |
| cautious-2 as shipped, ties toward its own den | 28.8 %, 31.9 %, 39.3 % | 0.88 | 9.56 | 60.0 % against 14.9 % |
| cautious-2 with ties toward the enemy den | 37.5 %, 39.1 %, 23.4 % | 2.04 | 8.90 | 64.9 % against 15.6 % |

The planning brings the knock-outs (57 assaults per match in its mirror, 46 without it); the direction of the tie-breaks sets the goal rate, most likely because goblins closing in on the enemy carrier with ties toward their own den end up between the carrier and that den. Ties toward the enemy den give back the first generation's 2 goals and 23 % draws, keep the mirror symmetric, and cost nothing measurable: head to head over 2,000 matches the shipped cautious-2 wins 37.8 %, the one with ties toward the enemy den 35.1 %, within chance. Which of the two the second generation keeps is still open.


## Rules 0.8, 18 Oct 2026

Rules 0.7 with nine changes: dens of 7 squares on the outer column, tied assaults that stand off, a carrier defending at −1, knocked-out goblins rolling to come back at each kick-off, assaults and throws at a step each and one of each kind per goblin, Stance back at +1 Defense for every step left, an eight-square reach for dodges, roll maluses and interceptions, and bounces on a d8. Seeds 1 to 1,000.

| Home vs away | Home wins | Away wins | Draws | Goals per match | Stances per team turn | Knock-outs per match |
|---|---|---|---|---|---|---|
| cautious vs cautious | 12.5 % | 10.9 % | 76.6 % | 0.27 | 5.01 | 2.65 |
| bold vs bold | 36.3 % | 24.9 % | 38.8 % | 0.97 | 1.72 | 8.20 |
| cautious vs bold | 30.7 % | 16.6 % | 52.7 % | 0.63 | 2.91 | 6.34 |
| bold vs cautious | 23.2 % | 25.3 % | 51.5 % | 0.68 | 2.93 | 6.18 |
| cautious vs random | 97.6 % | 0 % | 2.4 % | 2.58 | 1.42 | 2.52 |
| bold vs random | 99.3 % | 0 % | 0.7 % | 3.13 | 0.89 | 2.66 |
| cautious-2 vs cautious-2 | 1.9 % | 2.3 % | 95.8 % | 0.04 | 5.37 | 0.23 |
| balanced-2 vs balanced-2 | 2.7 % | 3.0 % | 94.3 % | 0.06 | 5.20 | 0.66 |
| bold-2 vs bold-2 | 11.5 % | 11.8 % | 76.7 % | 0.26 | 4.07 | 3.89 |
| cautious-2 vs bold-2 | 2.5 % | 9.3 % | 88.2 % | 0.12 | 4.99 | 1.13 |
| bold-2 vs cautious-2 | 8.3 % | 4.5 % | 87.2 % | 0.13 | 4.96 | 1.21 |

Each change undone on its own, first-generation mirrors, same seeds:

| Variant | Cautious mirror: draws, goals per match | Bold mirror: draws, goals per match | Bold mirror: knock-outs per match |
|---|---|---|---|
| rules 0.8 | 76.6 %, 0.27 | 38.8 %, 0.97 | 8.20 |
| a reach of four orthogonal squares | 40.9 %, 0.91 | 30.0 %, 1.48 | 8.51 |
| no Stance | 53.9 %, 0.61 | 33.6 %, 1.07 | 10.14 |
| dens of 11 squares | 60.5 %, 0.56 | 30.3 %, 1.51 | 6.97 |
| actions without a step to pay | 68.4 %, 0.40 | 33.3 %, 1.30 | 8.21 |
| tied assaults knocking down | 69.8 %, 0.36 | 36.2 %, 1.08 | 7.92 |
| one action in all | 71.7 %, 0.34 | 39.4 %, 0.99 | 8.95 |
| no return from a knock-out | 74.4 %, 0.31 | 33.3 %, 1.15 | 6.81 |
| bounces on a d4 | 75.2 %, 0.28 | 42.0 %, 0.94 | 8.35 |
| no carrier malus | 81.0 %, 0.21 | 41.6 %, 1.00 | 6.42 |
| rules 0.7, for reference | 23.8 %, 2.06 | 19.5 %, 2.56 | 5.84 |

What the numbers say:

- **Defence wins again.** The cautious mirror draws 76.6 % of its matches and scores 0.27 goals per match, where rules 0.7 gave 23.8 % and 2.06; between second-generation coaches, 94 to 96 % of matches end level.
- **The eight-square reach closes the game most.** With the old four-square reach the cautious mirror draws 40.9 % and scores 0.91 goals: dodging out of a crowd gets much harder, and every pass draws more interceptions.
- **Stance comes second, because it costs nothing when it is taken.** Careful coaches brace five goblins per team turn: a goblin done for the turn has no use for its steps left, so paying all of them is free. Without Stance the cautious mirror draws 53.9 %.
- **The 7-square den comes third** (60.5 % draws with the 11-square one), as intended: no Sprinter scores two turns after a kick-off any more. Stand-offs and the step an action costs close the game a little more each; the d8 bounce changes nothing measurable, and the carrier's −1 opens it slightly.
- **Knock-outs still pile up in bold matches** (8.2 per match), though a knocked-out goblin may now come back at the next kick-off; with few goals, that is often only half-time.
- **The home edge is the first generation's.** The bold mirror shows +11.4 points for home; the three second-generation mirrors stay within chance (−0.4, −0.3, −0.3).

## Rules 0.9, 18 Oct 2026

Rules 0.8 with chirurgical, brutal and projection assaults, chaos throws and focus. The first-generation coaches never use a style, so they play exactly as under 0.8; the second generation sacks and frees chirurgically, finishes a goblin brutally and throws into chaos when no pass is worth it. Seeds 1 to 1,000.

| Home vs away | Home wins | Away wins | Draws | Goals per match | Stances per team turn | Knock-outs per match |
|---|---|---|---|---|---|---|
| cautious-2 vs cautious-2 | 6.0 % | 6.1 % | 87.9 % | 0.13 | 5.06 | 0.58 |
| balanced-2 vs balanced-2 | 11.8 % | 10.4 % | 77.8 % | 0.25 | 4.44 | 1.25 |
| bold-2 vs bold-2 | 15.9 % | 20.0 % | 64.1 % | 0.45 | 3.05 | 3.11 |
| cautious-2 vs bold-2 | 12.2 % | 13.5 % | 74.3 % | 0.30 | 3.94 | 1.96 |
| bold-2 vs cautious-2 | 14.9 % | 11.7 % | 73.4 % | 0.29 | 3.97 | 1.86 |
| cautious-2 vs cautious | 19.2 % | 3.7 % | 77.1 % | 0.26 | 5.25 | 1.22 |
| cautious vs cautious-2 | 4.5 % | 13.5 % | 82.0 % | 0.20 | 4.92 | 1.42 |

What the numbers say:

- **The styles open the game a little.** The cautious-2 mirror scores three times as often as under 0.8 (0.13 goals per match) and draws 87.9 % of its matches instead of 95.8 %; the bold-2 mirror draws 64.1 % instead of 76.7 %. A chirurgical sack knocks the carrier down 62.5 % of the time between Bruisers, against 37.5 % for a simple one.
- **Still far from the open game of 0.7**, for the reasons measured above: the eight-square reach and a Stance free at the end of a goblin's turn.
- **The second generation beats the first** over both seats, 16.4 % of its matches against 4.1 %, most of them still drawn.

## Rules 0.9 corrected, 18 Oct 2026

Rules 0.9 with its three corrections: a chaos throw ends the turn unless a teammate holds the ball once its three bounces are over, focus belongs to the goblin holding the ball, and that goblin no longer braces in Stance. No coach ever focuses, so for them the corrections come down to two: the second generation throws into chaos only when the chance that a teammate keeps the ball clears its appetite for passes, and no carrier braces any more. The first generation never uses a style, so it only loses the carrier's Stance. Seeds 1 to 1,000.

| Home vs away | Home wins | Away wins | Draws | Goals per match | Stances per team turn | Knock-outs per match |
|---|---|---|---|---|---|---|
| cautious vs cautious | 16.7 % | 12.4 % | 70.9 % | 0.34 | 4.93 | 2.96 |
| bold vs bold | 35.0 % | 26.5 % | 38.5 % | 1.10 | 1.57 | 8.44 |
| cautious-2 vs cautious-2 | 5.0 % | 6.0 % | 89.0 % | 0.12 | 4.91 | 0.56 |
| balanced-2 vs balanced-2 | 13.4 % | 14.1 % | 72.5 % | 0.32 | 4.18 | 1.32 |
| bold-2 vs bold-2 | 22.6 % | 22.7 % | 54.7 % | 0.58 | 2.70 | 3.23 |
| cautious-2 vs bold-2 | 12.3 % | 17.3 % | 70.4 % | 0.34 | 3.69 | 2.03 |
| bold-2 vs cautious-2 | 18.2 % | 10.7 % | 71.1 % | 0.32 | 3.73 | 1.92 |
| cautious-2 vs cautious | 20.4 % | 4.9 % | 74.7 % | 0.28 | 5.11 | 1.40 |
| cautious vs cautious-2 | 5.1 % | 11.4 % | 83.5 % | 0.18 | 4.86 | 1.66 |

Each correction on its own, second-generation mirrors, same seeds (draws, goals per match):

| Mirror | 0.9 as first shipped | chaos corrected, carriers still bracing | 0.9 corrected |
|---|---|---|---|
| cautious-2 | 87.9 %, 0.13 | 91.2 %, 0.09 | 89.0 %, 0.12 |
| balanced-2 | 77.8 %, 0.25 | 80.8 %, 0.20 | 72.5 %, 0.32 |
| bold-2 | 64.1 %, 0.45 | 65.9 %, 0.42 | 54.7 %, 0.58 |

What the numbers say:

- **A carrier that cannot brace opens the game.** The first generation shows it alone: the cautious mirror draws 70.9 % of its matches instead of 76.6 % under 0.8 and scores 0.34 goals instead of 0.27, the bold mirror 1.10 instead of 0.97. Between second-generation coaches it moves the balanced-2 mirror from 80.8 % draws to 72.5 % and the bold-2 mirror from 65.9 % to 54.7 %. The carrier still defends at −1, now with no Stance to make up for it.
- **The corrected chaos throw closes it a little.** It used to be a free way to move the ball whenever nobody could intercept; it is now a gamble the coaches take only when teammates stand around the receiver, and every mirror scores a little less for it (0.13 to 0.09 goals for cautious-2, 0.45 to 0.42 for bold-2).
- **Together, the balanced-2 and bold-2 mirrors open up** compared with 0.9 as first shipped (72.5 % and 54.7 % draws instead of 77.8 % and 64.1 %), while the cautious-2 mirror stays where it was (89.0 %). All remain far from the open game of 0.7.
- **The second generation still beats the first**, 15.9 % of its matches over both seats against 5.0 %. The second-generation mirrors show no home edge (−1.0, −0.7 and −0.1 points); the first generation keeps its own (+4.3 points in the cautious mirror, +8.5 in the bold one).

## Rules 0.10, 18 Oct 2026

Rules 0.9 corrected without Stance, and with focused throws: a Thrower may sacrifice every step it has left after its throw for +1 each to the roll, in place of the whole-turn focus. The second generation weighs a focused pass against the plain one; the first generation never focuses. Seeds 1 to 1,000, on the tiny circle of the rules file unless said otherwise.

| Home vs away | Home wins | Away wins | Draws | Goals per match | Assaults per match | Knock-outs per match |
|---|---|---|---|---|---|---|
| cautious vs cautious | 25.2 % | 20.9 % | 53.9 % | 0.61 | 48.4 | 8.27 |
| bold vs bold | 38.5 % | 27.9 % | 33.6 % | 1.07 | 64.8 | 10.14 |
| cautious-2 vs cautious-2 | 10.1 % | 8.5 % | 81.4 % | 0.20 | 62.4 | 1.55 |
| balanced-2 vs balanced-2 | 11.6 % | 10.3 % | 78.1 % | 0.24 | 66.7 | 1.80 |
| bold-2 vs bold-2 | 26.5 % | 22.8 % | 50.7 % | 0.66 | 82.5 | 4.54 |
| cautious-2 vs bold-2 | 12.3 % | 18.2 % | 69.5 % | 0.38 | 75.7 | 2.96 |
| bold-2 vs cautious-2 | 22.5 % | 11.7 % | 65.8 % | 0.40 | 75.3 | 3.01 |
| cautious-2 vs cautious | 28.1 % | 12.2 % | 59.7 % | 0.49 | 52.2 | 5.14 |
| cautious vs cautious-2 | 13.7 % | 17.3 % | 69.0 % | 0.34 | 52.3 | 5.55 |
| cautious-2 vs cautious-2, medium circle | 8.1 % | 6.9 % | 85.0 % | 0.16 | 62.4 | 1.54 |
| balanced-2 vs balanced-2, medium circle | 9.4 % | 8.3 % | 82.3 % | 0.19 | 67.1 | 1.77 |
| bold-2 vs bold-2, medium circle | 20.3 % | 20.9 % | 58.8 % | 0.52 | 83.3 | 4.57 |

Second-generation mirrors, same seeds (draws, goals per match):

| Mirror | 0.9 corrected | 0.10 without focused throws | 0.10 |
|---|---|---|---|
| cautious-2 | 89.0 %, 0.12 | 82.6 %, 0.19 | 81.4 %, 0.20 |
| balanced-2 | 72.5 %, 0.32 | 77.0 %, 0.25 | 78.1 %, 0.24 |
| bold-2 | 54.7 %, 0.58 | 51.1 %, 0.67 | 50.7 %, 0.66 |

What the numbers say:

- **Without Stance, the cautious coaches open up most.** The first-generation cautious mirror draws 53.9 % of its matches instead of 70.9 % and scores 0.61 goals instead of 0.34; the cautious-2 mirror draws 81.4 % instead of 89.0 %. The bold-2 mirror opens a little (50.7 % draws), while the balanced-2 mirror closes a little (78.1 % instead of 72.5 %).
- **The coaches fight instead of bracing.** Assaults per match rise from 42 to 67 in the balanced-2 mirror and from 26 to 62 in the cautious-2 one, and knock-outs follow (8.27 per match in the first-generation cautious mirror, 2.96 before).
- **Focused throws change little between bots,** whose Throwers pass about twice a match: with them switched off every mirror lands within about a point, and throws are caught one to three points less often. The bots never set up long passes, so this says little about what a coach who plays around the Thrower gets from them.
- **The medium circle**, where the latest recorded match was played, is a little more closed than the tiny one: 85.0 % draws in the cautious-2 mirror, 82.3 % for balanced-2, 58.8 % for bold-2.
- **The second generation still beats the first**, 22.7 % of its matches over both seats against 13.0 %, a narrower gap than under 0.9. Home edges stay small in the second-generation mirrors (+1.6, +1.3 and +3.7 points) and remain in the first generation's (+4.3 cautious, +10.6 bold).

## Rules 0.11, 20 Oct 2026

Rules 0.10 with two changes to make the game more mobile: an assault or a throw no longer costs a step, so a goblin may still act after its last step (`actions_cost_a_step = false`), and every agility dodge gets +1 (`dodge_bonus = 1`), so leaving one enemy is as easy as a ball roll with nobody around: a Sprinter gets away from 1, 2 or 3 enemies 83, 67 or 50 % of the time instead of 67, 50 or 33 %, a Bruiser from 1 or 2 enemies 67 or 33 % instead of 33 or 0 %. Hand-offs keep their catch and interceptions the eight squares around the flight line. Second-generation coaches, seeds 1 to 1,000, on the tiny circle of the rules file; the 0.10 figures quoted below were played again on `assessment/rules-0.10.toml` with today's bots and match the 0.10 section to the decimal.

| Home vs away | Home wins | Away wins | Draws | Goals per match | Dodges per match (made) | Throws per match | Knock-outs per match |
|---|---|---|---|---|---|---|---|
| cautious-2 vs cautious-2 | 18.0 % | 14.3 % | 67.7 % | 0.38 | 10.2 (80 %) | 2.9 | 1.85 |
| balanced-2 vs balanced-2 | 18.9 % | 16.4 % | 64.7 % | 0.43 | 12.3 (80 %) | 3.0 | 2.00 |
| bold-2 vs bold-2 | 29.2 % | 29.4 % | 41.4 % | 0.87 | 26.2 (79 %) | 4.3 | 4.43 |
| cautious-2 vs bold-2 | 21.9 % | 27.0 % | 51.1 % | 0.68 | 18.3 (79 %) | 3.6 | 3.11 |
| bold-2 vs cautious-2 | 28.9 % | 20.4 % | 50.7 % | 0.65 | 18.1 (79 %) | 3.6 | 3.17 |

Each change on its own in the mirrors, same seeds (draws, goals per match):

| Mirror | 0.10 | 0.10 with free actions | 0.10 with the dodge bonus | 0.11 |
|---|---|---|---|---|
| cautious-2 | 81.4 %, 0.20 | 81.9 %, 0.19 | 67.5 %, 0.39 | 67.7 %, 0.38 |
| balanced-2 | 78.1 %, 0.24 | 79.9 %, 0.21 | 62.1 %, 0.47 | 64.7 %, 0.43 |
| bold-2 | 50.7 %, 0.66 | 44.5 %, 0.77 | 44.4 %, 0.74 | 41.4 %, 0.87 |

What the numbers say:

- **The game opens in every pairing.** Draws fall by 9 to 18 points and goals per match rise by a third to nearly double, a bigger step than 0.10 took in any second-generation mirror; the mixed pairings move most, from 69.5 % and 65.8 % draws to about 51 %.
- **The dodge bonus does most of it.** Coaches dodge two to four times as often (2.4 to 10.2 per match in the cautious-2 mirror, 12.4 to 26.2 in the bold-2 one) and make about 80 % of their dodges instead of 62 to 66 %. With the bonus alone the careful mirrors open as far as under the whole of 0.11, the balanced-2 one a little further.
- **Free actions help the bold coaches.** On their own they open only the bold-2 mirror (44.5 % draws), whose goblins now run their whole Move and still assault or throw; careful coaches gain nothing from them, and the balanced-2 mirror even closes a little, probably because defenders too can now run their whole Move and still hit the carrier. Throws rise with them (2.6 to 3.8 per match in the bold-2 mirror), dodges do not.
- **Fights stay as they were**: assaults per match within 2.5 of 0.10 and knock-outs within 0.3, so the extra goals come from goblins slipping out of the scrum, not from bloodier matches.
- **Fewer draws pay both appetites, the cautious one a little more**: over both seats, cautious-2 wins 21.2 % of its matches against bold-2 (12.0 % under 0.10) and bold-2 28.0 % (20.4 %). Home edges stay within 4 points.
- **The bots still barely pass**: three to four throws per match, and still no interception. They go forward and fight; none keeps its receivers out of the scrum or cages its carrier, so these tables say what 0.11 does for that play, not for a positional passing game, which only matches between people, or a coach built for it, will show.

## Rules 0.12, 20 Oct 2026

Rules 0.11 where each archetype has its own ways: only Throwers throw into chaos, and a focused throw needs a Thrower that has not moved, which spends its whole Move for +3 and falls, unhurt, when its throw roll misses; only Bruisers assault brutally or project, a projection losing its −1 to win and taking one Health less; only Sprinters intercept from a square next to the flight line, and Sprinters dive at −1 for a ball landing on an empty square around them, the highest roll diving. Assaults reach the eight squares around, an assault on a downed goblin is an aggression, +1 to attack counted in the Health it takes and the only way to hit it, and a hand-off's receiver catches at +1. Second-generation coaches, seeds 1 to 1,000, on the tiny circle of the rules file. The 0.11 figures quoted here were played again after the odds of focused throws were corrected the same day (a bonus above the target of 2 had lost its excess in the previews the bots weigh, two of a standing Thrower's four points under 0.11), which moves them by a point at most from the 0.11 section.

| Home vs away | Home wins | Away wins | Draws | Goals per match | Assaults per match | Throws per match | Dives per match (caught) | Knock-outs per match |
|---|---|---|---|---|---|---|---|---|
| cautious-2 vs cautious-2 | 26.1 % | 24.2 % | 49.7 % | 0.72 | 96.5 | 4.9 | 11.5 (60 %) | 5.99 |
| balanced-2 vs balanced-2 | 27.6 % | 27.5 % | 44.9 % | 0.78 | 102.1 | 5.3 | 12.1 (59 %) | 5.87 |
| bold-2 vs bold-2 | 33.8 % | 32.2 % | 34.0 % | 1.18 | 118.4 | 7.4 | 14.0 (60 %) | 6.26 |
| cautious-2 vs bold-2 | 25.5 % | 35.8 % | 38.7 % | 0.95 | 109.3 | 6.1 | 13.2 (60 %) | 6.11 |
| bold-2 vs cautious-2 | 35.1 % | 25.1 % | 39.8 % | 0.94 | 109.3 | 6.0 | 13.4 (60 %) | 6.09 |

From 0.11 to 0.12, same seeds (draws, goals per match, knock-outs per match):

| Home vs away | 0.11 | 0.12 |
|---|---|---|
| cautious-2 vs cautious-2 | 66.7 %, 0.38, 1.89 | 49.7 %, 0.72, 5.99 |
| balanced-2 vs balanced-2 | 64.3 %, 0.43, 2.03 | 44.9 %, 0.78, 5.87 |
| bold-2 vs bold-2 | 42.0 %, 0.87, 4.43 | 34.0 %, 1.18, 6.26 |
| cautious-2 vs bold-2 | 50.9 %, 0.68, 3.11 | 38.7 %, 0.95, 6.11 |
| bold-2 vs cautious-2 | 51.1 %, 0.64, 3.21 | 39.8 %, 0.94, 6.09 |

Each change undone on its own in the mirrors, same seeds (draws, goals per match, knock-outs per match):

| 0.12 | cautious-2 | balanced-2 | bold-2 |
|---|---|---|---|
| as shipped | 49.7 %, 0.72, 5.99 | 44.9 %, 0.78, 5.87 | 34.0 %, 1.18, 6.26 |
| without diving catches | 61.2 %, 0.48, 6.36 | 57.4 %, 0.53, 6.35 | 38.1 %, 0.97, 6.87 |
| every enemy intercepting beside the line | 50.2 %, 0.64, 5.90 | 46.4 %, 0.75, 5.83 | 35.4 %, 1.12, 6.18 |
| orthogonal assaults | 47.0 %, 0.72, 2.08 | 44.0 %, 0.79, 1.99 | 36.7 %, 1.15, 2.67 |
| brutal and projection for everyone, projection as in 0.11 | 49.8 %, 0.68, 2.75 | 46.1 %, 0.71, 2.98 | 35.1 %, 1.12, 4.31 |
| no aggressions, the downed malus back at 2 | 47.6 %, 0.68, 4.49 | 46.7 %, 0.72, 4.59 | 38.7 %, 1.09, 5.04 |
| the Throwers' throws of 0.11: chaos for anyone, focus +1 a step left, no fall | 45.4 %, 0.77, 6.03 | 41.7 %, 0.84, 6.05 | 34.3 %, 1.23, 6.31 |
| no hand-off bonus | 49.7 %, 0.72, 5.99 | 44.9 %, 0.78, 5.87 | 34.0 %, 1.18, 6.26 |

What the numbers say:

- **The game opens again, and gets bloodier.** Draws fall by 8 to 19 points from 0.11 and goals per match rise by a third to nearly double; knock-outs triple in the careful mirrors (1.9 to 6.0 a match) and assaults go up by half.
- **Diving catches do most of the opening.** Sprinters dive 11 to 14 times a match and catch 60 % of those balls; without dives the careful mirrors fall back to 57 to 61 % draws. A chaos throw towards a lone Sprinter with nobody near is now kept two times in three.
- **Passes rise too**, from 3 to 5 a match in the careful mirrors and from 4 to 7 in the bold one (chaos throws not counted), dives and clearer flight lines both playing a part: without dives the cautious mirror throws 3.8 times a match, with every enemy intercepting beside the line 3.9.
- **Diagonal assaults make the bloodshed, not the goals.** With orthogonal assaults the mirrors score about as much (0.72, 0.79 and 1.15 goals) with 76 to 92 assaults a match instead of 97 to 118, and 2.0 to 2.7 knock-outs instead of 5.9 to 6.3.
- **Keeping brutal hits for Bruisers adds knock-outs through the bots' play.** Their other goblins hit chirurgically, +1 to win, where they would hit brutally at −1, so they take more hits and knock down more goblins, which aggressions then finish, most likely; with brutal hits and projections open to all, knock-outs fall to 2.8 to 4.3 a match and assaults by a fifth in the careful mirrors, while goals barely move.
- **Keeping chaos and focus for Throwers closes the careful mirrors a little**: with the Throwers' throws of 0.11 they draw 45.4 % and 41.7 % instead of 49.7 % and 44.9 %, with a few more goals, likely from chaos thrown by any goblin towards a diving Sprinter; the bold mirror does not move.
- **Aggressions open the bold mirror** (38.7 % to 34.0 % draws) and add a knock-out or so a match in each mirror; the careful mirrors barely move.
- **Interceptions kept beside the line for Sprinters open a little**: 0.5 to 1.5 points fewer draws, a few more goals, and the bots pass more (4.9 throws a match in the cautious mirror against 3.9 when every enemy intercepts beside the line).
- **The hand-off bonus changes nothing between bots**, to the decimal: their passes look for an open teammate three squares closer to the den, never an adjacent one, so they never hand off. It matters only to coaches who play hand-offs, like the passing game this release is for.
- **The bold appetite pays more than before**: over both seats bold-2 wins 35.5 % of its matches against cautious-2 (27.9 % under 0.11) and cautious-2 25.3 % (21.1 %). Home edges stay within 2 points.
- **A cost of the bots, not of the game**: matches between bots take about five times longer to compute under 0.12, the previews they weigh for every candidate chaos throw now counting every dive; a single preview, as a coach or an agent asks for one, still takes milliseconds.
