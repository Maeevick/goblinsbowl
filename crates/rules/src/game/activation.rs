//! Activations: validating a goblin's order in full, then resolving it step by step.

use crate::command::{Action, Activation};
use crate::dice::DiceRoller;
use crate::event::{GameEvent, TurnEndReason};
use crate::roster::GoblinId;
use crate::square::Square;
use crate::state::{Ball, Presence};
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
        let final_square = visited.last().copied().unwrap_or(start);
        match activation.action {
            None | Some(Action::Stance) => Ok(()),
            Some(Action::Assault { target }) => self.validate_assault(team, final_square, target),
            Some(Action::Throw { receiver }) => {
                self.validate_throw(team, id, &activation.path, receiver)
            }
        }
    }

    fn validate_assault(
        &self,
        team: TeamId,
        attacker_square: Square,
        target: GoblinId,
    ) -> Result<(), RuleViolation> {
        let defender = self
            .state
            .goblin(target)
            .ok_or(RuleViolation::UnknownGoblin { goblin: target })?;
        if defender.team() == team {
            return Err(RuleViolation::TargetNotAnOpponent { target });
        }
        let Some(defender_square) = defender.square() else {
            return Err(RuleViolation::TargetNotOnTheBowl { target });
        };
        if !attacker_square.is_orthogonally_adjacent(defender_square) {
            return Err(RuleViolation::TargetNotAdjacent { target });
        }
        Ok(())
    }

    fn validate_throw(
        &self,
        team: TeamId,
        thrower: GoblinId,
        path: &[Square],
        receiver: GoblinId,
    ) -> Result<(), RuleViolation> {
        let holds_the_ball = self.state.carrier() == Some(thrower);
        let picks_it_on_the_way =
            matches!(self.state.ball, Ball::Loose { at } if path.contains(&at));
        if !holds_the_ball && !picks_it_on_the_way {
            return Err(RuleViolation::ThrowWithoutBall { goblin: thrower });
        }
        let can_receive = self.state.goblin(receiver).is_some_and(|goblin| {
            goblin.team() == team && goblin.id != thrower && goblin.is_standing()
        });
        if !can_receive {
            return Err(RuleViolation::ReceiverUnavailable { receiver });
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
            let Some(Presence::Standing(from)) =
                self.state.goblin(id).map(|goblin| goblin.presence)
            else {
                return;
            };
            if !self.dodge_succeeds(id, from, events) {
                self.finish_turn(TurnEndReason::Knockdown, events);
                return;
            }
            let Some(goblin) = self.state.goblin_mut(id) else {
                return;
            };
            goblin.presence = Presence::Standing(to);
            events.push(GameEvent::Moved {
                goblin: id,
                from,
                to,
            });
            if self.state.ball == (Ball::Loose { at: to }) && !self.pick_succeeds(id, to, events) {
                self.finish_turn(TurnEndReason::Fumble, events);
                return;
            }
        }
        if let Some(goblin) = self.state.goblin_mut(id) {
            goblin.has_acted = true;
        }
        let turn_ending = match activation.action {
            None => None,
            Some(Action::Stance) => {
                self.take_stance(id, events);
                None
            }
            Some(Action::Assault { target }) => self.resolve_assault(id, target, events),
            Some(Action::Throw { receiver }) => {
                (!self.throw_completes(id, receiver, events)).then_some(TurnEndReason::Fumble)
            }
        };
        if let Some(reason) = turn_ending {
            self.finish_turn(reason, events);
            return;
        }
        if !self.has_goblin_able_to_act(team) {
            self.finish_turn(TurnEndReason::NoGoblinCanAct, events);
        }
    }

    fn pick_succeeds(&mut self, id: GoblinId, at: Square, events: &mut Vec<GameEvent>) -> bool {
        let roll = self.ball_roll(id, 0);
        events.push(GameEvent::PickAttempted { goblin: id, roll });
        if roll.success {
            self.state.ball = Ball::Carried { by: id };
            return true;
        }
        if let Some(picker) = self.state.goblin_mut(id) {
            picker.has_acted = true;
        }
        self.bounce_ball(at, events);
        false
    }

    fn dodge_succeeds(&mut self, id: GoblinId, from: Square, events: &mut Vec<GameEvent>) -> bool {
        let Some(tackler) = self.most_dangerous_tackler(id, from) else {
            return true;
        };
        let agility = self
            .state
            .goblin(id)
            .map_or(1, |dodger| dodger.characteristics.agility);
        let defense = self
            .state
            .goblin(tackler)
            .map_or(1, |tackler| tackler.characteristics.defense);
        let bonus = i8::try_from(self.state.stance_bonus(tackler)).unwrap_or(i8::MAX);
        let contest = self.roll_contest(agility, defense, bonus);
        events.push(GameEvent::DodgeAttempted {
            goblin: id,
            tackler,
            contest,
        });
        if contest.actor_won() {
            return true;
        }
        let damage = contest.margin.unsigned_abs().saturating_sub(1);
        if let Some(dodger) = self.state.goblin_mut(id) {
            dodger.has_acted = true;
        }
        self.knock_down(id, damage, events);
        false
    }
}

