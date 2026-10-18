# Balance runs (M4), 10 Oct 2026

How rules 0.1 play out between scripted coaches, measured with `make balance`: 1,000 seeded matches per line (seeds 1 to 1,000), the same seeds for every line.

```bash
make balance ARGS="--games 1000 --home cautious --away bold --rules rules.toml"
make balance ARGS="--games 1000 --home cautious-2 --away bold-2 --arena rectangle --arena-size medium"
```

## The coaches

- **random** plays legal commands at random: the sparring partner of the property tests.
- **cautious** and **bold** are the same scripted coach after the Bot Bowl recipe, with two risk appetites. In order of priority they score, assault the enemy carrier, pick up the loose ball, hit the carrier's markers, run Sprinters ahead into open space, pass along a clear line, advance the carrier and brace it, guard it without standing in its lane, hit, close in on the ball without dice, and brace any goblin in contact. A risk is taken only above the coach's appetite:

| Lowest chance played for | Score | Sack the carrier | Pick up | Pass | Free the carrier, advance, hit |
|---|---|---|---|---|---|
| cautious | 70 % | 50 % | 50 % | 50 % | 75 % |
| bold | 50 % | 33 % | 33 % | 33 % | 50 % |

- **cautious-2**, **balanced-2** and **bold-2**, since 17 Oct 2026: the second generation, beside the first, see [the second-generation coaches](#second-generation-coaches-17-oct-2026).

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
- **Knocked-out goblins come back**, so knock-outs no longer snowball: the bold mirror loses 8.2 goblins per match but plays most of its second half in numbers.
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
