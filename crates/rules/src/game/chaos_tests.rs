//! Chaos throws: no throw roll, the ball bouncing three times from the receiver's square.

use crate::characteristics::Archetype::{Sprinter, Thrower};
use crate::command::{Action, Activation, Command, ThrowStyle};
use crate::event::{GameEvent, TurnEndReason};
use crate::game::scenario::Scenario;
use crate::roster::GoblinId;
use crate::square::Square;
use crate::state::Ball;
use crate::team::TeamId;
use crate::violation::RuleViolation;

const NORTH: u8 = 1;
const EAST: u8 = 2;
const SOUTH: u8 = 3;
const CATCH: u8 = 4;

fn id(text: &str) -> GoblinId {
    text.parse().unwrap()
}

fn chaos(goblin: &str, path: &[(u8, u8)], receiver: &str) -> Command {
    Command::Act(Activation {
        goblin: id(goblin),
        path: path.iter().map(|(x, y)| Square::new(*x, *y)).collect(),
        action: Some(Action::Throw {
            receiver: id(receiver),
            style: ThrowStyle::Chaos,
        }),
    })
}

fn thrower_with_the_ball_and_chaos() -> Scenario {
    Scenario::new()
        .rules(|ruleset| {
            ruleset.keeps_moving_after_action = true;
            ruleset.throws_have_styles = true;
        })
        .goblin("H1", Thrower, 4, 7)
        .goblin("H2", Sprinter, 8, 7)
        .goblin("A1", Sprinter, 12, 3)
        .ball_carried_by("H1")
}

#[test]
fn given_chaos_throws_when_the_thrower_throws_into_chaos_then_no_throw_roll_and_three_bounces_from_the_receiver()
 {
    let mut game = thrower_with_the_ball_and_chaos().game([NORTH, EAST, SOUTH]);
    let events = game.apply(TeamId::HOME, chaos("H1", &[], "H2")).unwrap();
    let bounces = events
        .iter()
        .filter(|event| matches!(event, GameEvent::BallBounced { .. }))
        .count();
    assert_eq!(
        (
            bounces,
            game.state().ball,
            events
                .iter()
                .any(|event| matches!(event, GameEvent::ThrowAttempted { .. }))
        ),
        (
            3,
            Ball::Loose {
                at: Square::new(9, 7)
            },
            false
        )
    );
}

#[test]
fn given_chaos_throws_when_the_ball_lies_loose_after_its_three_bounces_then_the_turn_ends_on_a_fumble()
 {
    let mut game = thrower_with_the_ball_and_chaos().game([NORTH, EAST, SOUTH]);
    let events = game.apply(TeamId::HOME, chaos("H1", &[], "H2")).unwrap();
    assert!(events.contains(&GameEvent::TurnEnded {
        team: TeamId::HOME,
        reason: TurnEndReason::Fumble
    }));
}

#[test]
fn given_chaos_throws_when_a_bounce_lands_in_a_teammate_s_hands_then_the_whole_move_is_spent_and_the_turn_goes_on()
 {
    let mut game = thrower_with_the_ball_and_chaos()
        .goblin("H3", Sprinter, 8, 6)
        .game([NORTH, CATCH]);
    game.apply(TeamId::HOME, chaos("H1", &[], "H2")).unwrap();
    assert_eq!(
        (
            game.state().ball,
            game.state().goblin(id("H1")).unwrap().steps_left,
            game.state().active_team()
        ),
        (Ball::Carried { by: id("H3") }, 0, Some(TeamId::HOME))
    );
}

#[test]
fn given_chaos_throws_when_a_bounce_lands_in_an_enemy_s_hands_then_the_turn_ends_on_a_fumble() {
    let mut game = thrower_with_the_ball_and_chaos()
        .goblin("A2", Sprinter, 8, 6)
        .game([NORTH, CATCH]);
    let events = game.apply(TeamId::HOME, chaos("H1", &[], "H2")).unwrap();
    assert_eq!(
        (
            game.state().ball,
            events.contains(&GameEvent::TurnEnded {
                team: TeamId::HOME,
                reason: TurnEndReason::Fumble
            })
        ),
        (Ball::Carried { by: id("A2") }, true)
    );
}

#[test]
fn given_chaos_throws_when_the_thrower_walked_first_then_it_must_stand_still() {
    let mut game = thrower_with_the_ball_and_chaos().game([]);
    let refusal = game.apply(TeamId::HOME, chaos("H1", &[(4, 8)], "H2"));
    assert_eq!(
        refusal,
        Err(RuleViolation::MustStandStillFirst { goblin: id("H1") })
    );
}

#[test]
fn given_simple_throws_only_when_throwing_into_chaos_then_refused() {
    let mut game = thrower_with_the_ball_and_chaos()
        .rules(|ruleset| ruleset.throws_have_styles = false)
        .game([]);
    let refusal = game.apply(TeamId::HOME, chaos("H1", &[], "H2"));
    assert_eq!(
        refusal,
        Err(RuleViolation::StyleNotInTheseRules { goblin: id("H1") })
    );
}

#[test]
fn given_chaos_throws_when_an_enemy_on_the_line_intercepts_then_the_turn_ends_on_a_fumble() {
    let mut game = thrower_with_the_ball_and_chaos()
        .goblin("A2", Sprinter, 6, 7)
        .game([4]);
    let events = game.apply(TeamId::HOME, chaos("H1", &[], "H2")).unwrap();
    assert!(events.contains(&GameEvent::TurnEnded {
        team: TeamId::HOME,
        reason: TurnEndReason::Fumble
    }));
}
