//! Throws: the throw roll, interceptions along the flight line, and the receiver's catch.

use crate::dice::DiceRoller;
use crate::event::GameEvent;
use crate::flight::squares_crossed;
use crate::roster::GoblinId;
use crate::state::Ball;

use super::Game;

impl<D: DiceRoller> Game<D> {
    pub(super) fn throw_completes(
        &mut self,
        thrower: GoblinId,
        receiver: GoblinId,
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
        let distance_malus = distance.saturating_sub(self.ruleset.throw_free_range);
        let roll = self.ball_roll(thrower, distance_malus);
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
        for square in squares_crossed(thrower_square, receiver_square) {
            let interceptor = self
                .state
                .goblin_at(square)
                .filter(|goblin| goblin.is_standing() && goblin.team() != team)
                .map(|goblin| goblin.id);
            if let Some(interceptor) = interceptor {
                let roll = self.ball_roll(interceptor, self.ruleset.interception_surprise_malus);
                events.push(GameEvent::InterceptAttempted {
                    goblin: interceptor,
                    roll,
                });
                if roll.success {
                    self.state.ball = Ball::Carried { by: interceptor };
                    return false;
                }
            }
        }
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
