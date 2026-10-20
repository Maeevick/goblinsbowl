//! Kick-offs, bounces and catches: how the ball comes back into play and moves on its own.

use std::collections::VecDeque;

use crate::bounces::{
    MOST_DIRECTION_ROLLS, MOST_DIVE_TIE_BREAKS, MOST_MOVES_IN_A_BOUNCE, direction_die, landing,
    settling_square,
};
use crate::dice::DiceRoller;
use crate::dives::divers_around;
use crate::event::{AgilityRoll, DiveRoll, GameEvent};
use crate::formation::{kickoff_drop_square, kickoff_squares};
use crate::odds::AgilityDice;
use crate::roster::{GoblinId, Roster};
use crate::square::Square;
use crate::state::{Ball, Focus, Goblin, Presence};
use crate::team::TeamId;

use super::Game;

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
            goblin.focus = Focus::Unfocused;
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
        for _ in 0..MOST_MOVES_IN_A_BOUNCE {
            let to = self.roll_bounce_destination(from);
            events.push(GameEvent::BallBounced { from, to });
            self.state.ball = Ball::Loose { at: to };
            match self
                .state
                .goblin_at(to)
                .map(|goblin| (goblin.id, goblin.is_standing()))
            {
                None => match self.dive_for_the_ball(to, events) {
                    Some((diver, false)) => missed_catches.push(diver),
                    _ => return missed_catches,
                },
                Some((catcher, true)) => {
                    if self.attempt_catch(catcher, 0, 0, events) {
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

    /// Lets the standing Sprinters next to `at`, the empty square where the ball just landed, dive
    /// for it where the rules have diving catches: the one rolling highest lands there holding it,
    /// or falls where it stands. Tells who dived and whether it caught the ball.
    fn dive_for_the_ball(
        &mut self,
        at: Square,
        events: &mut Vec<GameEvent>,
    ) -> Option<(GoblinId, bool)> {
        let candidates = divers_around(&self.ruleset, &self.state, at);
        let DiveRoll {
            goblin: diver,
            roll,
        } = self.contest_the_dive(&candidates, events)?;
        let from = self.state.goblin(diver)?.square()?;
        events.push(GameEvent::DiveAttempted {
            goblin: diver,
            from,
            to: at,
            roll,
        });
        if roll.success {
            if let Some(goblin) = self.state.goblin_mut(diver) {
                goblin.presence = Presence::Standing(at);
            }
            self.state.ball = Ball::Carried { by: diver };
        } else {
            self.knock_down(diver, 0, events);
        }
        Some((diver, roll.success))
    }

    /// Who dives among `candidates`, with the roll that decides its catch: each rolls its diving
    /// catch, the highest result dives, and those tied roll again among themselves only to tell
    /// who.
    fn contest_the_dive(
        &mut self,
        candidates: &[GoblinId],
        events: &mut Vec<GameEvent>,
    ) -> Option<DiveRoll> {
        let first_rolls = self.roll_dives(candidates);
        if first_rolls.len() > 1 {
            events.push(GameEvent::DiveContested {
                rolls: first_rolls.clone(),
            });
        }
        let mut tied = highest_dives(&first_rolls);
        for _ in 0..MOST_DIVE_TIE_BREAKS {
            if tied.len() < 2 {
                break;
            }
            let rerolls = self.roll_dives(&tied);
            events.push(GameEvent::DiveContested {
                rolls: rerolls.clone(),
            });
            tied = highest_dives(&rerolls);
        }
        let diver = tied.first()?;
        first_rolls.into_iter().find(|dive| dive.goblin == *diver)
    }

    fn roll_dives(&mut self, divers: &[GoblinId]) -> Vec<DiveRoll> {
        let malus = self.ruleset.diving_catch_malus;
        divers
            .iter()
            .map(|diver| DiveRoll {
                goblin: *diver,
                roll: self.agility_roll(*diver, malus),
            })
            .collect()
    }

    fn roll_bounce_destination(&mut self, from: Square) -> Square {
        let faces = direction_die(&self.ruleset);
        for _ in 0..MOST_DIRECTION_ROLLS {
            let face = self.dice.roll(faces);
            if let Some(destination) = landing(&self.arena, &self.ruleset, from, face) {
                return destination;
            }
        }
        settling_square(&self.arena, from)
    }

    pub(super) fn attempt_catch(
        &mut self,
        catcher: GoblinId,
        extra_malus: u8,
        bonus: u8,
        events: &mut Vec<GameEvent>,
    ) -> bool {
        let roll = self.agility_roll_with_bonus(catcher, extra_malus, bonus);
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
        self.agility_roll_with_bonus(goblin, extra_malus, 0)
    }

    pub(super) fn agility_roll_with_bonus(
        &mut self,
        goblin: GoblinId,
        extra_malus: u8,
        bonus: u8,
    ) -> AgilityRoll {
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
            bonus,
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
            bonus: dice.bonus,
            target: dice.target,
            success: i16::from(rolled) - i16::from(dice.malus) + i16::from(dice.bonus)
                >= i16::from(dice.target),
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

/// The divers whose result is the highest among `dives`.
fn highest_dives(dives: &[DiveRoll]) -> Vec<GoblinId> {
    let result = |dive: &DiveRoll| {
        i16::from(dive.roll.rolled) + i16::from(dive.roll.bonus) - i16::from(dive.roll.malus)
    };
    let best = dives.iter().map(result).max();
    dives
        .iter()
        .filter(|dive| Some(result(dive)) == best)
        .map(|dive| dive.goblin)
        .collect()
}
