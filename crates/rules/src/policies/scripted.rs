//! Scripted coaches after the Bot Bowl recipe: score when the odds are good, free the ball, then
//! move without dice and brace, taking a risk only within the coach's appetite, and ending the turn
//! otherwise. The cautious and bold coaches differ only in their appetite.

use std::cmp::Reverse;

use crate::arena::Arena;
use crate::characteristics::Archetype;
use crate::command::{Action, Activation, Command};
use crate::odds::Probability;
use crate::paths::{PlannedPath, reachable_squares};
use crate::preview::{ActionOdds, activation_odds};
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

/// The next command of a scripted coach for `team` with this `appetite`, without any randomness.
///
/// In order of priority: score, assault the enemy carrier, pick up the loose ball, hit the carrier's
/// markers, run Sprinters ahead into open space while the carrier still has to act, throw to an
/// open teammate well ahead along a clear line, advance the carrier (without dice if that gets
/// closer, else within the appetite) and brace it, guard the carrier without standing in front of
/// it, hit an adjacent enemy, close in on the ball without dice, brace any goblin in contact;
/// otherwise end the turn. Every risk is taken only within the appetite.
pub fn scripted_command(
    arena: &Arena,
    ruleset: &Ruleset,
    state: &GameState,
    team: TeamId,
    appetite: Appetite,
) -> Command {
    let coach = Coach {
        arena,
        ruleset,
        state,
        team,
        appetite,
    };
    coach
        .score()
        .or_else(|| coach.sack())
        .or_else(|| coach.pick())
        .or_else(|| coach.free_carrier())
        .or_else(|| coach.run_ahead())
        .or_else(|| coach.pass())
        .or_else(|| coach.advance_carrier())
        .or_else(|| coach.guard_carrier())
        .or_else(|| coach.hit())
        .or_else(|| coach.close_in())
        .or_else(|| coach.brace())
        .map_or(Command::EndTurn, Command::Act)
}

struct Coach<'a> {
    arena: &'a Arena,
    ruleset: &'a Ruleset,
    state: &'a GameState,
    team: TeamId,
    appetite: Appetite,
}

impl Coach<'_> {
    fn movers(&self) -> impl Iterator<Item = &Goblin> + '_ {
        self.state
            .goblins_of(self.team)
            .filter(|goblin| goblin.can_act())
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
            .standing_opponents_next_to(square, self.team)
            .count()
    }

    fn most_likely(
        candidates: impl IntoIterator<Item = (Activation, Probability)>,
        threshold: Probability,
    ) -> Option<Activation> {
        candidates
            .into_iter()
            .filter(|(_, success)| *success >= threshold)
            .min_by_key(|(activation, success)| (Reverse(*success), activation.path.len()))
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
        Self::most_likely(candidates, self.appetite.score)
    }

    fn sack(&self) -> Option<Activation> {
        let carrier = self.enemy_carrier()?;
        let target = carrier.square()?;
        let assault = Some(Action::Assault { target: carrier.id });
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
                self.success(&activation)
                    .map(|success| (activation, success))
            })
            .collect();
        Self::most_likely(candidates, self.appetite.sack)
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
        Self::most_likely(candidates, self.appetite.pick)
    }

    fn free_carrier(&self) -> Option<Activation> {
        let carrier = self.own_carrier()?;
        let carrier_square = carrier.square()?;
        let markers: Vec<_> = self
            .state
            .standing_opponents_next_to(carrier_square, self.team)
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
                            action: Some(Action::Assault { target: *marker }),
                        })
                })
            })
            .filter_map(|activation| {
                self.success(&activation)
                    .map(|success| (activation, success))
            })
            .collect();
        Self::most_likely(candidates, self.appetite.free)
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
                    .min_by_key(|path| (self.den_distance(path.destination), path.squares.len()))?;
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
            .filter_map(|receiver| {
                let throw = Activation {
                    goblin: carrier.id,
                    path: Vec::new(),
                    action: Some(Action::Throw {
                        receiver: receiver.id,
                    }),
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
        Self::most_likely(candidates, self.appetite.pass)
    }

    fn advance_carrier(&self) -> Option<Activation> {
        let carrier = self.own_carrier().filter(|carrier| carrier.can_act())?;
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
                )
            });
        let fairly_safe = || {
            closer
                .iter()
                .filter(|path| path.success >= self.appetite.advance)
                .min_by_key(|path| {
                    (
                        self.den_distance(path.destination),
                        Reverse(path.success),
                        path.squares.len(),
                    )
                })
        };
        let path = without_dice
            .or_else(fairly_safe)
            .map(|path| path.squares.clone())
            .unwrap_or_default();
        let action = carrier.has_action_left.then_some(Action::Stance);
        (!path.is_empty() || action.is_some()).then_some(Activation {
            goblin: carrier.id,
            path,
            action,
        })
    }

    fn guard_carrier(&self) -> Option<Activation> {
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
                    Vec::new()
                } else {
                    self.paths(mover.id)
                        .into_iter()
                        .filter(|path| {
                            path.success == Probability::CERTAIN && guard_post(path.destination)
                        })
                        .min_by_key(|path| path.squares.len())?
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
                        action: Some(Action::Assault { target: enemy.id }),
                    })
            })
            .filter_map(|activation| {
                self.success(&activation)
                    .map(|success| (activation, success))
            })
            .collect();
        Self::most_likely(candidates, self.appetite.hit)
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
                    )
                })?;
            let braces_in_contact =
                mover.has_action_left && self.enemies_next_to(best.destination) > 0;
            Some(Activation {
                goblin: mover.id,
                path: best.squares,
                action: braces_in_contact.then_some(Action::Stance),
            })
        })
    }

    fn brace(&self) -> Option<Activation> {
        self.actors()
            .find(|mover| {
                mover
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
    use crate::command::{Action, Activation};
    use crate::game::scenario::Scenario;
    use crate::roster::GoblinId;
    use crate::square::Square;

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
                action: Some(Action::Assault { target: id("A5") }),
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
                action: Some(Action::Assault { target: id("A5") }),
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
                action: Some(Action::Throw { receiver: id("H5") }),
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
    fn given_every_goblin_already_acted_when_choosing_then_the_turn_ends() {
        let scenario = Scenario::new()
            .goblin("H5", Sprinter, 7, 7)
            .edit("H5", Goblin::spend_its_turn)
            .ball_loose_at(9, 7);
        assert_eq!(choose(scenario), Command::EndTurn);
    }
}
