//! Contests and their consequences: opposed rolls, knockdowns, health loss and knock-outs.

use crate::dice::DiceRoller;
use crate::event::{ContestRoll, GameEvent};
use crate::odds::contest_success;
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
        let dodger = self.state.goblin(dodger)?;
        let agility = dodger.characteristics.agility;
        self.state
            .standing_opponents_next_to(from, dodger.team())
            .map(|tackler| {
                let bonus = i8::try_from(self.state.stance_bonus(tackler.id)).unwrap_or(i8::MAX);
                let escape = contest_success(agility, tackler.characteristics.defense, bonus);
                (escape, tackler.id)
            })
            .min()
            .map(|(_, tackler)| tackler)
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
}
