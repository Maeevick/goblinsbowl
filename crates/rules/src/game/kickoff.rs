//! Kick-offs, bounces and catches: how the ball comes back into play and moves on its own.

use std::collections::VecDeque;

use crate::dice::DiceRoller;
use crate::event::{AgilityRoll, GameEvent};
use crate::formation::{kickoff_drop_square, kickoff_squares};
use crate::odds::AgilityDice;
use crate::roster::{GoblinId, Roster};
use crate::square::{Direction, Square};
use crate::state::{Ball, Goblin, Presence};
use crate::team::TeamId;

use super::Game;

const MAX_DIRECTION_ROLLS: usize = 32;
const MAX_BOUNCES: usize = 32;

impl<D: DiceRoller> Game<D> {
    pub(super) fn kick_off(&mut self, receiving_team: TeamId, events: &mut Vec<GameEvent>) {
        events.push(GameEvent::KickedOff { receiving_team });
        for team in TeamId::DUEL {
            let heals = self.ruleset.goblins_heal_at_kick_off;
            let returning = if !heals && self.ruleset.knocked_out_goblins_may_return {
                self.roll_returns_from_knock_out(team, events)
            } else {
                Vec::new()
            };
            let lining_up: Vec<&Goblin> = self
                .state
                .goblins_of(team)
                .filter(|goblin| {
                    heals
                        || goblin.presence != Presence::KnockedOut
                        || returning.contains(&goblin.id)
                })
                .collect();
            let roster = if lining_up.len() == self.state.goblins_of(team).count() {
                self.state.rosters[team.index()]
            } else {
                Roster::of(lining_up.iter().map(|goblin| goblin.archetype))
            };
            let ids: Vec<GoblinId> = lining_up.iter().map(|goblin| goblin.id).collect();
            let lineup = kickoff_squares(&self.arena, &self.ruleset.formation, team, &roster);
            for (id, square) in ids.into_iter().zip(lineup) {
                self.line_up(id, square, events);
            }
        }
        let drop = self.kickoff_drop(receiving_team);
        self.state.ball = Ball::Loose { at: drop };
        events.push(GameEvent::BallDropped { at: drop });
        for _ in 0..self.ruleset.kickoff_bounces {
            let Ball::Loose { at } = self.state.ball else {
                break;
            };
            self.bounce_ball(at, events);
        }
    }

    /// Each knocked-out goblin of `team` rolls its Health die minus one: the ones above 0 come back
    /// with that much Health, the others stay out.
    fn roll_returns_from_knock_out(
        &mut self,
        team: TeamId,
        events: &mut Vec<GameEvent>,
    ) -> Vec<GoblinId> {
        let knocked_out: Vec<(GoblinId, u8)> = self
            .state
            .goblins_of(team)
            .filter(|goblin| goblin.presence == Presence::KnockedOut)
            .map(|goblin| (goblin.id, goblin.characteristics.health))
            .collect();
        knocked_out
            .into_iter()
            .filter_map(|(goblin, full_health)| {
                let health = self.dice.roll(full_health).saturating_sub(1);
                if health == 0 {
                    events.push(GameEvent::StillKnockedOut { goblin });
                    return None;
                }
                if let Some(back) = self.state.goblin_mut(goblin) {
                    back.health = health;
                }
                events.push(GameEvent::ReturnedFromKnockOut { goblin, health });
                Some(goblin)
            })
            .collect()
    }

    fn kickoff_drop(&mut self, receiving_team: TeamId) -> Square {
        let depth = self
            .ruleset
            .kickoff_depth
            .saturating_add(self.roll_from_zero(self.ruleset.kickoff_depth_spread));
        let across = self.roll_around_zero(self.ruleset.kickoff_across_spread);
        kickoff_drop_square(&self.arena, receiving_team, depth, across)
            .unwrap_or_else(|| self.arena.centre())
    }

    fn roll_from_zero(&mut self, spread: u8) -> u8 {
        if spread == 0 {
            return 0;
        }
        self.dice.roll(spread.saturating_add(1)).saturating_sub(1)
    }

    fn roll_around_zero(&mut self, spread: u8) -> i8 {
        if spread == 0 {
            return 0;
        }
        let faces = spread.saturating_mul(2).saturating_add(1);
        let face = i16::from(self.dice.roll(faces));
        i8::try_from(face - i16::from(spread) - 1).unwrap_or(0)
    }

