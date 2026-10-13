//! Contests and their consequences: opposed rolls, knockdowns, health loss and knock-outs.

use crate::dice::DiceRoller;
use crate::event::{ContestRoll, GameEvent, TurnEndReason};
use crate::odds::ContestDice;
use crate::preview::{assault_dice, most_dangerous_tackler, stance_bonus};
use crate::roster::GoblinId;
use crate::square::Square;
use crate::state::{Ball, Presence};

use super::Game;

impl<D: DiceRoller> Game<D> {
    pub(super) fn roll_contest(
        &mut self,
        actor_faces: u8,
        opponent_faces: u8,
        opponent_modifier: i8,
    ) -> ContestRoll {
        let actor_rolled = self.dice.roll(actor_faces);
        let opponent_rolled = self.dice.roll(opponent_faces);
        let margin =
            i16::from(actor_rolled) - (i16::from(opponent_rolled) + i16::from(opponent_modifier));
        ContestRoll {
            actor_faces,
            actor_rolled,
            opponent_faces,
            opponent_rolled,
            opponent_modifier,
            margin: i8::try_from(margin).unwrap_or(if margin < 0 { i8::MIN } else { i8::MAX }),
        }
    }

    pub(super) fn most_dangerous_tackler(
        &self,
        dodger: GoblinId,
        from: Square,
    ) -> Option<GoblinId> {
        most_dangerous_tackler(&self.ruleset, &self.state, dodger, from).map(|(tackler, _)| tackler)
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
            events.push(GameEvent::KnockedOut { goblin });
        }
    }

    pub(super) fn take_stance(&mut self, goblin: GoblinId, events: &mut Vec<GameEvent>) {
        if let Some(braced) = self.state.goblin_mut(goblin) {
            braced.in_stance = true;
            events.push(GameEvent::StanceTaken { goblin });
        }
    }

    pub(super) fn resolve_assault(
        &mut self,
        attacker: GoblinId,
        defender: GoblinId,
        events: &mut Vec<GameEvent>,
    ) -> Option<TurnEndReason> {
        let defender_is_down = self
            .state
            .goblin(defender)
            .is_some_and(|goblin| matches!(goblin.presence, Presence::Down(_)));
        let dice =
            assault_dice(&self.ruleset, &self.state, attacker, defender).unwrap_or(ContestDice {
                actor_faces: 1,
                opponent_faces: 1,
                opponent_modifier: 0,
            });
        let contest = self.roll_contest(
            dice.actor_faces,
            dice.opponent_faces,
            dice.opponent_modifier,
        );
        let defender_braced = stance_bonus(&self.ruleset, &self.state, defender) > 0;
        events.push(GameEvent::AssaultResolved {
            attacker,
            defender,
            contest,
            defender_braced,
        });
        if !contest.actor_won() {
            let damage = contest.margin.unsigned_abs().saturating_sub(1);
            self.knock_down(attacker, damage, events);
            return Some(TurnEndReason::Knockdown);
        }
        let damage = contest.margin.unsigned_abs();
        let missed_catches = if defender_is_down {
            self.lose_health(defender, damage, events);
            Vec::new()
        } else {
            self.knock_down(defender, damage, events)
        };
        let active_team = self.state.active_team();
        missed_catches
            .iter()
            .any(|goblin| Some(goblin.team) == active_team)
            .then_some(TurnEndReason::Fumble)
    }
}
