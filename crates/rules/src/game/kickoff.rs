//! Kick-offs, bounces and catches: how the ball comes back into play and moves on its own.

use std::collections::VecDeque;

use crate::dice::DiceRoller;
use crate::event::{AgilityRoll, GameEvent};
use crate::formation::kickoff_squares;
use crate::roster::GoblinId;
use crate::square::{Direction, Square};
use crate::state::{Ball, Presence};
use crate::team::TeamId;

use super::Game;

const MAX_DIRECTION_ROLLS: usize = 32;
const MAX_BOUNCES: usize = 32;

impl<D: DiceRoller> Game<D> {
    pub(super) fn kick_off(&mut self, receiving_team: TeamId, events: &mut Vec<GameEvent>) {
        events.push(GameEvent::KickedOff { receiving_team });
        for team in TeamId::DUEL {
            let roster = self.state.rosters[team.index()];
            let lineup = kickoff_squares(&self.arena, &self.ruleset.formation, team, &roster);
            let ids: Vec<GoblinId> = self
                .state
                .goblins_of(team)
                .map(|goblin| goblin.id)
                .collect();
            for (id, square) in ids.into_iter().zip(lineup) {
                self.line_up(id, square, events);
            }
        }
        let drop = self.kickoff_drop(receiving_team);
        self.state.ball = Ball::Loose { at: drop };
        events.push(GameEvent::BallDropped { at: drop });
        self.bounce_ball(drop, events);
    }

    fn kickoff_drop(&self, receiving_team: TeamId) -> Square {
        let den_distance = |square: Square| {
            self.arena
                .den_squares(receiving_team)
                .map(|den| den.orthogonal_distance(square))
                .min()
        };
        let mut drop = self.arena.centre();
        for _ in 0..self.ruleset.kickoff_depth {
            let deeper = Direction::ALL
                .into_iter()
                .filter_map(|direction| direction.step_from(drop))
                .filter(|square| self.arena.is_in_bounds(*square))
                .min_by_key(|square| den_distance(*square));
            match deeper {
                Some(square) if den_distance(square) < den_distance(drop) => drop = square,
                _ => break,
            }
        }
        drop
    }

    fn line_up(&mut self, id: GoblinId, square: Square, events: &mut Vec<GameEvent>) {
        let Some(goblin) = self.state.goblin(id) else {
            return;
        };
        let full_health = goblin.characteristics.health;
        let health = if goblin.presence == Presence::KnockedOut {
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
        let (faces, adjacent_opponents) = self
            .state
            .goblin(goblin)
            .and_then(|goblin| {
                goblin.square().map(|square| {
                    (
                        goblin.characteristics.agility,
                        self.state
                            .standing_opponents_next_to(square, goblin.team())
                            .count(),
                    )
                })
            })
            .unwrap_or((1, 0));
        let malus = u8::try_from(adjacent_opponents)
            .unwrap_or(u8::MAX)
            .saturating_add(extra_malus);
        let rolled = self.dice.roll(faces);
        let target = self.ruleset.ball_roll_target;
        AgilityRoll {
            faces,
            rolled,
            malus,
            target,
            success: i16::from(rolled) - i16::from(malus) >= i16::from(target),
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
