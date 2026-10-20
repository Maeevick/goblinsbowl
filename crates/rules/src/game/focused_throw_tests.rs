//! Focused throws: a Thrower sacrificing every step it has left after its throw, +1 each to the roll.

use crate::characteristics::Archetype::{Sprinter, Thrower};
use crate::command::{Action, Activation, Command, ThrowStyle};
use crate::event::GameEvent;
use crate::game::scenario::Scenario;
use crate::roster::GoblinId;
use crate::square::Square;
use crate::state::Ball;
use crate::team::TeamId;
use crate::violation::RuleViolation;

const THROW_ROLL_OF_ONE: u8 = 1;
const CATCH: u8 = 4;

fn id(text: &str) -> GoblinId {
    text.parse().unwrap()
}

fn focused_throw(goblin: &str, path: &[(u8, u8)], receiver: &str) -> Command {
    Command::Act(Activation {
        goblin: id(goblin),
        path: path.iter().map(|(x, y)| Square::new(*x, *y)).collect(),
        action: Some(Action::Throw {
            receiver: id(receiver),
            style: ThrowStyle::Focused,
        }),
    })
}

fn thrower_with_the_ball_and_focused_throws() -> Scenario {
    Scenario::new()
        .rules(|ruleset| {
            ruleset.keeps_moving_after_action = true;
            ruleset.actions_cost_a_step = true;
            ruleset.turn_ends_when_no_goblin_can_act = false;
            ruleset.focused_throws_spend_steps_left = true;
        })
        .goblin("H1", Thrower, 4, 7)
        .goblin("H2", Sprinter, 8, 7)
        .goblin("A1", Sprinter, 12, 3)
        .ball_carried_by("H1")
}

fn throw_bonus(events: &[GameEvent]) -> Option<u8> {
    events.iter().find_map(|event| match event {
        GameEvent::ThrowAttempted { roll, .. } => Some(roll.bonus),
        _ => None,
    })
}

#[test]
fn given_focused_throws_when_a_thrower_that_has_not_moved_throws_focused_then_its_three_steps_left_add_three_and_are_spent()
 {
    let mut game = thrower_with_the_ball_and_focused_throws().game([THROW_ROLL_OF_ONE, CATCH]);
    let events = game
        .apply(TeamId::HOME, focused_throw("H1", &[], "H2"))
        .unwrap();
    assert_eq!(
        (
            throw_bonus(&events),
            game.state().ball,
            game.state().goblin(id("H1")).unwrap().steps_left
        ),
        (Some(3), Ball::Carried { by: id("H2") }, 0)
    );
}

#[test]
fn given_focused_throws_when_a_thrower_walks_a_square_first_then_two_steps_are_left_to_sacrifice() {
    let mut game = thrower_with_the_ball_and_focused_throws().game([THROW_ROLL_OF_ONE, CATCH]);
    let events = game
        .apply(TeamId::HOME, focused_throw("H1", &[(5, 7)], "H2"))
        .unwrap();
    assert_eq!(throw_bonus(&events), Some(2));
}

#[test]
fn given_focused_throws_when_a_sprinter_throws_focused_then_refused_as_only_throwers_focus() {
    let mut game = thrower_with_the_ball_and_focused_throws()
        .ball_carried_by("H2")
        .game([]);
    let refusal = game.apply(TeamId::HOME, focused_throw("H2", &[], "H1"));
    assert_eq!(
        refusal,
        Err(RuleViolation::OnlyThrowersFocus { goblin: id("H2") })
    );
}

#[test]
fn given_focused_throws_when_a_thrower_orders_a_focus_on_its_own_then_refused_as_focus_comes_with_a_throw()
 {
    let mut game = thrower_with_the_ball_and_focused_throws()
        .rules(|ruleset| ruleset.throws_have_styles = true)
        .game([]);
    let refusal = game.apply(
        TeamId::HOME,
        Command::Act(Activation {
            goblin: id("H1"),
            path: Vec::new(),
            action: Some(Action::Focus),
        }),
    );
    assert_eq!(
        refusal,
        Err(RuleViolation::FocusOnlyAsAThrow { goblin: id("H1") })
    );
}

#[test]
fn given_rules_without_focused_throws_when_a_thrower_throws_focused_then_refused() {
    let mut game = thrower_with_the_ball_and_focused_throws()
        .rules(|ruleset| ruleset.focused_throws_spend_steps_left = false)
        .game([]);
    let refusal = game.apply(TeamId::HOME, focused_throw("H1", &[], "H2"));
    assert_eq!(
        refusal,
        Err(RuleViolation::StyleNotInTheseRules { goblin: id("H1") })
    );
}

fn thrower_focusing_for_a_flat_three() -> Scenario {
    thrower_with_the_ball_and_focused_throws().rules(|ruleset| {
        ruleset.actions_cost_a_step = false;
        ruleset.focused_throw_bonus = Some(3);
    })
}

#[test]
fn given_a_flat_focus_bonus_of_three_when_a_thrower_that_has_not_moved_throws_focused_then_three_and_its_whole_move_spent()
 {
    let mut game = thrower_focusing_for_a_flat_three().game([THROW_ROLL_OF_ONE, CATCH]);

    let events = game
        .apply(TeamId::HOME, focused_throw("H1", &[], "H2"))
        .unwrap();

    assert_eq!(
        (
            throw_bonus(&events),
            game.state().ball,
            game.state().goblin(id("H1")).unwrap().steps_left
        ),
        (Some(3), Ball::Carried { by: id("H2") }, 0)
    );
}

#[test]
fn given_a_flat_focus_bonus_when_a_thrower_walks_a_square_first_then_refused_as_it_must_stand_still()
 {
    let mut game = thrower_focusing_for_a_flat_three().game([]);

    let refusal = game.apply(TeamId::HOME, focused_throw("H1", &[(5, 7)], "H2"));

    assert_eq!(
        refusal,
        Err(RuleViolation::MustStandStillFirst { goblin: id("H1") })
    );
}
