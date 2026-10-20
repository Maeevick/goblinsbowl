//! Contests and their consequences: opposed rolls, knockdowns, health loss and knock-outs.

use crate::command::AssaultStyle;
use crate::dice::DiceRoller;
use crate::event::{ContestRoll, GameEvent, TurnEndReason};
use crate::odds::ContestDice;
use crate::preview::{stance_bonus, styled_assault_dice};
use crate::roster::GoblinId;
use crate::square::{Compass, Square};
use crate::state::{Ball, Focus, Presence};
use crate::validation::projection_landing;

use super::Game;

impl<D: DiceRoller> Game<D> {
    pub(super) fn roll_contest(&mut self, dice: ContestDice) -> ContestRoll {
        let actor_rolled = self.dice.roll(dice.actor_faces);
        let opponent_rolled = self.dice.roll(dice.opponent_faces);
        let margin = i16::from(actor_rolled) + i16::from(dice.actor_modifier)
            - (i16::from(opponent_rolled) + i16::from(dice.opponent_modifier));
        ContestRoll {
            actor_faces: dice.actor_faces,
            actor_rolled,
            actor_modifier: dice.actor_modifier,
            opponent_faces: dice.opponent_faces,
            opponent_rolled,
            opponent_modifier: dice.opponent_modifier,
            margin: i8::try_from(margin).unwrap_or(if margin < 0 { i8::MIN } else { i8::MAX }),
        }
    }

    pub(super) fn knock_down(
        &mut self,
        goblin: GoblinId,
        damage: u8,
        events: &mut Vec<GameEvent>,
    ) -> Vec<GoblinId> {
        let Some(square) = self.state.goblin(goblin).and_then(|fallen| fallen.square()) else {
            return Vec::new();
        };
        if let Some(fallen) = self.state.goblin_mut(goblin) {
            fallen.presence = Presence::Down(square);
            fallen.in_stance = false;
            fallen.focus = Focus::Unfocused;
        }
        events.push(GameEvent::KnockedDown { goblin, at: square });
        self.lose_health(goblin, damage, events);
        if self.state.carrier() == Some(goblin) {
            self.state.ball = Ball::Loose { at: square };
            return self.bounce_ball(square, events);
        }
        Vec::new()
    }

    pub(super) fn lose_health(
        &mut self,
        goblin: GoblinId,
        amount: u8,
        events: &mut Vec<GameEvent>,
    ) {
        if amount == 0 {
            return;
        }
        let Some(hurt) = self.state.goblin_mut(goblin) else {
            return;
        };
        hurt.health = hurt.health.saturating_sub(amount);
        let remaining = hurt.health;
        events.push(GameEvent::HealthLost {
            goblin,
            amount,
            remaining,
        });
        if remaining == 0 {
            hurt.presence = Presence::KnockedOut;
            hurt.in_stance = false;
            hurt.focus = Focus::Unfocused;
            events.push(GameEvent::KnockedOut { goblin });
        }
    }

    pub(super) fn take_focus(&mut self, goblin: GoblinId, events: &mut Vec<GameEvent>) {
        if let Some(focusing) = self.state.goblin_mut(goblin) {
            focusing.focus = Focus::Focusing;
            events.push(GameEvent::FocusTaken { goblin });
        }
    }

    pub(super) fn take_stance(&mut self, goblin: GoblinId, events: &mut Vec<GameEvent>) {
        if let Some(braced) = self.state.goblin_mut(goblin) {
            braced.in_stance = true;
            events.push(GameEvent::StanceTaken { goblin });
        }
    }

    /// Moves `defender` onto the square `attacker` projects it to, and returns that square when the
    /// loose ball lay there, to bounce once the defender is down.
    fn project(
        &mut self,
        attacker: GoblinId,
        defender: GoblinId,
        direction: Compass,
        events: &mut Vec<GameEvent>,
    ) -> Option<Square> {
        let attacker_square = self
            .state
            .goblin(attacker)
            .and_then(|goblin| goblin.square())?;
        let from = self
            .state
            .goblin(defender)
            .and_then(|goblin| goblin.square())?;
        let landing = projection_landing(
            &self.arena,
            &self.state,
            attacker,
            attacker_square,
            defender,
            direction,
        )?;
        if landing != from {
            if let Some(projected) = self.state.goblin_mut(defender) {
                projected.presence = Presence::Standing(landing);
            }
            events.push(GameEvent::Projected {
                goblin: defender,
                from,
                to: landing,
            });
        }
        (self.state.ball == Ball::Loose { at: landing }).then_some(landing)
    }

    pub(super) fn resolve_assault(
        &mut self,
        attacker: GoblinId,
        defender: GoblinId,
        style: AssaultStyle,
        events: &mut Vec<GameEvent>,
    ) -> Option<TurnEndReason> {
        let defender_is_down = self
            .state
            .goblin(defender)
            .is_some_and(|goblin| matches!(goblin.presence, Presence::Down(_)));
        let dice = styled_assault_dice(&self.ruleset, &self.state, attacker, defender, style)
            .unwrap_or(ContestDice {
                actor_modifier: 0,
                actor_faces: 1,
                opponent_faces: 1,
                opponent_modifier: 0,
            });
        let contest = self.roll_contest(dice);
        let defender_braced = stance_bonus(&self.ruleset, &self.state, defender) > 0;
        events.push(GameEvent::AssaultResolved {
            attacker,
            defender,
            contest,
            defender_braced,
            style,
        });
        if contest.margin == 0 && !self.ruleset.assault_ties_knock_down {
            events.push(GameEvent::StandOff { attacker, defender });
            return None;
        }
        let margin_as_rolled = contest.margin.saturating_sub(contest.actor_modifier);
        if !contest.actor_won() {
            let damage = margin_as_rolled.unsigned_abs().saturating_sub(1);
            self.knock_down(attacker, damage, events);
            return Some(TurnEndReason::Knockdown);
        }
        let damage = style.damage(margin_as_rolled, &self.ruleset);
        let landed_on_the_ball = match style {
            AssaultStyle::Projection(direction) => {
                self.project(attacker, defender, direction, events)
            }
            AssaultStyle::Simple | AssaultStyle::Chirurgical | AssaultStyle::Brutal => None,
        };
        let mut missed_catches = if defender_is_down {
            self.lose_health(defender, damage, events);
            Vec::new()
        } else {
            self.knock_down(defender, damage, events)
        };
        if let Some(landing) = landed_on_the_ball {
            missed_catches.extend(self.bounce_ball(landing, events));
        }
        let active_team = self.state.active_team();
        let a_teammate_missed = missed_catches
            .iter()
            .any(|goblin| Some(goblin.team) == active_team);
        (self.ruleset.missed_bounces_end_the_turn && a_teammate_missed)
            .then_some(TurnEndReason::Fumble)
    }
}
