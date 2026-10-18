//! Goblins that keep moving after their action: their Move and one action per turn, over several orders.

use crate::characteristics::Archetype::{Bruiser, Sprinter, Thrower};
use crate::command::{Action, Activation, Command};
use crate::game::scenario::Scenario;
use crate::roster::GoblinId;
use crate::square::Square;
use crate::state::Presence;
use crate::team::TeamId;
use crate::violation::RuleViolation;

fn id(text: &str) -> GoblinId {
    text.parse().unwrap()
}

fn order(goblin: &str, path: &[(u8, u8)], action: Option<Action>) -> Command {
    Command::Act(Activation {
        goblin: id(goblin),
        path: path.iter().map(|(x, y)| Square::new(*x, *y)).collect(),
        action,
    })
}

fn moving_after_action(scenario: Scenario) -> Scenario {
    scenario.rules(|ruleset| ruleset.keeps_moving_after_action = true)
}

fn a_sprinter_behind_two_bruisers_face_to_face() -> Scenario {
    moving_after_action(
        Scenario::new()
            .goblin("H1", Bruiser, 5, 7)
            .goblin("H5", Sprinter, 3, 9)
            .goblin("A1", Bruiser, 6, 7),
    )
}

#[test]
fn given_a_sprinter_that_walked_two_of_its_five_squares_when_the_order_is_done_then_three_steps_and_its_action_are_left()
 {
    let mut game = a_sprinter_behind_two_bruisers_face_to_face().game([]);

    game.apply(TeamId::HOME, order("H5", &[(4, 9), (5, 9)], None))
        .unwrap();

    let sprinter = game.state().goblin(id("H5")).unwrap();
    assert_eq!((sprinter.steps_left, sprinter.has_action_left), (3, true));
}

#[test]
fn given_a_bruiser_that_won_its_assault_when_ordered_to_walk_then_it_walks_its_three_steps() {
    let mut game = a_sprinter_behind_two_bruisers_face_to_face().game([4, 2]);
    let assault = Action::assault(id("A1"));
    game.apply(TeamId::HOME, order("H1", &[], Some(assault)))
        .unwrap();

    game.apply(TeamId::HOME, order("H1", &[(5, 6), (5, 5), (5, 4)], None))
        .unwrap();

    assert_eq!(
        game.state().goblin(id("H1")).unwrap().presence,
        Presence::Standing(Square::new(5, 4))
    );
}

#[test]
fn given_a_thrower_whose_pass_was_caught_when_ordered_to_walk_then_it_runs_on() {
    let mut game = moving_after_action(
        Scenario::new()
            .goblin("H9", Thrower, 4, 7)
            .goblin("H5", Sprinter, 8, 7)
            .ball_carried_by("H9"),
    )
    .game([3, 3]);
    let pass = Action::throw(id("H5"));
    game.apply(TeamId::HOME, order("H9", &[], Some(pass)))
        .unwrap();

    game.apply(TeamId::HOME, order("H9", &[(4, 8), (4, 9)], None))
        .unwrap();

    assert_eq!(
        game.state().goblin(id("H9")).unwrap().presence,
        Presence::Standing(Square::new(4, 9))
    );
}

#[test]
fn given_a_sprinter_that_walked_when_a_teammate_played_then_the_sprinter_walks_on() {
    let mut game = a_sprinter_behind_two_bruisers_face_to_face().game([]);
    game.apply(TeamId::HOME, order("H5", &[(4, 9), (5, 9)], None))
        .unwrap();
    game.apply(TeamId::HOME, order("H1", &[], Some(Action::Stance)))
        .unwrap();

    game.apply(TeamId::HOME, order("H5", &[(6, 9), (7, 9), (8, 9)], None))
        .unwrap();

    assert_eq!(
        game.state().goblin(id("H5")).unwrap().presence,
        Presence::Standing(Square::new(8, 9))
    );
}

#[test]
fn given_a_sprinter_with_three_steps_left_when_ordered_four_squares_then_path_too_long() {
    let mut game = a_sprinter_behind_two_bruisers_face_to_face().game([]);
    game.apply(TeamId::HOME, order("H5", &[(4, 9), (5, 9)], None))
        .unwrap();

    let refusal = game.apply(
        TeamId::HOME,
        order("H5", &[(6, 9), (7, 9), (8, 9), (9, 9)], None),
    );

    assert_eq!(
        refusal,
        Err(RuleViolation::PathTooLong {
            goblin: id("H5"),
            steps_left: 3,
            steps: 4
        })
    );
}

#[test]
fn given_a_bruiser_that_braced_when_ordered_to_assault_then_its_action_is_already_used() {
    let mut game = a_sprinter_behind_two_bruisers_face_to_face().game([4, 2]);
    game.apply(TeamId::HOME, order("H1", &[], Some(Action::Stance)))
        .unwrap();

    let refusal = game.apply(
        TeamId::HOME,
        order("H1", &[], Some(Action::assault(id("A1")))),
    );

    assert_eq!(
        refusal,
        Err(RuleViolation::ActionAlreadyUsed { goblin: id("H1") })
    );
}

#[test]
fn given_an_order_without_step_or_action_when_applied_then_the_goblin_is_done_for_the_turn() {
    let mut game = a_sprinter_behind_two_bruisers_face_to_face().game([]);

    game.apply(TeamId::HOME, order("H5", &[], None)).unwrap();

    assert!(!game.state().goblin(id("H5")).unwrap().can_act());
}
