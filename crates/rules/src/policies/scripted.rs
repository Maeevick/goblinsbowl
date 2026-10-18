//! Scripted coaches after the Bot Bowl recipe: score when the odds are good, free the ball, then
//! move without dice and brace, taking a risk only within the coach's appetite, and ending the turn
//! otherwise. The cautious and bold coaches differ only in their appetite.

use std::cmp::Reverse;

use crate::arena::Arena;
use crate::characteristics::Archetype;
use crate::command::{Action, Activation, AssaultStyle, Command, ThrowStyle};
use crate::odds::Probability;
use crate::paths::{PlannedPath, best_chance_for, reachable_squares};
use crate::preview::{ActionOdds, activation_odds, board_after_success};
use crate::roster::GoblinId;
use crate::ruleset::Ruleset;
use crate::square::Square;
use crate::state::{Ball, GameState, Goblin};
use crate::team::TeamId;

/// The lowest chance a scripted coach plays for, kind of risk by kind of risk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Appetite {
    /// Walking the carrier into the enemy den.
    pub score: Probability,
    /// Assaulting the enemy carrier.
    pub sack: Probability,
    /// Picking up the loose ball.
    pub pick: Probability,
    /// Assaulting a goblin that marks the own carrier.
    pub free: Probability,
    /// Throwing to an open teammate closer to the den.
    pub pass: Probability,
    /// Moving the carrier towards the den when no move without dice gets closer.
    pub advance: Probability,
    /// Any other assault.
    pub hit: Probability,
}

impl Appetite {
    /// Scores at 70 %, assaults the carrier, picks up or passes at 50 %, frees, advances or hits at 75 %.
    pub fn cautious() -> Self {
        Self {
            score: Probability::new(7, 10),
            sack: Probability::new(1, 2),
            pick: Probability::new(1, 2),
            free: Probability::new(3, 4),
            pass: Probability::new(1, 2),
            advance: Probability::new(3, 4),
            hit: Probability::new(3, 4),
        }
    }

    /// Scores, frees, advances or hits at 50 %, assaults the carrier, picks up or passes at a third.
    pub fn bold() -> Self {
        Self {
            score: Probability::new(1, 2),
            sack: Probability::new(1, 3),
            pick: Probability::new(1, 3),
            free: Probability::new(1, 2),
            pass: Probability::new(1, 3),
            advance: Probability::new(1, 2),
            hit: Probability::new(1, 2),
        }
    }

    fn lowest_chance(&self, risk: Risk) -> Probability {
        match risk {
            Risk::Score => self.score,
            Risk::Sack => self.sack,
            Risk::Pick => self.pick,
            Risk::Free => self.free,
            Risk::Pass => self.pass,
            Risk::Advance => self.advance,
            Risk::Hit => self.hit,
        }
    }
}

/// A kind of risk a scripted coach weighs, one per line of its appetite.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Risk {
    Score,
    Sack,
    Pick,
    Free,
    Pass,
    Advance,
    Hit,
}

/// Which scripted coach plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Generation {
    /// The coaches of the first batches: orders by priority alone, ties broken by square order.
    First,
    /// Ties broken the same way for both teams, the square nearest its own den first, then the
    /// northern one; and before a gamble, another goblin's certain move when it leaves the gamble
    /// its chance.
    Second,
}

/// How much a scripted coach risks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Temper {
    /// The same lowest chance per kind of risk, whatever the turn.
    Fixed(Appetite),
    /// The careful appetite while every other goblin can still play, sliding to the daring one as
    /// fewer are left: a failure ends the turn, so it costs more early in the turn.
    Exposed {
        /// The appetite when every other goblin can still play.
        careful: Appetite,
        /// The appetite when no other goblin can play.
        daring: Appetite,
    },
}

/// The chance `weight` parts of `scale` of the way from `low` to `high`, as an exact fraction.
fn between(low: Probability, high: Probability, weight: u64, scale: u64) -> Probability {
    let scale = scale.max(1);
    let weight = weight.min(scale);
    let numerator =
        u128::from(low.numerator()) * u128::from(high.denominator()) * u128::from(scale - weight)
            + u128::from(high.numerator()) * u128::from(low.denominator()) * u128::from(weight);
    let denominator =
        u128::from(low.denominator()) * u128::from(high.denominator()) * u128::from(scale);
    Probability::new(
        u64::try_from(numerator).unwrap_or(u64::MAX),
        u64::try_from(denominator).unwrap_or(u64::MAX),
    )
}

/// A scripted coach: its generation and its temper.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Style {
    /// Which coach plays.
    pub generation: Generation,
    /// How much it risks.
    pub temper: Temper,
}

impl Style {
    /// The second-generation coach with a fixed `appetite`.
    pub fn second(appetite: Appetite) -> Self {
        Self {
            generation: Generation::Second,
            temper: Temper::Fixed(appetite),
        }
    }

    /// The second-generation coach as cautious as the cautious one while every other goblin can
    /// still play, and as bold as the bold one once none can.
    pub fn balanced() -> Self {
        Self {
            generation: Generation::Second,
            temper: Temper::Exposed {
                careful: Appetite::cautious(),
                daring: Appetite::bold(),
            },
        }
    }
}

/// The next command of a careful coach for `team`: [`scripted_command`] with a cautious appetite.
pub fn cautious_command(
    arena: &Arena,
    ruleset: &Ruleset,
    state: &GameState,
    team: TeamId,
) -> Command {
    scripted_command(arena, ruleset, state, team, Appetite::cautious())
}

/// The next command of a daring coach for `team`: [`scripted_command`] with a bold appetite.
pub fn bold_command(arena: &Arena, ruleset: &Ruleset, state: &GameState, team: TeamId) -> Command {
    scripted_command(arena, ruleset, state, team, Appetite::bold())
}

/// The next command of a first-generation scripted coach for `team` with this `appetite`.
pub fn scripted_command(
    arena: &Arena,
    ruleset: &Ruleset,
    state: &GameState,
    team: TeamId,
    appetite: Appetite,
) -> Command {
    styled_command(
        arena,
        ruleset,
        state,
        team,
        Style {
            generation: Generation::First,
            temper: Temper::Fixed(appetite),
        },
    )
}

/// The next command of the scripted coach of this `style` for `team`, without any randomness.
///
/// In order of priority: score, assault the enemy carrier, pick up the loose ball, hit the carrier's
/// markers, run Sprinters ahead into open space while the carrier still has to act, throw to an
/// open teammate well ahead along a clear line, advance the carrier (without dice if that gets
/// closer, else within the appetite) and brace it, guard the carrier without standing in front of
/// it, hit an adjacent enemy, close in on the ball without dice, brace any goblin in contact;
/// otherwise end the turn. Every risk is taken only within the temper, and nobody braces or
/// guards under rules without Stance.
pub fn styled_command(
    arena: &Arena,
    ruleset: &Ruleset,
    state: &GameState,
    team: TeamId,
    style: Style,
) -> Command {
    let coach = Coach {
        arena,
        ruleset,
        state,
        team,
        style,
        excluded: None,
    };
    let chosen = coach.by_priority();
    let chosen = match (style.generation, chosen) {
        (Generation::Second, Some(gamble)) => Some(coach.certain_before(&gamble).unwrap_or(gamble)),
        (_, chosen) => chosen,
    };
    chosen.map_or(Command::EndTurn, Command::Act)
}

