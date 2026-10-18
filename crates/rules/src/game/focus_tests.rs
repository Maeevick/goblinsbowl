//! Focus: standing still a whole turn to throw at +1 on the team's next turn.

use crate::characteristics::Archetype::{Sprinter, Thrower};
use crate::command::{Action, Activation, Command};
use crate::event::GameEvent;
use crate::game::scenario::Scenario;
use crate::roster::GoblinId;
use crate::square::Square;
use crate::state::Focus;
use crate::team::TeamId;
use crate::violation::RuleViolation;

fn id(text: &str) -> GoblinId {
    text.parse().unwrap()
}

fn order(goblin: &str, path: &[(u8, u8)], action: Action) -> Command {
    Command::Act(Activation {
        goblin: id(goblin),
        path: path.iter().map(|(x, y)| Square::new(*x, *y)).collect(),
        action: Some(action),
    })
}

fn thrower_and_receiver_with_focus() -> Scenario {
    Scenario::new()
        .rules(|ruleset| {
            ruleset.keeps_moving_after_action = true;
            ruleset.throws_have_styles = true;
            ruleset.turn_ends_when_no_goblin_can_act = false;
        })
        .goblin("H1", Thrower, 4, 7)
        .goblin("H2", Sprinter, 8, 7)
        .goblin("A1", Sprinter, 12, 3)
        .ball_carried_by("H1")
}

#[test]
fn given_focus_when_a_thrower_that_has_not_moved_focuses_then_its_whole_move_is_spent() {
    let mut game = thrower_and_receiver_with_focus().game([]);
    let events = game
        .apply(TeamId::HOME, order("H1", &[], Action::Focus))
        .unwrap();
    let thrower = game.state().goblin(id("H1")).unwrap();
    assert_eq!(
        (
            thrower.steps_left,
            thrower.focus,
            events.contains(&GameEvent::FocusTaken { goblin: id("H1") })
        ),
        (0, Focus::Focusing, true)
    );
}

#[test]
fn given_focus_when_the_team_s_next_turn_starts_then_the_thrower_is_focused() {
    let mut game = thrower_and_receiver_with_focus().game([]);
    game.apply(TeamId::HOME, order("H1", &[], Action::Focus))
        .unwrap();
    game.apply(TeamId::HOME, Command::EndTurn).unwrap();
    game.apply(TeamId::AWAY, Command::EndTurn).unwrap();
    assert_eq!(game.state().goblin(id("H1")).unwrap().focus, Focus::Focused);
}

#[test]
fn given_focus_when_a_focused_thrower_rolls_a_one_then_its_plus_one_carries_the_throw() {
    let mut game = thrower_and_receiver_with_focus()
        .edit("H1", |goblin| goblin.focus = Focus::Focused)
        .game([1, 3]);
    let events = game
        .apply(TeamId::HOME, order("H1", &[], Action::throw(id("H2"))))
        .unwrap();
    assert!(events.iter().any(|event| matches!(
        event,
        GameEvent::ThrowAttempted { roll, .. } if roll.bonus == 1 && roll.success
    )));
}

#[test]
fn given_focus_when_the_turn_after_it_ends_then_the_focus_wears_off() {
    let mut game = thrower_and_receiver_with_focus()
        .edit("H1", |goblin| goblin.focus = Focus::Focused)
        .game([]);
    game.apply(TeamId::HOME, Command::EndTurn).unwrap();
    game.apply(TeamId::AWAY, Command::EndTurn).unwrap();
    assert_eq!(
        game.state().goblin(id("H1")).unwrap().focus,
        Focus::Unfocused
    );
}

#[test]
fn given_focus_when_a_thrower_walked_first_then_it_must_stand_still() {
    let mut game = thrower_and_receiver_with_focus().game([]);
    let refusal = game.apply(TeamId::HOME, order("H1", &[(4, 8)], Action::Focus));
    assert_eq!(
        refusal,
        Err(RuleViolation::MustStandStillFirst { goblin: id("H1") })
    );
}
