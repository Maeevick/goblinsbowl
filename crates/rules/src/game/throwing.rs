//! Throws: the throw roll, interceptions along the flight line, and the receiver's catch.

use crate::bounces::CHAOS_BOUNCES;
use crate::dice::DiceRoller;
use crate::event::{GameEvent, TurnEndReason};
use crate::preview::interceptors;
use crate::roster::GoblinId;
use crate::square::Square;
use crate::state::Ball;

use super::Game;

impl<D: DiceRoller> Game<D> {
    pub(super) fn throw_completes(
        &mut self,
        thrower: GoblinId,
        receiver: GoblinId,
        steps_sacrificed: u8,
        events: &mut Vec<GameEvent>,
    ) -> bool {
        let Some((thrower_square, team)) = self
            .state
            .goblin(thrower)
            .and_then(|goblin| goblin.square().map(|square| (square, goblin.team())))
        else {
            return false;
        };
        let Some(receiver_square) = self
            .state
            .goblin(receiver)
            .and_then(|goblin| goblin.square())
        else {
            return false;
        };
        let distance = thrower_square.orthogonal_distance(receiver_square);
        if self.ruleset.adjacent_throws_are_hand_offs && distance == 1 {
            events.push(GameEvent::HandedOff {
                giver: thrower,
                receiver,
            });
            return self.receiver_catches(receiver, receiver_square, events);
        }
        let distance_malus = distance.saturating_sub(self.ruleset.throw_free_range);
        let focus_bonus = self
            .state
            .goblin(thrower)
            .map_or(0, |goblin| goblin.focus.throw_bonus())
            .saturating_add(steps_sacrificed);
        let roll = self.agility_roll_with_bonus(thrower, distance_malus, focus_bonus);
        events.push(GameEvent::ThrowAttempted {
            thrower,
            receiver,
            roll,
            distance,
        });
        if !roll.success {
            self.state.ball = Ball::Loose { at: thrower_square };
            self.bounce_ball(thrower_square, events);
            return false;
        }
        for interceptor in interceptors(
            &self.ruleset,
            &self.state,
            thrower_square,
            receiver_square,
            team,
        ) {
            let roll = self.agility_roll(interceptor, self.ruleset.interception_surprise_malus);
            events.push(GameEvent::InterceptAttempted {
                goblin: interceptor,
                roll,
            });
            if roll.success {
                self.state.ball = Ball::Carried { by: interceptor };
                return false;
            }
        }
        self.receiver_catches(receiver, receiver_square, events)
    }

    /// Throws the ball into chaos towards `receiver`: no throw roll; enemies near the line may
    /// intercept, which ends the turn; otherwise the ball bounces three times from the receiver's
    /// square, and the turn ends on a fumble unless a teammate holds it by then.
    pub(super) fn throw_into_chaos(
        &mut self,
        thrower: GoblinId,
        receiver: GoblinId,
        events: &mut Vec<GameEvent>,
    ) -> Option<TurnEndReason> {
        let thrower_square = self.state.goblin(thrower)?.square()?;
        let team = thrower.team;
        let receiver_square = self.state.goblin(receiver)?.square()?;
        events.push(GameEvent::ChaosThrown { thrower, receiver });
        for interceptor in interceptors(
            &self.ruleset,
            &self.state,
            thrower_square,
            receiver_square,
            team,
        ) {
            let roll = self.agility_roll(interceptor, self.ruleset.interception_surprise_malus);
            events.push(GameEvent::InterceptAttempted {
                goblin: interceptor,
                roll,
            });
            if roll.success {
                self.state.ball = Ball::Carried { by: interceptor };
                return Some(TurnEndReason::Fumble);
            }
        }
        let mut from = receiver_square;
        for _ in 0..CHAOS_BOUNCES {
            self.bounce_ball(from, events);
            match self.state.ball {
                Ball::Loose { at } => from = at,
                Ball::Carried { .. } => break,
            }
        }
        let a_teammate_holds_it = self
            .state
            .carrier()
            .is_some_and(|carrier| carrier.team == team);
        (!a_teammate_holds_it).then_some(TurnEndReason::Fumble)
    }

    fn receiver_catches(
        &mut self,
        receiver: GoblinId,
        receiver_square: Square,
        events: &mut Vec<GameEvent>,
    ) -> bool {
        if self.attempt_catch(receiver, 0, events) {
            return true;
        }
        self.state.ball = Ball::Loose {
            at: receiver_square,
        };
        self.bounce_ball(receiver_square, events);
        false
    }
}