#[derive(Clone, Copy)]
struct Coach<'a> {
    arena: &'a Arena,
    ruleset: &'a Ruleset,
    state: &'a GameState,
    team: TeamId,
    style: Style,
    excluded: Option<GoblinId>,
}

impl Coach<'_> {
    fn by_priority(&self) -> Option<Activation> {
        self.score()
            .or_else(|| self.sack())
            .or_else(|| self.pick())
            .or_else(|| self.free_carrier())
            .or_else(|| self.run_ahead())
            .or_else(|| self.pass())
            .or_else(|| self.advance_carrier())
            .or_else(|| self.guard_carrier())
            .or_else(|| self.hit())
            .or_else(|| self.close_in())
            .or_else(|| self.brace())
    }

    fn certain_before(&self, gamble: &Activation) -> Option<Activation> {
        let chance = self.success(gamble)?;
        if chance == Probability::CERTAIN {
            return None;
        }
        let others = Coach {
            excluded: Some(gamble.goblin),
            ..*self
        };
        let certain_moves: [&dyn Fn() -> Option<Activation>; 3] = [
            &|| others.run_ahead(),
            &|| others.advance_carrier(),
            &|| others.close_in(),
        ];
        certain_moves
            .into_iter()
            .filter_map(|move_of| move_of())
            .filter(|order| self.success(order) == Some(Probability::CERTAIN))
            .find(|order| self.leaves_its_chance(gamble, chance, order))
    }

    fn leaves_its_chance(
        &self,
        gamble: &Activation,
        chance: Probability,
        first: &Activation,
    ) -> bool {
        let Some(destination) = self.destination(gamble) else {
            return false;
        };
        board_after_success(self.arena, self.ruleset, self.state, self.team, first)
            .ok()
            .and_then(|board| {
                best_chance_for(
                    self.arena,
                    self.ruleset,
                    &board,
                    self.team,
                    gamble.goblin,
                    destination,
                    gamble.action,
                )
            })
            .is_some_and(|best| best >= chance)
    }

    fn lowest_chance(&self, risk: Risk, mover: GoblinId) -> Probability {
        match self.style.temper {
            Temper::Fixed(appetite) => appetite.lowest_chance(risk),
            Temper::Exposed { careful, daring } => {
                let others_left = self
                    .state
                    .goblins_of(self.team)
                    .filter(|goblin| goblin.id != mover && goblin.can_act())
                    .count();
                between(
                    daring.lowest_chance(risk),
                    careful.lowest_chance(risk),
                    u64::try_from(others_left).unwrap_or(u64::MAX),
                    u64::from(self.ruleset.team_size.saturating_sub(1)),
                )
            }
        }
    }

    fn tie_break(&self, square: Square) -> (u8, u8) {
        match self.style.generation {
            Generation::First => (0, 0),
            Generation::Second => {
                let from_own_den = if self.team == TeamId::HOME {
                    square.x
                } else {
                    self.arena
                        .width()
                        .saturating_sub(1)
                        .saturating_sub(square.x)
                };
                (from_own_den, square.y)
            }
        }
    }

    fn destination(&self, activation: &Activation) -> Option<Square> {
        activation.path.last().copied().or_else(|| {
            self.state
                .goblin(activation.goblin)
                .and_then(Goblin::square)
        })
    }

    fn movers(&self) -> impl Iterator<Item = &Goblin> + '_ {
        self.state
            .goblins_of(self.team)
            .filter(|goblin| goblin.can_act() && Some(goblin.id) != self.excluded)
    }

    fn actors(&self) -> impl Iterator<Item = &Goblin> + '_ {
        self.movers().filter(|goblin| goblin.has_action_left)
    }

    fn paths(&self, goblin: GoblinId) -> Vec<PlannedPath> {
        reachable_squares(self.arena, self.ruleset, self.state, goblin)
    }

    fn success(&self, activation: &Activation) -> Option<Probability> {
        activation_odds(self.arena, self.ruleset, self.state, self.team, activation)
            .ok()
            .map(|odds| odds.overall_success)
    }

    /// The chance `activation` does what it is played for: for the second generation, an assault's
    /// chance to knock its target down, a stand-off doing nothing; otherwise the chance the turn
    /// goes on, which is the same where ties knock the target down.
    fn chance_played_for(&self, activation: &Activation) -> Option<Probability> {
        let odds =
            activation_odds(self.arena, self.ruleset, self.state, self.team, activation).ok()?;
        Some(match (self.style.generation, &odds.action) {
            (Generation::Second, Some(ActionOdds::Assault { knock_down, .. })) => {
                odds.path_success.and(*knock_down)
            }
            _ => odds.overall_success,
        })
    }

    fn own_carrier(&self) -> Option<&Goblin> {
        self.state
            .carrier()
            .and_then(|carrier| self.state.goblin(carrier))
            .filter(|carrier| carrier.team() == self.team)
    }

    fn enemy_carrier(&self) -> Option<&Goblin> {
        self.state
            .carrier()
            .and_then(|carrier| self.state.goblin(carrier))
            .filter(|carrier| carrier.team() != self.team)
    }

    fn den_distance(&self, square: Square) -> u8 {
        self.arena
            .in_bounds_squares()
            .filter(|den| self.arena.is_scoring_square_for(self.team, *den))
            .map(|den| den.orthogonal_distance(square))
            .min()
            .unwrap_or(u8::MAX)
    }

    fn enemies_next_to(&self, square: Square) -> usize {
        self.state
            .standing_opponents_in_reach(square, self.team, self.ruleset.reach())
            .count()
    }

    fn most_likely(
        &self,
        candidates: impl IntoIterator<Item = (Activation, Probability)>,
        risk: Risk,
    ) -> Option<Activation> {
        candidates
            .into_iter()
            .filter(|(activation, success)| *success >= self.lowest_chance(risk, activation.goblin))
            .min_by_key(|(activation, success)| {
                (
                    Reverse(*success),
                    activation.path.len(),
                    self.destination(activation)
                        .map(|square| self.tie_break(square)),
                )
            })
            .map(|(activation, _)| activation)
    }

    fn score(&self) -> Option<Activation> {
        let carrier = self.own_carrier().filter(|carrier| carrier.can_act())?;
        let candidates = self
            .paths(carrier.id)
            .into_iter()
            .filter(|path| {
                self.arena
                    .is_scoring_square_for(self.team, path.destination)
            })
            .map(|path| {
                let walk = Activation {
                    goblin: carrier.id,
                    path: path.squares,
                    action: None,
                };
                (walk, path.success)
            });
        self.most_likely(candidates, Risk::Score)
    }

    fn sack(&self) -> Option<Activation> {
        let carrier = self.enemy_carrier()?;
        let target = carrier.square()?;
        let assault = Some(Action::Assault {
            target: carrier.id,
            style: self.style_to_knock_down(),
        });
        let candidates: Vec<_> = self
            .actors()
            .flat_map(|mover| {
                let in_contact = mover
                    .square()
                    .is_some_and(|square| square.is_orthogonally_adjacent(target));
                let stay = in_contact.then(Vec::new);
                let walks = self
                    .paths(mover.id)
                    .into_iter()
                    .filter(|path| path.destination.is_orthogonally_adjacent(target))
                    .map(|path| path.squares);
                stay.into_iter().chain(walks).map(move |path| Activation {
                    goblin: mover.id,
                    path,
                    action: assault,
                })
            })
            .filter_map(|activation| {
                self.chance_played_for(&activation)
                    .map(|chance| (activation, chance))
            })
            .collect();
        self.most_likely(candidates, Risk::Sack)
    }

    fn pick(&self) -> Option<Activation> {
        let Ball::Loose { at } = self.state.ball else {
            return None;
        };
        let candidates: Vec<_> = self
            .movers()
            .filter_map(|mover| {
                self.paths(mover.id)
                    .into_iter()
                    .find(|path| path.destination == at)
                    .map(|path| {
                        let walk = Activation {
                            goblin: mover.id,
                            path: path.squares,
                            action: None,
                        };
                        (walk, path.success)
                    })
            })
            .collect();
        self.most_likely(candidates, Risk::Pick)
    }

    fn free_carrier(&self) -> Option<Activation> {
        let carrier = self.own_carrier()?;
        let carrier_square = carrier.square()?;
        let markers: Vec<_> = self
            .state
            .standing_opponents_in_reach(carrier_square, self.team, self.ruleset.reach())
            .filter_map(|marker| marker.square().map(|square| (marker.id, square)))
            .collect();
        let candidates: Vec<_> = self
            .actors()
            .filter(|mover| mover.id != carrier.id)
            .flat_map(|mover| {
                let here = mover.square();
                let walks = self.paths(mover.id);
                markers.iter().flat_map(move |(marker, marker_square)| {
                    let stay = here
                        .is_some_and(|square| square.is_orthogonally_adjacent(*marker_square))
                        .then(Vec::new);
                    let certain_walks = walks
                        .iter()
                        .filter(|path| {
                            path.success == Probability::CERTAIN
                                && path.destination.is_orthogonally_adjacent(*marker_square)
                        })
                        .map(|path| path.squares.clone())
                        .collect::<Vec<_>>();
                    stay.into_iter()
                        .chain(certain_walks)
                        .map(move |path| Activation {
                            goblin: mover.id,
                            path,
                            action: Some(Action::Assault {
                                target: *marker,
                                style: self.style_to_knock_down(),
                            }),
                        })
                })
            })
            .filter_map(|activation| {
                self.chance_played_for(&activation)
                    .map(|chance| (activation, chance))
            })
            .collect();
        self.most_likely(candidates, Risk::Free)
    }

    fn run_ahead(&self) -> Option<Activation> {
        let carrier = self.own_carrier().filter(|carrier| carrier.can_act())?;
        let carrier_distance = self.den_distance(carrier.square()?);
        self.movers()
            .filter(|runner| runner.id != carrier.id && runner.archetype == Archetype::Sprinter)
            .find_map(|runner| {
                let here = runner.square()?;
                let best = self
                    .paths(runner.id)
                    .into_iter()
                    .filter(|path| {
                        let distance = self.den_distance(path.destination);
                        path.success == Probability::CERTAIN
                            && self.enemies_next_to(path.destination) == 0
                            && distance < self.den_distance(here)
                            && distance < carrier_distance
                    })
                    .min_by_key(|path| {
                        (
                            self.den_distance(path.destination),
                            path.squares.len(),
                            self.tie_break(path.destination),
                        )
                    })?;
                Some(Activation {
                    goblin: runner.id,
                    path: best.squares,
                    action: None,
                })
            })
    }

    fn pass(&self) -> Option<Activation> {
        let carrier = self
            .own_carrier()
            .filter(|carrier| carrier.can_act() && carrier.has_action_left)?;
        let carrier_distance = self.den_distance(carrier.square()?);
        let candidates: Vec<_> = self
            .state
            .goblins_of(self.team)
            .filter(|receiver| receiver.id != carrier.id && receiver.is_standing())
            .filter(|receiver| {
                receiver.square().is_some_and(|square| {
                    self.enemies_next_to(square) == 0
                        && self.den_distance(square).saturating_add(3) <= carrier_distance
                })
            })
            .flat_map(|receiver| {
                self.throw_styles_of(carrier)
                    .into_iter()
                    .map(move |style| (receiver.id, style))
            })
            .filter_map(|(receiver, style)| {
                let throw = Activation {
                    goblin: carrier.id,
                    path: Vec::new(),
                    action: Some(Action::Throw { receiver, style }),
                };
                let odds = activation_odds(self.arena, self.ruleset, self.state, self.team, &throw)
                    .ok()?;
                let clear_line = matches!(
                    &odds.action,
                    Some(ActionOdds::Throw { interceptions, .. }) if interceptions.is_empty()
                );
                clear_line.then_some((throw, odds.overall_success))
            })
            .collect();
        self.most_likely(candidates, Risk::Pass)
            .or_else(|| self.chaos_pass(carrier))
    }

    /// How `carrier` may throw a pass: simply, and focused too for a second-generation coach's
    /// Thrower where the rules have focused throws, the better chance then winning.
    fn throw_styles_of(&self, carrier: &Goblin) -> Vec<ThrowStyle> {
        let focuses = self.style.generation == Generation::Second
            && self.ruleset.focused_throws_spend_steps_left
            && carrier.archetype == Archetype::Thrower;
        if focuses {
            vec![ThrowStyle::Simple, ThrowStyle::Focused]
        } else {
            vec![ThrowStyle::Simple]
        }
    }

    /// Where the rules have styles, a second-generation coach whose carrier has not moved and finds
    /// no pass worth its appetite throws into chaos towards a teammate three squares closer to the
    /// enemy den at least, the one around whom a teammate most likely ends up holding the ball, when
    /// that chance is worth its appetite for passes; otherwise nothing.
    fn chaos_pass(&self, carrier: &Goblin) -> Option<Activation> {
        if self.style.generation != Generation::Second
            || !self.ruleset.throws_have_styles
            || carrier.steps_left < carrier.characteristics.movement
        {
            return None;
        }
        let carrier_distance = self.den_distance(carrier.square()?);
        let candidates: Vec<_> = self
            .state
            .goblins_of(self.team)
            .filter(|receiver| receiver.id != carrier.id && receiver.is_standing())
            .filter(|receiver| {
                receiver.square().is_some_and(|square| {
                    self.den_distance(square).saturating_add(3) <= carrier_distance
                })
            })
            .filter_map(|receiver| {
                let chaos = Activation {
                    goblin: carrier.id,
                    path: Vec::new(),
                    action: Some(Action::Throw {
                        receiver: receiver.id,
                        style: ThrowStyle::Chaos,
                    }),
                };
                let success = self.success(&chaos)?;
                Some((chaos, success))
            })
            .collect();
        self.most_likely(candidates, Risk::Pass)
    }

    /// How a second-generation coach assaults to knock a goblin down where the rules have styles:
    /// chirurgically, +1 to win; otherwise simply.
    fn style_to_knock_down(&self) -> AssaultStyle {
        if self.style.generation == Generation::Second && self.ruleset.assaults_have_styles {
            AssaultStyle::Chirurgical
        } else {
            AssaultStyle::Simple
        }
    }

    /// How a second-generation coach hits `enemy` where the rules have styles: brutally when it lies
    /// down or has 3 Health or less, a win then knocking it out; chirurgically otherwise.
    fn style_to_hit(&self, enemy: &Goblin) -> AssaultStyle {
        let finishable = !enemy.is_standing() || enemy.health <= 3;
        match self.style_to_knock_down() {
            AssaultStyle::Chirurgical if finishable => AssaultStyle::Brutal,
            style => style,
        }
    }

    fn advance_carrier(&self) -> Option<Activation> {
        let carrier = self
            .own_carrier()
            .filter(|carrier| carrier.can_act() && Some(carrier.id) != self.excluded)?;
        let here = carrier.square()?;
        let closer: Vec<_> = self
            .paths(carrier.id)
            .into_iter()
            .filter(|path| self.den_distance(path.destination) < self.den_distance(here))
            .collect();
        let without_dice = closer
            .iter()
            .filter(|path| path.success == Probability::CERTAIN)
            .min_by_key(|path| {
                (
                    self.den_distance(path.destination),
                    self.enemies_next_to(path.destination),
                    path.squares.len(),
                    self.tie_break(path.destination),
                )
            });
        let fairly_safe = || {
            closer
                .iter()
                .filter(|path| path.success >= self.lowest_chance(Risk::Advance, carrier.id))
                .min_by_key(|path| {
                    (
                        self.den_distance(path.destination),
                        Reverse(path.success),
                        path.squares.len(),
                        self.tie_break(path.destination),
                    )
                })
        };
        let path = without_dice
            .or_else(fairly_safe)
            .map(|path| path.squares.clone())
            .unwrap_or_default();
        let action = self
            .may_brace(carrier, path.len())
            .then_some(Action::Stance);
        (!path.is_empty() || action.is_some()).then_some(Activation {
            goblin: carrier.id,
            path,
            action,
        })
    }

    fn guard_carrier(&self) -> Option<Activation> {
        if !self.ruleset.goblins_can_take_stance {
            return None;
        }
        let carrier = self.own_carrier()?;
        let carrier_square = carrier.square()?;
        let ahead = carrier_square
            .orthogonal_neighbours()
            .map(|square| self.den_distance(square))
            .min()
            .unwrap_or(u8::MAX);
        let guard_post = |square: Square| {
            square.is_orthogonally_adjacent(carrier_square) && self.den_distance(square) > ahead
        };
        self.actors()
            .filter(|mover| mover.id != carrier.id)
            .find_map(|mover| {
                let here = mover.square()?;
                let path = if guard_post(here) {
                    self.may_brace(mover, 0).then(Vec::new)?
                } else {
                    self.paths(mover.id)
                        .into_iter()
                        .filter(|path| {
                            path.success == Probability::CERTAIN
                                && guard_post(path.destination)
                                && self.may_brace(mover, path.squares.len())
                        })
                        .min_by_key(|path| (path.squares.len(), self.tie_break(path.destination)))?
                        .squares
                };
                Some(Activation {
                    goblin: mover.id,
                    path,
                    action: Some(Action::Stance),
                })
            })
    }

    fn hit(&self) -> Option<Activation> {
        let candidates: Vec<_> = self
            .actors()
            .flat_map(|mover| {
                let here = mover.square();
                self.state
                    .goblins
                    .iter()
                    .filter(move |enemy| {
                        enemy.team() != self.team
                            && here
                                .zip(enemy.square())
                                .is_some_and(|(from, to)| from.is_orthogonally_adjacent(to))
                    })
                    .map(move |enemy| Activation {
                        goblin: mover.id,
                        path: Vec::new(),
                        action: Some(Action::Assault {
                            target: enemy.id,
                            style: self.style_to_hit(enemy),
                        }),
                    })
            })
            .filter_map(|activation| {
                self.chance_played_for(&activation)
                    .map(|chance| (activation, chance))
            })
            .collect();
        self.most_likely(candidates, Risk::Hit)
    }

    fn focus(&self) -> Option<Square> {
        match self.state.ball {
            Ball::Loose { at } => Some(at),
            Ball::Carried { .. } => self.enemy_carrier().and_then(Goblin::square),
        }
    }

    fn close_in(&self) -> Option<Activation> {
        let focus = self.focus()?;
        self.movers().find_map(|mover| {
            let here = mover.square()?;
            let best = self
                .paths(mover.id)
                .into_iter()
                .filter(|path| {
                    path.success == Probability::CERTAIN
                        && path.destination.orthogonal_distance(focus)
                            < here.orthogonal_distance(focus)
                })
                .min_by_key(|path| {
                    (
                        path.destination.orthogonal_distance(focus),
                        path.squares.len(),
                        self.tie_break(path.destination),
                    )
                })?;
            let braces_in_contact = self.may_brace(mover, best.squares.len())
                && self.enemies_next_to(best.destination) > 0;
            Some(Activation {
                goblin: mover.id,
                path: best.squares,
                action: braces_in_contact.then_some(Action::Stance),
            })
        })
    }

    fn may_brace(&self, goblin: &Goblin, steps_walked: usize) -> bool {
        goblin.may_act_after(Action::Stance, steps_walked, self.ruleset)
            && (self.ruleset.carriers_can_brace || self.state.carrier() != Some(goblin.id))
    }

    fn brace(&self) -> Option<Activation> {
        self.actors()
            .find(|mover| {
                self.may_brace(mover, 0)
                    && mover
                        .square()
                        .is_some_and(|square| self.enemies_next_to(square) > 0)
            })
            .map(|mover| Activation {
                goblin: mover.id,
                path: Vec::new(),
                action: Some(Action::Stance),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::characteristics::Archetype::{Bruiser, Sprinter, Thrower};
    use crate::command::{Action, Activation, AssaultStyle};
    use crate::game::scenario::Scenario;
    use crate::roster::GoblinId;
    use crate::square::Square;
    use crate::state::{Phase, Presence};

    fn id(text: &str) -> GoblinId {
        text.parse().unwrap()
    }

    fn choose(scenario: Scenario) -> Command {
        let game = scenario.game([]);
        cautious_command(game.arena(), game.ruleset(), game.state(), TeamId::HOME)
    }

    fn activation(command: Command) -> Activation {
        match command {
            Command::Act(activation) => activation,
            other => panic!("expected an activation, got {other:?}"),
        }
    }

    fn ends_in_the_away_den(activation: &Activation) -> bool {
        activation
            .path
            .last()
            .is_some_and(|square| Arena::bowl_15().is_scoring_square_for(TeamId::HOME, *square))
    }

    #[test]
    fn given_a_carrier_with_a_free_run_into_the_den_when_choosing_then_it_walks_in_to_score() {
        let scenario = Scenario::new()
            .goblin("H5", Sprinter, 11, 7)
            .ball_carried_by("H5");
        assert!(ends_in_the_away_den(&activation(choose(scenario))));
    }

    #[test]
    fn given_a_carrier_surrounded_by_bruisers_when_choosing_then_it_does_not_gamble_on_scoring() {
        let scenario = Scenario::new()
            .goblin("H5", Sprinter, 12, 7)
            .goblin("A1", Bruiser, 13, 7)
            .goblin("A2", Bruiser, 12, 6)
            .goblin("A3", Bruiser, 12, 8)
            .ball_carried_by("H5");
        assert!(!matches!(
            choose(scenario),
            Command::Act(activation) if ends_in_the_away_den(&activation)
        ));
    }

    #[test]
    fn given_an_enemy_carrier_next_to_a_bruiser_when_choosing_then_the_bruiser_assaults_it() {
        let scenario = Scenario::new()
            .goblin("H1", Bruiser, 7, 7)
            .goblin("A5", Sprinter, 8, 7)
            .ball_carried_by("A5");
        assert_eq!(
            activation(choose(scenario)),
            Activation {
                goblin: id("H1"),
                path: Vec::new(),
                action: Some(Action::assault(id("A5"))),
            }
        );
    }

    #[test]
    fn given_a_loose_ball_within_reach_when_choosing_then_the_best_placed_goblin_goes_for_it() {
        let scenario = Scenario::new()
            .goblin("H1", Bruiser, 4, 7)
            .goblin("H5", Sprinter, 7, 7)
            .ball_loose_at(9, 7);
        let chosen = activation(choose(scenario));
        assert_eq!(
            (chosen.goblin, chosen.path.last().copied()),
            (id("H5"), Some(Square::new(9, 7)))
        );
    }

    #[test]
    fn given_a_carrier_far_from_the_den_when_choosing_then_it_advances_without_dice_and_braces() {
        let scenario = Scenario::new()
            .goblin("H5", Sprinter, 3, 7)
            .ball_carried_by("H5");
        let chosen = activation(choose(scenario));
        assert_eq!(
            (chosen.path.last().copied(), chosen.action),
            (Some(Square::new(8, 7)), Some(Action::Stance))
        );
    }

    #[test]
    fn given_a_carrier_marked_by_a_sprinter_next_to_a_bruiser_when_choosing_then_the_marker_is_hit_first()
     {
        let scenario = Scenario::new()
            .goblin("H1", Bruiser, 8, 8)
            .goblin("H5", Sprinter, 7, 7)
            .goblin("A5", Sprinter, 8, 7)
            .ball_carried_by("H5");
        assert_eq!(
            activation(choose(scenario)),
            Activation {
                goblin: id("H1"),
                path: Vec::new(),
                action: Some(Action::assault(id("A5"))),
            }
        );
    }

    #[test]
    fn given_a_carrier_marked_only_by_a_sprinter_when_choosing_then_it_slips_away_towards_the_den()
    {
        let scenario = Scenario::new()
            .goblin("H5", Sprinter, 7, 7)
            .goblin("A5", Sprinter, 6, 7)
            .ball_carried_by("H5");
        assert_eq!(
            activation(choose(scenario)).path.last().copied(),
            Some(Square::new(12, 7))
        );
    }

    #[test]
    fn given_an_open_sprinter_well_ahead_with_a_clear_line_when_choosing_then_the_carrier_throws_to_it()
     {
        let scenario = Scenario::new()
            .goblin("H9", Thrower, 4, 7)
            .goblin("H5", Sprinter, 9, 7)
            .edit("H5", Goblin::spend_its_turn)
            .ball_carried_by("H9");
        assert_eq!(
            activation(choose(scenario)),
            Activation {
                goblin: id("H9"),
                path: Vec::new(),
                action: Some(Action::throw(id("H5"))),
            }
        );
    }

    #[test]
    fn given_a_carrier_still_to_act_and_an_open_sprinter_behind_it_when_choosing_then_the_sprinter_runs_ahead_first()
     {
        let scenario = Scenario::new()
            .goblin("H9", Thrower, 5, 7)
            .goblin("H5", Sprinter, 4, 5)
            .ball_carried_by("H9");
        let chosen = activation(choose(scenario));
        assert_eq!(
            (chosen.goblin, chosen.path.last().copied()),
            (id("H5"), Some(Square::new(9, 5)))
        );
    }

    #[test]
    fn given_a_carrier_that_already_acted_when_a_bruiser_guards_it_then_it_leaves_the_square_in_front_free()
     {
        let scenario = Scenario::new()
            .goblin("H1", Bruiser, 9, 7)
            .goblin("H5", Sprinter, 7, 7)
            .edit("H5", Goblin::spend_its_turn)
            .ball_carried_by("H5");
        let chosen = activation(choose(scenario));
        assert!(
            chosen.goblin == id("H1")
                && chosen.path.last() != Some(&Square::new(8, 7))
                && chosen
                    .path
                    .last()
                    .is_some_and(|square| square.is_orthogonally_adjacent(Square::new(7, 7)))
        );
    }

    #[test]
    fn given_a_carrier_that_already_acted_when_choosing_then_a_teammate_guards_it_braced() {
        let scenario = Scenario::new()
            .goblin("H1", Bruiser, 4, 7)
            .goblin("H5", Sprinter, 7, 7)
            .edit("H5", Goblin::spend_its_turn)
            .ball_carried_by("H5");
        let chosen = activation(choose(scenario));
        assert_eq!(
            (
                chosen.goblin,
                chosen
                    .path
                    .last()
                    .is_some_and(|square| square.is_orthogonally_adjacent(Square::new(7, 7))),
                chosen.action
            ),
            (id("H1"), true, Some(Action::Stance))
        );
    }

    #[test]
    fn given_a_sprinter_stuck_against_a_bruiser_with_nothing_safe_to_do_when_choosing_then_it_braces()
     {
        let scenario = Scenario::new()
            .goblin("H5", Sprinter, 7, 7)
            .goblin("A1", Bruiser, 8, 7)
            .goblin("A9", Sprinter, 13, 7)
            .ball_carried_by("A9");
        assert_eq!(
            activation(choose(scenario)),
            Activation {
                goblin: id("H5"),
                path: Vec::new(),
                action: Some(Action::Stance),
            }
        );
    }

    #[test]
    fn given_goblins_that_keep_moving_when_a_bruiser_in_contact_already_braced_then_the_turn_ends()
    {
        let scenario = Scenario::new()
            .rules(|ruleset| ruleset.keeps_moving_after_action = true)
            .goblin("H1", Bruiser, 5, 7)
            .goblin("A1", Bruiser, 6, 7)
            .edit("H1", |goblin| goblin.has_action_left = false)
            .ball_loose_at(9, 3);
        assert_eq!(choose(scenario), Command::EndTurn);
    }

    #[test]
    fn given_goblins_that_keep_moving_when_the_carrier_already_braced_then_it_advances_without_bracing()
     {
        let scenario = Scenario::new()
            .rules(|ruleset| ruleset.keeps_moving_after_action = true)
            .goblin("H9", Thrower, 4, 7)
            .edit("H9", |goblin| goblin.has_action_left = false)
            .ball_carried_by("H9");
        assert!(matches!(
            choose(scenario),
            Command::Act(Activation { goblin, action: None, .. }) if goblin == id("H9")
        ));
    }

    #[test]
    fn given_rules_without_stance_when_the_carrier_advances_then_it_does_not_brace() {
        let scenario = Scenario::new()
            .rules(|ruleset| ruleset.goblins_can_take_stance = false)
            .goblin("H5", Sprinter, 3, 7)
            .ball_carried_by("H5");
        let chosen = activation(choose(scenario));
        assert_eq!(
            (chosen.path.last().copied(), chosen.action),
            (Some(Square::new(8, 7)), None)
        );
    }

    #[test]
    fn given_rules_without_stance_when_the_carrier_already_acted_then_nobody_walks_up_to_guard_it()
    {
        let scenario = Scenario::new()
            .rules(|ruleset| ruleset.goblins_can_take_stance = false)
            .goblin("H1", Bruiser, 4, 7)
            .goblin("H5", Sprinter, 7, 7)
            .edit("H5", Goblin::spend_its_turn)
            .ball_carried_by("H5");
        assert!(!matches!(
            choose(scenario),
            Command::Act(Activation {
                action: Some(Action::Stance),
                ..
            })
        ));
    }

    #[test]
    fn given_rules_without_stance_when_a_sprinter_stuck_against_a_bruiser_has_nothing_safe_to_do_then_the_turn_ends()
     {
        let scenario = Scenario::new()
            .rules(|ruleset| ruleset.goblins_can_take_stance = false)
            .goblin("H5", Sprinter, 7, 7)
            .goblin("A1", Bruiser, 8, 7)
            .goblin("A9", Sprinter, 13, 7)
            .ball_carried_by("A9");
        assert_eq!(choose(scenario), Command::EndTurn);
    }

    #[test]
    fn given_carriers_that_cannot_assault_when_a_marked_carrier_could_hit_a_sprinter_then_it_does_not()
     {
        let scenario = Scenario::new()
            .rules(|ruleset| {
                ruleset.carriers_can_assault = false;
                ruleset.goblins_can_take_stance = false;
            })
            .goblin("H1", Bruiser, 7, 7)
            .goblin("A1", Bruiser, 8, 7)
            .goblin("A5", Sprinter, 7, 6)
            .ball_carried_by("H1");
        assert!(!matches!(
            choose(scenario),
            Command::Act(Activation {
                action: Some(Action::Assault { .. }),
                ..
            })
        ));
    }

    fn mirrored_square(arena: &Arena, square: Square) -> Square {
        Square::new(arena.width() - 1 - square.x, square.y)
    }

    fn mirrored_id(id: GoblinId) -> GoblinId {
        GoblinId::new(id.team.duel_opponent(), id.number)
    }

    fn mirrored_presence(arena: &Arena, presence: Presence) -> Presence {
        match presence {
            Presence::Standing(square) => Presence::Standing(mirrored_square(arena, square)),
            Presence::Down(square) => Presence::Down(mirrored_square(arena, square)),
            Presence::KnockedOut => Presence::KnockedOut,
        }
    }

    fn mirrored_state(arena: &Arena, state: &GameState) -> GameState {
        let mut goblins: Vec<Goblin> = state
            .goblins
            .iter()
            .map(|goblin| Goblin {
                id: mirrored_id(goblin.id),
                presence: mirrored_presence(arena, goblin.presence),
                ..goblin.clone()
            })
            .collect();
        goblins.sort_by_key(|goblin| (goblin.id.team.index(), goblin.id.number));
        GameState {
            rosters: state.rosters.iter().rev().copied().collect(),
            goblins,
            ball: match state.ball {
                Ball::Loose { at } => Ball::Loose {
                    at: mirrored_square(arena, at),
                },
                Ball::Carried { by } => Ball::Carried {
                    by: mirrored_id(by),
                },
            },
            score: state.score.iter().rev().copied().collect(),
            half: state.half,
            turns_played: state.turns_played.iter().rev().copied().collect(),
            phase: match state.phase {
                Phase::Playing { active_team } => Phase::Playing {
                    active_team: active_team.duel_opponent(),
                },
                Phase::Finished => Phase::Finished,
            },
            opening_team: state.opening_team.duel_opponent(),
        }
    }

    type Purpose = Option<(GoblinId, Option<Square>, Option<Action>)>;

    fn purpose(state: &GameState, command: &Command) -> Purpose {
        let Command::Act(activation) = command else {
            return None;
        };
        let destination = activation
            .path
            .last()
            .copied()
            .or_else(|| state.goblin(activation.goblin).and_then(Goblin::square));
        Some((activation.goblin, destination, activation.action))
    }

    fn mirrored_purpose(arena: &Arena, purpose: Purpose) -> Purpose {
        purpose.map(|(goblin, destination, action)| {
            (
                mirrored_id(goblin),
                destination.map(|square| mirrored_square(arena, square)),
                action.map(|action| match action {
                    Action::Assault { target, style } => Action::Assault {
                        target: mirrored_id(target),
                        style: match style {
                            AssaultStyle::Projection(direction) => {
                                AssaultStyle::Projection(direction.mirrored_east_west())
                            }
                            other => other,
                        },
                    },
                    Action::Throw { receiver, style } => Action::Throw {
                        receiver: mirrored_id(receiver),
                        style,
                    },
                    Action::Stance | Action::Focus => action,
                }),
            )
        })
    }

    fn bruisers_face_to_face_where_ties_stand_off() -> Scenario {
        Scenario::new()
            .rules(|ruleset| ruleset.assault_ties_knock_down = false)
            .goblin("H1", Bruiser, 5, 7)
            .goblin("A1", Bruiser, 6, 7)
            .goblin("A9", Sprinter, 13, 13)
            .ball_carried_by("A9")
    }

    fn is_an_assault(command: &Command) -> bool {
        matches!(
            command,
            Command::Act(Activation {
                action: Some(Action::Assault { .. }),
                ..
            })
        )
    }

    #[test]
    fn given_ties_that_stand_off_when_the_first_bold_coach_faces_an_even_fight_then_it_still_hits_on_its_five_eighths_to_keep_its_feet()
     {
        let game = bruisers_face_to_face_where_ties_stand_off().game([]);
        let command = bold_command(game.arena(), game.ruleset(), game.state(), TeamId::HOME);
        assert!(is_an_assault(&command));
    }

    #[test]
    fn given_ties_that_stand_off_when_the_second_bold_coach_faces_an_even_fight_then_its_three_eighths_to_knock_down_hold_it_back()
     {
        let game = bruisers_face_to_face_where_ties_stand_off().game([]);
        let command = styled_command(
            game.arena(),
            game.ruleset(),
            game.state(),
            TeamId::HOME,
            Style::second(Appetite::bold()),
        );
        assert!(!is_an_assault(&command));
    }

    fn styled_command_of(scenario: Scenario, style: Style) -> Command {
        let game = scenario.game([]);
        styled_command(
            game.arena(),
            game.ruleset(),
            game.state(),
            TeamId::HOME,
            style,
        )
    }

    fn with_styles(scenario: Scenario) -> Scenario {
        scenario.rules(|ruleset| {
            ruleset.assault_ties_knock_down = false;
            ruleset.assaults_have_styles = true;
            ruleset.throws_have_styles = true;
        })
    }

    #[test]
    fn given_assault_styles_when_the_second_cautious_coach_sacks_the_carrier_then_chirurgically() {
        let scenario = with_styles(
            Scenario::new()
                .goblin("H1", Bruiser, 5, 7)
                .goblin("A5", Sprinter, 6, 7)
                .ball_carried_by("A5"),
        );
        assert!(matches!(
            styled_command_of(scenario, Style::second(Appetite::cautious())),
            Command::Act(Activation {
                action: Some(Action::Assault {
                    style: AssaultStyle::Chirurgical,
                    ..
                }),
                ..
            })
        ));
    }

    #[test]
    fn given_assault_styles_when_the_second_bold_coach_hits_a_downed_goblin_left_on_two_health_then_brutally()
     {
        let scenario = with_styles(
            Scenario::new()
                .goblin("H1", Bruiser, 5, 7)
                .goblin("A5", Sprinter, 6, 7)
                .goblin("A9", Sprinter, 13, 13)
                .ball_carried_by("A9")
                .edit("A5", |goblin| {
                    goblin.presence = Presence::Down(Square::new(6, 7));
                    goblin.health = 2;
                }),
        );
        assert!(matches!(
            styled_command_of(scenario, Style::second(Appetite::bold())),
            Command::Act(Activation {
                action: Some(Action::Assault {
                    style: AssaultStyle::Brutal,
                    ..
                }),
                ..
            })
        ));
    }

    fn thrower_far_from_its_receiver_with_focused_throws() -> Scenario {
        Scenario::new()
            .rules(|ruleset| ruleset.focused_throws_spend_steps_left = true)
            .goblin("H9", Thrower, 4, 7)
            .goblin("H2", Thrower, 12, 7)
            .goblin("A1", Sprinter, 12, 2)
            .ball_carried_by("H9")
    }

    #[test]
    fn given_focused_throws_when_a_second_generation_thrower_finds_a_long_pass_worth_it_only_focused_then_it_throws_focused()
     {
        assert_eq!(
            styled_command_of(
                thrower_far_from_its_receiver_with_focused_throws(),
                Style::second(Appetite::cautious())
            ),
            Command::Act(Activation {
                goblin: id("H9"),
                path: Vec::new(),
                action: Some(Action::Throw {
                    receiver: id("H2"),
                    style: ThrowStyle::Focused
                }),
            })
        );
    }

    #[test]
    fn given_focused_throws_when_a_first_generation_thrower_finds_the_same_pass_then_it_never_throws_focused()
     {
        assert!(!matches!(
            choose(thrower_far_from_its_receiver_with_focused_throws()),
            Command::Act(Activation {
                action: Some(Action::Throw {
                    style: ThrowStyle::Focused,
                    ..
                }),
                ..
            })
        ));
    }

    fn marked_thrower_with_the_ball() -> Scenario {
        Scenario::new()
            .goblin("H9", Thrower, 4, 7)
            .goblin("H2", Thrower, 11, 7)
            .goblin("A1", Sprinter, 4, 8)
            .ball_carried_by("H9")
    }

    #[test]
    fn given_throw_styles_when_a_marked_thrower_finds_no_pass_worth_its_appetite_then_it_throws_into_chaos_towards_a_ring_of_teammates()
     {
        let scenario = with_styles(
            marked_thrower_with_the_ball()
                .goblin("H3", Thrower, 10, 6)
                .goblin("H4", Thrower, 11, 6)
                .goblin("H5", Thrower, 12, 6)
                .goblin("H6", Thrower, 10, 8)
                .goblin("H7", Thrower, 11, 8)
                .goblin("H8", Thrower, 12, 8),
        );
        assert_eq!(
            styled_command_of(scenario, Style::second(Appetite::cautious())),
            Command::Act(Activation {
                goblin: id("H9"),
                path: Vec::new(),
                action: Some(Action::Throw {
                    receiver: id("H2"),
                    style: ThrowStyle::Chaos
                }),
            })
        );
    }

    #[test]
    fn given_throw_styles_when_a_marked_thrower_s_teammate_stands_alone_then_no_chaos_throw() {
        let scenario = with_styles(marked_thrower_with_the_ball());
        assert!(!matches!(
            styled_command_of(scenario, Style::second(Appetite::cautious())),
            Command::Act(Activation {
                action: Some(Action::Throw {
                    style: ThrowStyle::Chaos,
                    ..
                }),
                ..
            })
        ));
    }

    fn rules_where_goblins_act_often_at_a_cost() -> Ruleset {
        Ruleset {
            goblins_can_take_stance: true,
            stance_bonus_cap: 1,
            neighbour_stance_bonus_cap: 0,
            assault_ties_knock_down: false,
            carrier_defense_malus: 1,
            knocked_out_goblins_may_return: true,
            actions_cost_a_step: true,
            one_action_of_each_kind: true,
            braced_tacklers_weigh_double: false,
            reach_covers_diagonals: true,
            ball_bounces_all_around: true,
            assaults_have_styles: true,
            throws_have_styles: true,
            carriers_can_brace: false,
            ..Ruleset::default()
        }
    }

    fn refusals_over_whole_matches(style: Style, ruleset: &Ruleset, seeds: u64) -> usize {
        let mut refusals = 0;
        for seed in 1..=seeds {
            let (mut game, _) = crate::game::Game::start(
                ruleset.clone(),
                [crate::roster::Roster::DEFAULT; 2],
                crate::dice::SeededDice::from_seed(seed),
            )
            .unwrap();
            for _ in 0..4_000 {
                let Some(team) = game.state().active_team() else {
                    break;
                };
                let arena = game.arena().clone();
                let rules = game.ruleset().clone();
                let command = styled_command(&arena, &rules, game.state(), team, style);
                if game.apply(team, command).is_err() {
                    refusals += 1;
                    game.apply(team, Command::EndTurn).unwrap();
                }
            }
        }
        refusals
    }

    #[test]
    fn given_actions_at_a_step_one_of_each_kind_and_stance_when_every_coach_plays_whole_matches_then_no_order_is_refused()
     {
        let ruleset = rules_where_goblins_act_often_at_a_cost();
        let first = |appetite| Style {
            generation: Generation::First,
            temper: Temper::Fixed(appetite),
        };
        assert_eq!(
            [
                first(Appetite::cautious()),
                first(Appetite::bold()),
                Style::second(Appetite::cautious()),
                Style::balanced(),
                Style::second(Appetite::bold()),
            ]
            .map(|style| refusals_over_whole_matches(style, &ruleset, 2)),
            [0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn given_focused_throws_and_no_stance_when_every_coach_plays_whole_matches_then_no_order_is_refused()
     {
        let ruleset = Ruleset {
            goblins_can_take_stance: false,
            focused_throws_spend_steps_left: true,
            ..rules_where_goblins_act_often_at_a_cost()
        };
        let first = |appetite| Style {
            generation: Generation::First,
            temper: Temper::Fixed(appetite),
        };
        assert_eq!(
            [
                first(Appetite::cautious()),
                first(Appetite::bold()),
                Style::second(Appetite::cautious()),
                Style::balanced(),
                Style::second(Appetite::bold()),
            ]
            .map(|style| refusals_over_whole_matches(style, &ruleset, 2)),
            [0, 0, 0, 0, 0]
        );
    }

    fn mirror_breaks(style: Style, seeds: std::ops::RangeInclusive<u64>, commands: usize) -> usize {
        let mut broken = 0;
        for seed in seeds {
            let (mut game, _) = crate::game::Game::start(
                Ruleset::default(),
                [crate::roster::Roster::DEFAULT; 2],
                crate::dice::SeededDice::from_seed(seed),
            )
            .unwrap();
            for _ in 0..commands {
                let Some(team) = game.state().active_team() else {
                    break;
                };
                let arena = game.arena().clone();
                let ruleset = game.ruleset().clone();
                let chosen = styled_command(&arena, &ruleset, game.state(), team, style);
                let mirror = mirrored_state(&arena, game.state());
                let chosen_in_the_mirror =
                    styled_command(&arena, &ruleset, &mirror, team.duel_opponent(), style);
                if purpose(game.state(), &chosen)
                    != mirrored_purpose(&arena, purpose(&mirror, &chosen_in_the_mirror))
                {
                    broken += 1;
                }
                if game.apply(team, chosen).is_err() {
                    break;
                }
            }
        }
        broken
    }

    #[test]
    fn given_the_second_generation_when_the_bowl_is_mirrored_then_both_teams_choose_mirrored_orders()
     {
        assert_eq!(
            [
                Style::second(Appetite::cautious()),
                Style::balanced(),
                Style::second(Appetite::bold())
            ]
            .map(|style| mirror_breaks(style, 1..=6, 120)),
            [0, 0, 0]
        );
    }

    #[test]
    fn given_the_first_generation_when_the_bowl_is_mirrored_then_its_square_order_shows() {
        let first = Style {
            generation: Generation::First,
            temper: Temper::Fixed(Appetite::cautious()),
        };
        assert!(mirror_breaks(first, 1..=6, 120) > 0);
    }

    fn ball_next_to_a_bruiser_and_a_sprinter_far_away() -> Scenario {
        Scenario::new()
            .goblin("H1", Sprinter, 6, 7)
            .goblin("H2", Sprinter, 3, 3)
            .goblin("A1", Bruiser, 9, 7)
            .ball_loose_at(8, 7)
    }

    #[test]
    fn given_the_first_generation_when_a_pick_up_is_within_its_appetite_then_it_gambles_at_once() {
        let chosen = activation(choose(ball_next_to_a_bruiser_and_a_sprinter_far_away()));
        assert_eq!(chosen.goblin, id("H1"));
    }

    #[test]
    fn given_the_second_generation_when_a_pick_up_is_within_its_appetite_then_another_goblin_closes_in_without_dice_first()
     {
        let game = ball_next_to_a_bruiser_and_a_sprinter_far_away().game([]);
        let chosen = activation(styled_command(
            game.arena(),
            game.ruleset(),
            game.state(),
            TeamId::HOME,
            Style::second(Appetite::cautious()),
        ));
        let chance = activation_odds(
            game.arena(),
            game.ruleset(),
            game.state(),
            TeamId::HOME,
            &chosen,
        )
        .map(|odds| odds.overall_success);
        assert_eq!(
            (chosen.goblin, chance),
            (id("H2"), Ok(Probability::CERTAIN))
        );
    }

    fn bruisers_face_to_face_far_from_the_ball() -> Scenario {
        Scenario::new()
            .goblin("H1", Bruiser, 5, 7)
            .goblin("A1", Bruiser, 6, 7)
            .ball_loose_at(1, 10)
    }

    fn balanced(scenario: Scenario) -> Command {
        let game = scenario.game([]);
        styled_command(
            game.arena(),
            game.ruleset(),
            game.state(),
            TeamId::HOME,
            Style::balanced(),
        )
    }

    fn assaults(command: &Command) -> bool {
        matches!(
            command,
            Command::Act(Activation {
                action: Some(Action::Assault { .. }),
                ..
            })
        )
    }

    #[test]
    fn given_the_balanced_coach_alone_to_play_when_a_bruiser_can_hit_another_at_62_5_percent_then_it_hits_as_the_bold_one_would()
     {
        assert!(assaults(&balanced(
            bruisers_face_to_face_far_from_the_ball()
        )));
    }

    #[test]
    fn given_the_balanced_coach_with_eight_teammates_still_to_play_when_a_bruiser_can_hit_at_62_5_percent_then_it_holds_back_as_the_cautious_one_would()
     {
        let crowded = (2..=9).fold(
            bruisers_face_to_face_far_from_the_ball(),
            |scenario, number| scenario.goblin(&format!("H{number}"), Sprinter, number, 1),
        );
        assert!(!assaults(&balanced(crowded)));
    }

    #[test]
    fn given_a_quarter_of_the_way_from_a_half_to_three_quarters_when_blending_then_nine_sixteenths()
    {
        assert_eq!(
            between(Probability::new(1, 2), Probability::new(3, 4), 2, 8),
            Probability::new(9, 16)
        );
    }

    #[test]
    fn given_every_goblin_already_acted_when_choosing_then_the_turn_ends() {
        let scenario = Scenario::new()
            .goblin("H5", Sprinter, 7, 7)
            .edit("H5", Goblin::spend_its_turn)
            .ball_loose_at(9, 7);
        assert_eq!(choose(scenario), Command::EndTurn);
    }
}