    fn line_up(&mut self, id: GoblinId, square: Square, events: &mut Vec<GameEvent>) {
        let Some(goblin) = self.state.goblin(id) else {
            return;
        };
        let full_health = goblin.characteristics.health;
        let health = if !self.ruleset.goblins_heal_at_kick_off {
            goblin.health
        } else if goblin.presence == Presence::KnockedOut {
            let health = self.dice.roll(full_health);
            events.push(GameEvent::ReturnedFromKnockOut { goblin: id, health });
            health
        } else {
            full_health
        };
        if let Some(goblin) = self.state.goblin_mut(id) {
            goblin.health = health;
            goblin.presence = Presence::Standing(square);
            goblin.in_stance = false;
            goblin.refresh_its_turn();
        }
    }

    pub(super) fn bounce_ball(
        &mut self,
        origin: Square,
        events: &mut Vec<GameEvent>,
    ) -> Vec<GoblinId> {
        let mut from = origin;
        let mut missed_catches = Vec::new();
        for _ in 0..MAX_BOUNCES {
            let to = self.roll_bounce_destination(from);
            events.push(GameEvent::BallBounced { from, to });
            self.state.ball = Ball::Loose { at: to };
            match self
                .state
                .goblin_at(to)
                .map(|goblin| (goblin.id, goblin.is_standing()))
            {
                None => return missed_catches,
                Some((catcher, true)) => {
                    if self.attempt_catch(catcher, 0, events) {
                        return missed_catches;
                    }
                    missed_catches.push(catcher);
                }
                Some((_, false)) => {}
            }
            from = to;
        }
        if let Some(resting_square) = self.nearest_empty_square(from) {
            self.state.ball = Ball::Loose { at: resting_square };
        }
        missed_catches
    }

    fn roll_bounce_destination(&mut self, from: Square) -> Square {
        for _ in 0..MAX_DIRECTION_ROLLS {
            let face = self.dice.roll(4);
            let destination = Direction::from_d4_face(face)
                .and_then(|direction| direction.step_from(from))
                .filter(|square| self.arena.is_in_bounds(*square));
            if let Some(destination) = destination {
                return destination;
            }
        }
        self.arena.in_bounds_neighbours(from).next().unwrap_or(from)
    }

    pub(super) fn attempt_catch(
        &mut self,
        catcher: GoblinId,
        extra_malus: u8,
        events: &mut Vec<GameEvent>,
    ) -> bool {
        let roll = self.agility_roll(catcher, extra_malus);
        events.push(GameEvent::CatchAttempted {
            goblin: catcher,
            roll,
        });
        if roll.success {
            self.state.ball = Ball::Carried { by: catcher };
        }
        roll.success
    }

    pub(super) fn agility_roll(&mut self, goblin: GoblinId, extra_malus: u8) -> AgilityRoll {
        let reach = self.ruleset.reach();
        let (faces, opponents_in_reach) = self
            .state
            .goblin(goblin)
            .and_then(|goblin| {
                goblin.square().map(|square| {
                    (
                        goblin.characteristics.agility,
                        self.state
                            .standing_opponents_in_reach(square, goblin.team(), reach)
                            .count(),
                    )
                })
            })
            .unwrap_or((1, 0));
        let malus = u8::try_from(opponents_in_reach)
            .unwrap_or(u8::MAX)
            .saturating_add(extra_malus);
        self.roll_agility_test(AgilityDice {
            faces,
            malus,
            target: self.ruleset.ball_roll_target,
        })
    }

    pub(super) fn roll_agility_test(&mut self, dice: AgilityDice) -> AgilityRoll {
        let rolled = self.dice.roll(dice.faces);
        AgilityRoll {
            faces: dice.faces,
            rolled,
            malus: dice.malus,
            target: dice.target,
            success: i16::from(rolled) - i16::from(dice.malus) >= i16::from(dice.target),
        }
    }

    fn nearest_empty_square(&self, origin: Square) -> Option<Square> {
        let mut visited = vec![origin];
        let mut frontier = VecDeque::from([origin]);
        while let Some(square) = frontier.pop_front() {
            if self.state.goblin_at(square).is_none() {
                return Some(square);
            }
            for neighbour in self.arena.in_bounds_neighbours(square) {
                if !visited.contains(&neighbour) {
                    visited.push(neighbour);
                    frontier.push_back(neighbour);
                }
            }
        }
        None
    }
}