#[cfg(test)]
mod tests {
    use crate::characteristics::Archetype::{Bruiser, Sprinter, Thrower};
    use crate::command::{Action, Activation, Command};
    use crate::event::{BallRoll, ContestRoll, GameEvent, TurnEndReason};
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
            action: None,
        })
    }

    fn throw(goblin: &str, path: &[(u8, u8)], receiver: &str) -> Command {
        Command::Act(Activation {
            goblin: id(goblin),
            path: path.iter().map(|(x, y)| Square::new(*x, *y)).collect(),
            action: Some(Action::Throw {
                receiver: id(receiver),
            }),
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

    fn sprinter_slipping_past_a_bruiser() -> Scenario {
        Scenario::new()
            .goblin("H1", Sprinter, 5, 7)
            .goblin("H2", Sprinter, 3, 3)
            .goblin("A1", Bruiser, 6, 7)
    }

    #[test]
    fn given_sprinter_next_to_a_bruiser_when_it_wins_the_dodge_then_it_moves_on() {
        let mut game = sprinter_slipping_past_a_bruiser().game([3, 2]);
        game.apply(TeamId::HOME, act("H1", &[(4, 7), (3, 7)]))
            .unwrap();
        assert_eq!(
            game.state().goblin(id("H1")).unwrap().presence,
            Presence::Standing(Square::new(3, 7))
        );
    }

    #[test]
    fn given_sprinter_next_to_a_bruiser_when_the_dodge_ties_then_the_dodger_wins() {
        let mut game = sprinter_slipping_past_a_bruiser().game([2, 2]);
        game.apply(TeamId::HOME, act("H1", &[(4, 7)])).unwrap();
        assert_eq!(
            game.state().goblin(id("H1")).unwrap().presence,
            Presence::Standing(Square::new(4, 7))
        );
    }

    #[test]
    fn given_sprinter_next_to_a_bruiser_when_dodging_then_the_contest_is_reported() {
        let mut game = sprinter_slipping_past_a_bruiser().game([3, 2]);
        let events = game.apply(TeamId::HOME, act("H1", &[(4, 7)])).unwrap();
        assert_eq!(
            events.first(),
            Some(&GameEvent::DodgeAttempted {
                goblin: id("H1"),
                tackler: id("A1"),
                contest: ContestRoll {
                    actor_faces: 4,
                    actor_rolled: 3,
                    opponent_faces: 4,
                    opponent_rolled: 2,
                    opponent_modifier: 0,
                    margin: 1
                }
            })
        );
    }

    #[test]
    fn given_sprinter_loses_the_dodge_by_two_when_resolved_then_it_lies_where_it_stood() {
        let mut game = sprinter_slipping_past_a_bruiser().game([1, 3]);
        game.apply(TeamId::HOME, act("H1", &[(4, 7), (3, 7)]))
            .unwrap();
        assert_eq!(
            game.state().goblin(id("H1")).unwrap().presence,
            Presence::Down(Square::new(5, 7))
        );
    }

    #[test]
    fn given_sprinter_loses_the_dodge_by_two_when_resolved_then_it_loses_one_health() {
        let mut game = sprinter_slipping_past_a_bruiser().game([1, 3]);
        game.apply(TeamId::HOME, act("H1", &[(4, 7)])).unwrap();
        assert_eq!(game.state().goblin(id("H1")).unwrap().health, 2);
    }

    #[test]
    fn given_sprinter_loses_the_dodge_by_one_when_resolved_then_it_only_falls() {
        let mut game = sprinter_slipping_past_a_bruiser().game([2, 3]);
        let events = game.apply(TeamId::HOME, act("H1", &[(4, 7)])).unwrap();
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, GameEvent::HealthLost { .. }))
        );
    }

    #[test]
    fn given_sprinter_loses_the_dodge_when_resolved_then_the_turn_ends_on_a_knockdown() {
        let mut game = sprinter_slipping_past_a_bruiser().game([1, 3]);
        let events = game.apply(TeamId::HOME, act("H1", &[(4, 7)])).unwrap();
        assert!(events.contains(&GameEvent::TurnEnded {
            team: TeamId::HOME,
            reason: TurnEndReason::Knockdown
        }));
    }

    #[test]
    fn given_sprinter_crushed_by_a_braced_bruiser_when_resolved_then_it_is_knocked_out() {
        let mut game = sprinter_slipping_past_a_bruiser()
            .edit("A1", |goblin| goblin.in_stance = true)
            .game([1, 4]);
        game.apply(TeamId::HOME, act("H1", &[(4, 7)])).unwrap();
        assert_eq!(
            game.state().goblin(id("H1")).unwrap().presence,
            Presence::KnockedOut
        );
    }

    #[test]
    fn given_carrier_loses_the_dodge_when_resolved_then_the_ball_bounces_away() {
        let mut game = sprinter_slipping_past_a_bruiser()
            .ball_carried_by("H1")
            .game([1, 3, 1]);
        game.apply(TeamId::HOME, act("H1", &[(4, 7)])).unwrap();
        assert_eq!(
            game.state().ball,
            Ball::Loose {
                at: Square::new(5, 6)
            }
        );
    }

    #[test]
    fn given_a_sprinter_and_a_bruiser_around_when_dodging_then_the_bruiser_tackles() {
        let mut game = Scenario::new()
            .goblin("H1", Sprinter, 5, 7)
            .goblin("A1", Sprinter, 6, 7)
            .goblin("A2", Bruiser, 5, 6)
            .game([4, 1]);
        let events = game.apply(TeamId::HOME, act("H1", &[(4, 7)])).unwrap();
        assert!(matches!(
            events.first(),
            Some(GameEvent::DodgeAttempted { tackler, .. }) if *tackler == id("A2")
        ));
    }

    #[test]
    fn given_a_sprinter_inside_a_stance_wall_and_a_plain_bruiser_when_dodging_then_the_wall_tackles()
     {
        let mut game = Scenario::new()
            .goblin("H1", Sprinter, 5, 7)
            .goblin("A1", Sprinter, 6, 7)
            .goblin("A2", Bruiser, 5, 6)
            .goblin("A3", Bruiser, 7, 7)
            .goblin("A4", Bruiser, 6, 8)
            .edit("A1", |goblin| goblin.in_stance = true)
            .edit("A3", |goblin| goblin.in_stance = true)
            .edit("A4", |goblin| goblin.in_stance = true)
            .game([4, 1]);
        let events = game.apply(TeamId::HOME, act("H1", &[(4, 7)])).unwrap();
        assert!(matches!(
            events.first(),
            Some(GameEvent::DodgeAttempted { tackler, .. }) if *tackler == id("A1")
        ));
    }

    #[test]
    fn given_only_a_downed_enemy_next_to_the_goblin_when_stepping_away_then_no_dodge() {
        let mut game = sprinter_slipping_past_a_bruiser()
            .edit("A1", |goblin| {
                goblin.presence = Presence::Down(Square::new(6, 7))
            })
            .game([]);
        game.apply(TeamId::HOME, act("H1", &[(4, 7)])).unwrap();
        assert_eq!(game.dice().remaining(), 0);
    }

    #[test]
    fn given_a_free_square_when_stepping_next_to_an_enemy_then_no_dodge() {
        let mut game = Scenario::new()
            .goblin("H1", Sprinter, 4, 7)
            .goblin("A1", Bruiser, 6, 7)
            .game([]);
        let events = game.apply(TeamId::HOME, act("H1", &[(5, 7)])).unwrap();
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, GameEvent::DodgeAttempted { .. }))
        );
    }

    #[test]
    fn given_loose_ball_on_the_path_when_the_pick_succeeds_then_the_goblin_carries_it_on() {
        let mut game = two_sprinters_apart().ball_loose_at(4, 7).game([3]);
        game.apply(TeamId::HOME, act("H1", &[(4, 7), (5, 7)]))
            .unwrap();
        assert_eq!(game.state().ball, Ball::Carried { by: id("H1") });
    }

    #[test]
    fn given_loose_ball_on_the_path_when_picking_then_the_roll_is_reported() {
        let mut game = two_sprinters_apart().ball_loose_at(4, 7).game([3]);
        let events = game.apply(TeamId::HOME, act("H1", &[(4, 7)])).unwrap();
        assert_eq!(
            events.get(1),
            Some(&GameEvent::PickAttempted {
                goblin: id("H1"),
                roll: BallRoll {
                    faces: 4,
                    rolled: 3,
                    malus: 0,
                    target: 2,
                    success: true
                }
            })
        );
    }

    #[test]
    fn given_an_enemy_next_to_the_ball_when_picking_with_a_two_then_the_malus_makes_it_fail() {
        let mut game = two_sprinters_apart()
            .goblin("A2", Bruiser, 4, 6)
            .ball_loose_at(4, 7)
            .game([2, 3]);
        let events = game.apply(TeamId::HOME, act("H1", &[(4, 7)])).unwrap();
        assert!(events.contains(&GameEvent::PickAttempted {
            goblin: id("H1"),
            roll: BallRoll {
                faces: 4,
                rolled: 2,
                malus: 1,
                target: 2,
                success: false
            }
        }));
    }

    #[test]
    fn given_the_pick_fails_when_resolved_then_the_turn_ends_on_a_fumble() {
        let mut game = two_sprinters_apart().ball_loose_at(4, 7).game([1, 3]);
        let events = game
            .apply(TeamId::HOME, act("H1", &[(4, 7), (5, 7)]))
            .unwrap();
        assert!(events.contains(&GameEvent::TurnEnded {
            team: TeamId::HOME,
            reason: TurnEndReason::Fumble
        }));
    }

    #[test]
    fn given_the_pick_fails_when_resolved_then_the_goblin_stays_on_the_ball_square() {
        let mut game = two_sprinters_apart().ball_loose_at(4, 7).game([1, 3]);
        game.apply(TeamId::HOME, act("H1", &[(4, 7), (5, 7)]))
            .unwrap();
        assert_eq!(
            game.state().goblin(id("H1")).unwrap().presence,
            Presence::Standing(Square::new(4, 7))
        );
    }

    #[test]
    fn given_the_pick_fails_when_resolved_then_the_ball_bounces_off() {
        let mut game = two_sprinters_apart().ball_loose_at(4, 7).game([1, 3]);
        game.apply(TeamId::HOME, act("H1", &[(4, 7)])).unwrap();
        assert_eq!(
            game.state().ball,
            Ball::Loose {
                at: Square::new(4, 8)
            }
        );
    }

    #[test]
    fn given_the_pick_fails_when_the_ball_bounces_onto_a_teammate_then_it_may_catch_it() {
        let mut game = two_sprinters_apart()
            .goblin("H3", Sprinter, 4, 8)
            .ball_loose_at(4, 7)
            .game([1, 3, 4]);
        game.apply(TeamId::HOME, act("H1", &[(4, 7)])).unwrap();
        assert_eq!(game.state().ball, Ball::Carried { by: id("H3") });
    }

    fn thrower_with_the_ball() -> Scenario {
        Scenario::new()
            .goblin("H1", Thrower, 4, 7)
            .goblin("H2", Sprinter, 8, 7)
            .goblin("H3", Sprinter, 4, 11)
            .goblin("A1", Sprinter, 12, 3)
            .ball_carried_by("H1")
    }

    #[test]
    fn given_a_clear_line_when_throw_and_catch_succeed_then_the_receiver_holds_the_ball() {
        let mut game = thrower_with_the_ball().game([3, 3]);
        game.apply(TeamId::HOME, throw("H1", &[], "H2")).unwrap();
        assert_eq!(game.state().ball, Ball::Carried { by: id("H2") });
    }

    #[test]
    fn given_a_clear_line_when_throwing_then_the_throw_roll_is_reported() {
        let mut game = thrower_with_the_ball().game([3, 3]);
        let events = game.apply(TeamId::HOME, throw("H1", &[], "H2")).unwrap();
        assert_eq!(
            events.first(),
            Some(&GameEvent::ThrowAttempted {
                thrower: id("H1"),
                receiver: id("H2"),
                roll: BallRoll {
                    faces: 4,
                    rolled: 3,
                    malus: 0,
                    target: 2,
                    success: true
                }
            })
        );
    }

    #[test]
    fn given_the_throw_roll_fails_when_resolved_then_the_ball_bounces_from_the_thrower() {
        let mut game = thrower_with_the_ball().game([1, 1]);
        game.apply(TeamId::HOME, throw("H1", &[], "H2")).unwrap();
        assert_eq!(
            game.state().ball,
            Ball::Loose {
                at: Square::new(4, 6)
            }
        );
    }

    #[test]
    fn given_the_throw_roll_fails_when_resolved_then_the_turn_ends_on_a_fumble() {
        let mut game = thrower_with_the_ball().game([1, 1]);
        let events = game.apply(TeamId::HOME, throw("H1", &[], "H2")).unwrap();
        assert!(events.contains(&GameEvent::TurnEnded {
            team: TeamId::HOME,
            reason: TurnEndReason::Fumble
        }));
    }

    #[test]
    fn given_the_catch_fails_when_resolved_then_the_ball_bounces_from_the_receiver() {
        let mut game = thrower_with_the_ball().game([3, 1, 2]);
        game.apply(TeamId::HOME, throw("H1", &[], "H2")).unwrap();
        assert_eq!(
            game.state().ball,
            Ball::Loose {
                at: Square::new(9, 7)
            }
        );
    }

    #[test]
    fn given_the_catch_fails_when_resolved_then_the_turn_ends_on_a_fumble() {
        let mut game = thrower_with_the_ball().game([3, 1, 2]);
        let events = game.apply(TeamId::HOME, throw("H1", &[], "H2")).unwrap();
        assert!(events.contains(&GameEvent::TurnEnded {
            team: TeamId::HOME,
            reason: TurnEndReason::Fumble
        }));
    }

    #[test]
    fn given_a_receiver_nine_squares_away_when_throwing_then_three_malus_points() {
        let mut game = thrower_with_the_ball()
            .goblin("H4", Sprinter, 13, 7)
            .game([4, 1]);
        let events = game.apply(TeamId::HOME, throw("H1", &[], "H4")).unwrap();
        assert!(matches!(
            events.first(),
            Some(GameEvent::ThrowAttempted { roll, .. }) if roll.malus == 3 && !roll.success
        ));
    }

    #[test]
    fn given_an_opponent_on_the_line_when_it_intercepts_then_it_holds_the_ball() {
        let mut game = thrower_with_the_ball()
            .goblin("A2", Sprinter, 6, 7)
            .game([3, 4]);
        game.apply(TeamId::HOME, throw("H1", &[], "H2")).unwrap();
        assert_eq!(game.state().ball, Ball::Carried { by: id("A2") });
    }

    #[test]
    fn given_an_opponent_on_the_line_when_intercepting_then_the_surprise_malus_applies() {
        let mut game = thrower_with_the_ball()
            .goblin("A2", Sprinter, 6, 7)
            .game([3, 4]);
        let events = game.apply(TeamId::HOME, throw("H1", &[], "H2")).unwrap();
        assert_eq!(
            events.get(1),
            Some(&GameEvent::InterceptAttempted {
                goblin: id("A2"),
                roll: BallRoll {
                    faces: 4,
                    rolled: 4,
                    malus: 1,
                    target: 2,
                    success: true
                }
            })
        );
    }

    #[test]
    fn given_an_interception_when_resolved_then_the_turn_ends_on_a_fumble() {
        let mut game = thrower_with_the_ball()
            .goblin("A2", Sprinter, 6, 7)
            .game([3, 4]);
        let events = game.apply(TeamId::HOME, throw("H1", &[], "H2")).unwrap();
        assert!(events.contains(&GameEvent::TurnEnded {
            team: TeamId::HOME,
            reason: TurnEndReason::Fumble
        }));
    }

    #[test]
    fn given_an_opponent_on_the_line_when_its_interception_fails_then_the_receiver_catches() {
        let mut game = thrower_with_the_ball()
            .goblin("A2", Sprinter, 6, 7)
            .game([3, 2, 3]);
        game.apply(TeamId::HOME, throw("H1", &[], "H2")).unwrap();
        assert_eq!(game.state().ball, Ball::Carried { by: id("H2") });
    }

    #[test]
    fn given_a_downed_opponent_on_the_line_when_throwing_then_nobody_intercepts() {
        let mut game = thrower_with_the_ball()
            .goblin("A2", Sprinter, 6, 7)
            .edit("A2", |goblin| {
                goblin.presence = Presence::Down(Square::new(6, 7))
            })
            .game([3, 3]);
        game.apply(TeamId::HOME, throw("H1", &[], "H2")).unwrap();
        assert_eq!(game.state().ball, Ball::Carried { by: id("H2") });
    }

    #[test]
    fn given_the_ball_on_its_path_when_the_thrower_picks_it_up_then_it_can_throw() {
        let mut game = thrower_with_the_ball().ball_loose_at(5, 7).game([3, 3, 3]);
        game.apply(TeamId::HOME, throw("H1", &[(5, 7)], "H2"))
            .unwrap();
        assert_eq!(game.state().ball, Ball::Carried { by: id("H2") });
    }

    #[test]
    fn given_the_pick_on_the_path_fails_when_resolved_then_no_throw_happens() {
        let mut game = thrower_with_the_ball().ball_loose_at(5, 7).game([1, 3]);
        let events = game
            .apply(TeamId::HOME, throw("H1", &[(5, 7)], "H2"))
            .unwrap();
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, GameEvent::ThrowAttempted { .. }))
        );
    }

    #[test]
    fn given_a_thrower_without_the_ball_when_throwing_then_throw_without_ball() {
        let mut game = thrower_with_the_ball().ball_loose_at(10, 10).game([]);
        assert_eq!(
            game.apply(TeamId::HOME, throw("H1", &[], "H2")),
            Err(RuleViolation::ThrowWithoutBall { goblin: id("H1") })
        );
    }

    #[test]
    fn given_an_opponent_as_receiver_when_throwing_then_receiver_unavailable() {
        let mut game = thrower_with_the_ball().game([]);
        assert_eq!(
            game.apply(TeamId::HOME, throw("H1", &[], "A1")),
            Err(RuleViolation::ReceiverUnavailable { receiver: id("A1") })
        );
    }

    #[test]
    fn given_a_downed_teammate_as_receiver_when_throwing_then_receiver_unavailable() {
        let mut game = thrower_with_the_ball()
            .edit("H2", |goblin| {
                goblin.presence = Presence::Down(Square::new(8, 7))
            })
            .game([]);
        assert_eq!(
            game.apply(TeamId::HOME, throw("H1", &[], "H2")),
            Err(RuleViolation::ReceiverUnavailable { receiver: id("H2") })
        );
    }

    #[test]
    fn given_the_thrower_as_its_own_receiver_when_throwing_then_receiver_unavailable() {
        let mut game = thrower_with_the_ball().game([]);
        assert_eq!(
            game.apply(TeamId::HOME, throw("H1", &[], "H1")),
            Err(RuleViolation::ReceiverUnavailable { receiver: id("H1") })
        );
    }
}
