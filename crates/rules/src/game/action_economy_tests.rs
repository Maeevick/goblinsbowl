//! What actions cost, a step for an assault or a throw and every step left for a Stance, and how
//! many a goblin takes: one in all, or one of each kind.

use crate::characteristics::Archetype::{Bruiser, Sprinter, Thrower};
use crate::command::{Action, Activation, Command};
use crate::game::scenario::Scenario;
use crate::roster::GoblinId;
use crate::square::Square;
use crate::state::{Ball, Presence};
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

fn assault(target: &str) -> Option<Action> {
    Some(Action::assault(id(target)))
}

fn actions_cost_a_step(scenario: Scenario) -> Scenario {
    scenario.rules(|ruleset| {
        ruleset.keeps_moving_after_action = true;
        ruleset.actions_cost_a_step = true;
    })
}

fn a_bruiser_two_steps_from_an_enemy_bruiser() -> Scenario {
    actions_cost_a_step(
        Scenario::new()
            .goblin("H1", Bruiser, 3, 7)
            .goblin("H5", Sprinter, 3, 10)
            .goblin("A1", Bruiser, 6, 7),
    )
}

#[test]
fn given_actions_that_cost_a_step_when_a_bruiser_walks_two_then_assaults_then_its_three_steps_are_spent()
 {
    let mut game = a_bruiser_two_steps_from_an_enemy_bruiser().game([4, 2]);

    game.apply(TeamId::HOME, order("H1", &[(4, 7), (5, 7)], assault("A1")))
        .unwrap();

    assert_eq!(game.state().goblin(id("H1")).unwrap().steps_left, 0);
}

#[test]
fn given_actions_that_cost_a_step_when_a_bruiser_walks_its_whole_move_to_assault_then_no_step_is_left_to_act()
 {
    let mut game = actions_cost_a_step(
        Scenario::new()
            .goblin("H1", Bruiser, 2, 7)
            .goblin("A1", Bruiser, 6, 7),
    )
    .game([4, 2]);

    let refusal = game.apply(
        TeamId::HOME,
        order("H1", &[(3, 7), (4, 7), (5, 7)], assault("A1")),
    );

    assert_eq!(
        refusal,
        Err(RuleViolation::NoStepLeftToAct {
            goblin: id("H1"),
            steps_left: 3,
            steps: 3
        })
    );
}

#[test]
fn given_actions_that_cost_a_step_when_a_sprinter_walks_two_then_braces_then_its_other_steps_are_spent()
 {
    let mut game = a_bruiser_two_steps_from_an_enemy_bruiser().game([]);

    game.apply(
        TeamId::HOME,
        order("H5", &[(4, 10), (5, 10)], Some(Action::Stance)),
    )
    .unwrap();

    let sprinter = game.state().goblin(id("H5")).unwrap();
    assert_eq!(
        (sprinter.steps_left, sprinter.in_stance, sprinter.can_act()),
        (0, true, false)
    );
}

#[test]
fn given_actions_that_cost_a_step_when_a_sprinter_walks_its_whole_move_then_it_can_no_longer_act() {
    let mut game = a_bruiser_two_steps_from_an_enemy_bruiser().game([]);

    game.apply(
        TeamId::HOME,
        order("H5", &[(4, 10), (5, 10), (6, 10), (7, 10), (8, 10)], None),
    )
    .unwrap();

    assert!(!game.state().goblin(id("H5")).unwrap().can_act());
}

fn one_action_of_each_kind(scenario: Scenario) -> Scenario {
    scenario.rules(|ruleset| {
        ruleset.keeps_moving_after_action = true;
        ruleset.one_action_of_each_kind = true;
    })
}

#[test]
fn given_one_action_of_each_kind_when_a_thrower_threw_then_it_may_still_assault() {
    let mut game = one_action_of_each_kind(
        Scenario::new()
            .goblin("H9", Thrower, 4, 7)
            .goblin("H5", Sprinter, 7, 7)
            .goblin("A1", Sprinter, 4, 9)
            .ball_carried_by("H9"),
    )
    .game([3, 3, 2, 1]);
    game.apply(
        TeamId::HOME,
        order("H9", &[], Some(Action::throw(id("H5")))),
    )
    .unwrap();

    game.apply(TeamId::HOME, order("H9", &[(4, 8)], assault("A1")))
        .unwrap();

    assert_eq!(
        game.state().goblin(id("A1")).unwrap().presence,
        Presence::Down(Square::new(4, 9))
    );
}

#[test]
fn given_one_action_of_each_kind_when_a_bruiser_assaults_a_second_time_then_its_assault_is_used() {
    let mut game = one_action_of_each_kind(
        Scenario::new()
            .goblin("H1", Bruiser, 5, 7)
            .goblin("A1", Bruiser, 6, 7),
    )
    .game([4, 2]);
    game.apply(TeamId::HOME, order("H1", &[], assault("A1")))
        .unwrap();

    let refusal = game.apply(TeamId::HOME, order("H1", &[], assault("A1")));

    assert_eq!(
        refusal,
        Err(RuleViolation::ActionAlreadyUsed { goblin: id("H1") })
    );
}

#[test]
fn given_each_kind_once_at_a_step_each_when_a_sprinter_assaults_picks_up_and_throws_then_the_ball_reaches_the_receiver_with_two_steps_left()
 {
    let mut game = actions_cost_a_step(one_action_of_each_kind(
        Scenario::new()
            .goblin("H5", Sprinter, 5, 7)
            .goblin("H2", Sprinter, 5, 11)
            .goblin("A1", Sprinter, 6, 7)
            .ball_loose_at(5, 8),
    ))
    .game([3, 1, 3, 3, 3]);

    game.apply(TeamId::HOME, order("H5", &[], assault("A1")))
        .unwrap();
    game.apply(TeamId::HOME, order("H5", &[(5, 8)], None))
        .unwrap();
    game.apply(
        TeamId::HOME,
        order("H5", &[], Some(Action::throw(id("H2")))),
    )
    .unwrap();

    assert_eq!(
        (
            game.state().ball,
            game.state().goblin(id("H5")).unwrap().steps_left
        ),
        (Ball::Carried { by: id("H2") }, 2)
    );
}
