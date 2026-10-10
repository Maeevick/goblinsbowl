//! Activations: validating a goblin's order in full, then resolving it step by step.

use crate::command::Activation;
use crate::dice::DiceRoller;
use crate::event::{GameEvent, TurnEndReason};
use crate::state::Presence;
use crate::team::TeamId;
use crate::violation::RuleViolation;

use super::Game;

impl<D: DiceRoller> Game<D> {
    pub(super) fn validate_activation(
        &self,
        team: TeamId,
        activation: &Activation,
    ) -> Result<(), RuleViolation> {
        let id = activation.goblin;
        let goblin = self
            .state
            .goblin(id)
            .ok_or(RuleViolation::UnknownGoblin { goblin: id })?;
        if goblin.team() != team {
            return Err(RuleViolation::NotYourGoblin { goblin: id });
        }
        let Presence::Standing(start) = goblin.presence else {
            return Err(RuleViolation::NotStanding { goblin: id });
        };
        if goblin.has_acted {
            return Err(RuleViolation::AlreadyActed { goblin: id });
        }
        let movement = goblin.characteristics.movement;
        if activation.path.len() > usize::from(movement) {
            return Err(RuleViolation::PathTooLong {
                goblin: id,
                movement,
                steps: activation.path.len(),
            });
        }
        let mut visited = vec![start];
        for (index, square) in activation.path.iter().copied().enumerate() {
            let previous = visited.last().copied().unwrap_or(start);
            if !previous.is_orthogonally_adjacent(square) {
                return Err(RuleViolation::PathNotContiguous { index, square });
            }
            if !self.arena.is_in_bounds(square) {
                return Err(RuleViolation::PathLeavesTheBowl { square });
            }
            if visited.contains(&square) {
                return Err(RuleViolation::PathRevisitsSquare { square });
            }
            if self.state.goblin_at(square).is_some() {
                return Err(RuleViolation::PathBlocked { square });
            }
            visited.push(square);
        }
        Ok(())
    }

    pub(super) fn resolve_activation(
        &mut self,
        activation: Activation,
        events: &mut Vec<GameEvent>,
    ) {
        let id = activation.goblin;
        let Some(team) = self.state.goblin(id).map(|goblin| goblin.team()) else {
            return;
        };
        for to in activation.path {
            let Some(goblin) = self.state.goblin_mut(id) else {
                return;
            };
            let Presence::Standing(from) = goblin.presence else {
                return;
            };
            goblin.presence = Presence::Standing(to);
            events.push(GameEvent::Moved {
                goblin: id,
                from,
                to,
            });
        }
        if let Some(goblin) = self.state.goblin_mut(id) {
            goblin.has_acted = true;
        }
        if !self.has_goblin_able_to_act(team) {
            self.finish_turn(TurnEndReason::NoGoblinCanAct, events);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::characteristics::Archetype::{Bruiser, Sprinter};
    use crate::command::{Activation, Command};
    use crate::event::{GameEvent, TurnEndReason};
    use crate::game::scenario::Scenario;
    use crate::roster::GoblinId;
    use crate::square::Square;
    use crate::state::{Ball, Presence};
    use crate::team::TeamId;
    use crate::violation::RuleViolation;

    fn id(text: &str) -> GoblinId {
        text.parse().unwrap()
    }

    fn act(goblin: &str, path: &[(u8, u8)]) -> Command {
        Command::Act(Activation {
            goblin: id(goblin),
            path: path.iter().map(|(x, y)| Square::new(*x, *y)).collect(),
        })
    }

    fn two_sprinters_apart() -> Scenario {
        Scenario::new()
            .goblin("H1", Sprinter, 3, 7)
            .goblin("H2", Sprinter, 3, 9)
            .goblin("A1", Sprinter, 11, 7)
    }

    #[test]
    fn given_free_path_when_a_goblin_moves_then_it_stands_on_the_last_square() {
        let mut game = two_sprinters_apart().game([]);
        game.apply(TeamId::HOME, act("H1", &[(4, 7), (5, 7), (5, 6)]))
            .unwrap();
        assert_eq!(
            game.state().goblin(id("H1")).unwrap().presence,
            Presence::Standing(Square::new(5, 6))
        );
    }

    #[test]
    fn given_free_path_when_a_goblin_moves_then_one_moved_event_per_step() {
        let mut game = two_sprinters_apart().game([]);
        let events = game
            .apply(TeamId::HOME, act("H1", &[(4, 7), (5, 7)]))
            .unwrap();
        assert_eq!(
            events,
            vec![
                GameEvent::Moved {
                    goblin: id("H1"),
                    from: Square::new(3, 7),
                    to: Square::new(4, 7)
                },
                GameEvent::Moved {
                    goblin: id("H1"),
                    from: Square::new(4, 7),
                    to: Square::new(5, 7)
                },
            ]
        );
    }

    #[test]
    fn given_a_goblin_that_moved_when_asking_if_it_can_act_then_no() {
        let mut game = two_sprinters_apart().game([]);
        game.apply(TeamId::HOME, act("H1", &[(4, 7)])).unwrap();
        assert!(!game.state().goblin(id("H1")).unwrap().can_act());
    }

    #[test]
    fn given_an_empty_path_when_a_goblin_acts_then_it_stays_and_has_acted() {
        let mut game = two_sprinters_apart().game([]);
        game.apply(TeamId::HOME, act("H1", &[])).unwrap();
        assert!(game.state().goblin(id("H1")).unwrap().has_acted);
    }

    #[test]
    fn given_a_carrier_when_it_moves_then_the_ball_moves_with_it() {
        let mut game = two_sprinters_apart().ball_carried_by("H1").game([]);
        game.apply(TeamId::HOME, act("H1", &[(4, 7), (5, 7)]))
            .unwrap();
        assert_eq!(game.state().ball_square(), Some(Square::new(5, 7)));
    }

    #[test]
    fn given_path_longer_than_movement_when_acting_then_path_too_long() {
        let mut game = two_sprinters_apart().game([]);
        let refusal = game.apply(
            TeamId::HOME,
            act("H1", &[(4, 7), (5, 7), (6, 7), (7, 7), (8, 7), (9, 7)]),
        );
        assert_eq!(
            refusal,
            Err(RuleViolation::PathTooLong {
                goblin: id("H1"),
                movement: 5,
                steps: 6
            })
        );
    }

    #[test]
    fn given_path_with_a_jump_when_acting_then_path_not_contiguous() {
        let mut game = two_sprinters_apart().game([]);
        assert_eq!(
            game.apply(TeamId::HOME, act("H1", &[(4, 7), (6, 7)])),
            Err(RuleViolation::PathNotContiguous {
                index: 1,
                square: Square::new(6, 7)
            })
        );
    }

    #[test]
    fn given_diagonal_step_when_acting_then_path_not_contiguous() {
        let mut game = two_sprinters_apart().game([]);
        assert_eq!(
            game.apply(TeamId::HOME, act("H1", &[(4, 8)])),
            Err(RuleViolation::PathNotContiguous {
                index: 0,
                square: Square::new(4, 8)
            })
        );
    }

    #[test]
    fn given_path_into_a_cut_corner_when_acting_then_path_leaves_the_bowl() {
        let mut game = Scenario::new().goblin("H1", Sprinter, 2, 3).game([]);
        assert_eq!(
            game.apply(TeamId::HOME, act("H1", &[(2, 2)])),
            Err(RuleViolation::PathLeavesTheBowl {
                square: Square::new(2, 2)
            })
        );
    }

    #[test]
    fn given_path_through_a_teammate_when_acting_then_path_blocked() {
        let mut game = two_sprinters_apart().game([]);
        assert_eq!(
            game.apply(TeamId::HOME, act("H1", &[(3, 8), (3, 9), (3, 10)])),
            Err(RuleViolation::PathBlocked {
                square: Square::new(3, 9)
            })
        );
    }

    #[test]
    fn given_path_through_a_downed_goblin_when_acting_then_path_blocked() {
        let mut game = two_sprinters_apart()
            .goblin("A2", Bruiser, 4, 7)
            .edit("A2", |goblin| {
                goblin.presence = Presence::Down(Square::new(4, 7))
            })
            .game([]);
        assert_eq!(
            game.apply(TeamId::HOME, act("H1", &[(4, 7)])),
            Err(RuleViolation::PathBlocked {
                square: Square::new(4, 7)
            })
        );
    }

    #[test]
    fn given_path_back_to_its_start_when_acting_then_path_revisits_square() {
        let mut game = two_sprinters_apart().game([]);
        assert_eq!(
            game.apply(TeamId::HOME, act("H1", &[(4, 7), (3, 7)])),
            Err(RuleViolation::PathRevisitsSquare {
                square: Square::new(3, 7)
            })
        );
    }

    #[test]
    fn given_home_turn_when_away_acts_then_not_your_turn() {
        let mut game = two_sprinters_apart().game([]);
        assert_eq!(
            game.apply(TeamId::AWAY, act("A1", &[(10, 7)])),
            Err(RuleViolation::NotYourTurn {
                active_team: TeamId::HOME
            })
        );
    }

    #[test]
    fn given_home_turn_when_home_orders_an_away_goblin_then_not_your_goblin() {
        let mut game = two_sprinters_apart().game([]);
        assert_eq!(
            game.apply(TeamId::HOME, act("A1", &[(10, 7)])),
            Err(RuleViolation::NotYourGoblin { goblin: id("A1") })
        );
    }

    #[test]
    fn given_unknown_goblin_when_acting_then_unknown_goblin() {
        let mut game = two_sprinters_apart().game([]);
        assert_eq!(
            game.apply(TeamId::HOME, act("H7", &[])),
            Err(RuleViolation::UnknownGoblin { goblin: id("H7") })
        );
    }

    #[test]
    fn given_downed_goblin_when_acting_then_not_standing() {
        let mut game = two_sprinters_apart()
            .edit("H1", |goblin| {
                goblin.presence = Presence::Down(Square::new(3, 7))
            })
            .game([]);
        assert_eq!(
            game.apply(TeamId::HOME, act("H1", &[])),
            Err(RuleViolation::NotStanding { goblin: id("H1") })
        );
    }

    #[test]
    fn given_goblin_that_already_acted_when_acting_again_then_already_acted() {
        let mut game = two_sprinters_apart().game([]);
        game.apply(TeamId::HOME, act("H1", &[(4, 7)])).unwrap();
        assert_eq!(
            game.apply(TeamId::HOME, act("H1", &[(5, 7)])),
            Err(RuleViolation::AlreadyActed { goblin: id("H1") })
        );
    }

    #[test]
    fn given_refused_order_when_applied_then_the_state_is_untouched() {
        let mut game = two_sprinters_apart().game([]);
        let before = game.state().clone();
        let _ = game.apply(TeamId::HOME, act("H1", &[(4, 7), (6, 7)]));
        assert_eq!(game.state(), &before);
    }

    #[test]
    fn given_home_turn_when_home_ends_it_then_away_plays() {
        let mut game = two_sprinters_apart().game([]);
        game.apply(TeamId::HOME, Command::EndTurn).unwrap();
        assert_eq!(game.state().active_team(), Some(TeamId::AWAY));
    }

    #[test]
    fn given_home_turn_when_home_ends_it_then_turn_ended_then_away_turn_one_started() {
        let mut game = two_sprinters_apart().game([]);
        assert_eq!(
            game.apply(TeamId::HOME, Command::EndTurn),
            Ok(vec![
                GameEvent::TurnEnded {
                    team: TeamId::HOME,
                    reason: TurnEndReason::EndTurn
                },
                GameEvent::TurnStarted {
                    team: TeamId::AWAY,
                    half: 1,
                    turn: 1
                },
            ])
        );
    }

    #[test]
    fn given_the_clock_runs_out_when_applied_then_the_turn_ends_for_time_up() {
        let mut game = two_sprinters_apart().game([]);
        let events = game.apply(TeamId::HOME, Command::TimeUp).unwrap();
        assert_eq!(
            events.first(),
            Some(&GameEvent::TurnEnded {
                team: TeamId::HOME,
                reason: TurnEndReason::TimeUp
            })
        );
    }

    #[test]
    fn given_last_home_goblin_acts_when_resolved_then_the_turn_ends_on_its_own() {
        let mut game = two_sprinters_apart().game([]);
        game.apply(TeamId::HOME, act("H1", &[(4, 7)])).unwrap();
        let events = game.apply(TeamId::HOME, act("H2", &[(4, 9)])).unwrap();
        assert_eq!(
            events.get(1),
            Some(&GameEvent::TurnEnded {
                team: TeamId::HOME,
                reason: TurnEndReason::NoGoblinCanAct
            })
        );
    }

    #[test]
    fn given_away_goblin_down_when_away_turn_starts_then_it_stands_but_cannot_act() {
        let mut game = two_sprinters_apart()
            .edit("A1", |goblin| {
                goblin.presence = Presence::Down(Square::new(11, 7))
            })
            .game([]);
        game.apply(TeamId::HOME, Command::EndTurn).unwrap();
        let a1 = game.state().goblin(id("A1")).unwrap();
        assert_eq!((a1.is_standing(), a1.can_act()), (true, false));
    }

    #[test]
    fn given_loose_ball_when_a_goblin_walks_past_then_the_ball_stays() {
        let mut game = two_sprinters_apart().ball_loose_at(9, 9).game([]);
        game.apply(TeamId::HOME, act("H1", &[(4, 7)])).unwrap();
        assert_eq!(
            game.state().ball,
            Ball::Loose {
                at: Square::new(9, 9)
            }
        );
    }
}
