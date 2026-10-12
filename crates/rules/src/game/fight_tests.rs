//! Fight actions resolved through whole activations: Assault and Stance.

use crate::characteristics::Archetype::{Bruiser, Sprinter};
use crate::command::{Action, Activation, Command};
use crate::event::{ContestRoll, GameEvent, TurnEndReason};
use crate::game::scenario::Scenario;
use crate::roster::GoblinId;
use crate::square::Square;
use crate::state::{Ball, Presence};
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

fn assault(goblin: &str, path: &[(u8, u8)], target: &str) -> Command {
    order(goblin, path, Action::Assault { target: id(target) })
}

fn bruisers_face_to_face() -> Scenario {
    Scenario::new()
        .goblin("H1", Bruiser, 5, 7)
        .goblin("H2", Sprinter, 3, 3)
        .goblin("A1", Bruiser, 6, 7)
}

#[test]
fn given_an_assault_won_by_two_when_resolved_then_the_defender_is_down() {
    let mut game = bruisers_face_to_face().game([4, 2]);
    game.apply(TeamId::HOME, assault("H1", &[], "A1")).unwrap();
    assert_eq!(
        game.state().goblin(id("A1")).unwrap().presence,
        Presence::Down(Square::new(6, 7))
    );
}

#[test]
fn given_an_assault_won_by_two_when_resolved_then_the_defender_loses_two_health() {
    let mut game = bruisers_face_to_face().game([4, 2]);
    game.apply(TeamId::HOME, assault("H1", &[], "A1")).unwrap();
    assert_eq!(game.state().goblin(id("A1")).unwrap().health, 4);
}

#[test]
fn given_an_assault_when_resolved_then_the_contest_is_reported() {
    let mut game = bruisers_face_to_face().game([4, 2]);
    let events = game.apply(TeamId::HOME, assault("H1", &[], "A1")).unwrap();
    assert_eq!(
        events.first(),
        Some(&GameEvent::AssaultResolved {
            attacker: id("H1"),
            defender: id("A1"),
            contest: ContestRoll {
                actor_faces: 4,
                actor_rolled: 4,
                opponent_faces: 4,
                opponent_rolled: 2,
                opponent_modifier: 0,
                margin: 2
            },
            defender_braced: false
        })
    );
}

#[test]
fn given_a_tied_assault_when_resolved_then_the_defender_falls_without_damage() {
    let mut game = bruisers_face_to_face().game([3, 3]);
    game.apply(TeamId::HOME, assault("H1", &[], "A1")).unwrap();
    let defender = game.state().goblin(id("A1")).unwrap();
    assert_eq!(
        (defender.presence, defender.health),
        (Presence::Down(Square::new(6, 7)), 6)
    );
}

#[test]
fn given_an_assault_lost_by_two_when_resolved_then_the_attacker_is_down_with_one_damage() {
    let mut game = bruisers_face_to_face().game([1, 3]);
    game.apply(TeamId::HOME, assault("H1", &[], "A1")).unwrap();
    let attacker = game.state().goblin(id("H1")).unwrap();
    assert_eq!(
        (attacker.presence, attacker.health),
        (Presence::Down(Square::new(5, 7)), 5)
    );
}

#[test]
fn given_an_assault_lost_when_resolved_then_the_turn_ends_on_a_knockdown() {
    let mut game = bruisers_face_to_face().game([1, 3]);
    let events = game.apply(TeamId::HOME, assault("H1", &[], "A1")).unwrap();
    assert!(events.contains(&GameEvent::TurnEnded {
        team: TeamId::HOME,
        reason: TurnEndReason::Knockdown
    }));
}

#[test]
fn given_a_defender_in_stance_when_assaulted_then_its_defense_gets_one() {
    let mut game = bruisers_face_to_face()
        .edit("A1", |goblin| goblin.in_stance = true)
        .game([4, 2]);
    let events = game.apply(TeamId::HOME, assault("H1", &[], "A1")).unwrap();
    assert!(matches!(
        events.first(),
        Some(GameEvent::AssaultResolved { contest, .. }) if contest.opponent_modifier == 1
    ));
}

#[test]
fn given_a_defender_in_stance_when_assaulted_then_the_assault_reports_it_braced() {
    let mut game = bruisers_face_to_face()
        .edit("A1", |goblin| goblin.in_stance = true)
        .game([4, 2]);
    let events = game.apply(TeamId::HOME, assault("H1", &[], "A1")).unwrap();
    assert!(matches!(
        events.first(),
        Some(GameEvent::AssaultResolved {
            defender_braced: true,
            ..
        })
    ));
}

#[test]
fn given_a_defender_in_a_two_goblin_stance_wall_when_assaulted_then_its_defense_gets_two() {
    let mut game = bruisers_face_to_face()
        .goblin("A2", Bruiser, 7, 7)
        .edit("A1", |goblin| goblin.in_stance = true)
        .edit("A2", |goblin| goblin.in_stance = true)
        .game([4, 2]);
    let events = game.apply(TeamId::HOME, assault("H1", &[], "A1")).unwrap();
    assert!(matches!(
        events.first(),
        Some(GameEvent::AssaultResolved { contest, .. }) if contest.opponent_modifier == 2
    ));
}

#[test]
fn given_a_downed_defender_when_assaulted_then_its_defense_loses_two() {
    let mut game = bruisers_face_to_face()
        .edit("A1", |goblin| {
            goblin.presence = Presence::Down(Square::new(6, 7))
        })
        .game([3, 2]);
    let events = game.apply(TeamId::HOME, assault("H1", &[], "A1")).unwrap();
    assert!(matches!(
        events.first(),
        Some(GameEvent::AssaultResolved { contest, .. }) if contest.opponent_modifier == -2
    ));
}

#[test]
fn given_a_downed_defender_when_the_assault_wins_by_three_then_it_stays_down_and_loses_three() {
    let mut game = bruisers_face_to_face()
        .edit("A1", |goblin| {
            goblin.presence = Presence::Down(Square::new(6, 7))
        })
        .game([3, 2]);
    game.apply(TeamId::HOME, assault("H1", &[], "A1")).unwrap();
    let defender = game.state().goblin(id("A1")).unwrap();
    assert_eq!(
        (defender.presence, defender.health),
        (Presence::Down(Square::new(6, 7)), 3)
    );
}

#[test]
fn given_a_defender_on_its_last_health_when_hit_then_it_is_knocked_out() {
    let mut game = bruisers_face_to_face()
        .edit("A1", |goblin| goblin.health = 1)
        .game([4, 2]);
    game.apply(TeamId::HOME, assault("H1", &[], "A1")).unwrap();
    assert_eq!(
        game.state().goblin(id("A1")).unwrap().presence,
        Presence::KnockedOut
    );
}

#[test]
fn given_the_carrier_is_knocked_down_when_resolved_then_the_ball_bounces_away() {
    let mut game = bruisers_face_to_face()
        .ball_carried_by("A1")
        .game([4, 1, 2]);
    game.apply(TeamId::HOME, assault("H1", &[], "A1")).unwrap();
    assert_eq!(
        game.state().ball,
        Ball::Loose {
            at: Square::new(7, 7)
        }
    );
}

#[test]
fn given_the_dropped_ball_bounces_to_an_attacking_teammate_who_misses_then_the_turn_ends_on_a_fumble()
 {
    let mut game = bruisers_face_to_face()
        .goblin("H3", Sprinter, 6, 6)
        .ball_carried_by("A1")
        .game([4, 1, 1, 1, 2]);
    let events = game.apply(TeamId::HOME, assault("H1", &[], "A1")).unwrap();
    assert!(events.contains(&GameEvent::TurnEnded {
        team: TeamId::HOME,
        reason: TurnEndReason::Fumble
    }));
}

#[test]
fn given_a_goblin_moving_next_to_an_enemy_when_assaulting_then_the_assault_happens_after_the_move()
{
    let mut game = bruisers_face_to_face()
        .edit("H1", |goblin| {
            goblin.presence = Presence::Standing(Square::new(4, 7))
        })
        .game([4, 2]);
    game.apply(TeamId::HOME, assault("H1", &[(5, 7)], "A1"))
        .unwrap();
    assert_eq!(
        game.state().goblin(id("A1")).unwrap().presence,
        Presence::Down(Square::new(6, 7))
    );
}

#[test]
fn given_a_target_out_of_reach_when_assaulting_then_target_not_adjacent() {
    let mut game = bruisers_face_to_face().goblin("A2", Bruiser, 9, 9).game([]);
    assert_eq!(
        game.apply(TeamId::HOME, assault("H1", &[], "A2")),
        Err(RuleViolation::TargetNotAdjacent { target: id("A2") })
    );
}

#[test]
fn given_a_teammate_as_target_when_assaulting_then_target_not_an_opponent() {
    let mut game = bruisers_face_to_face().goblin("H3", Bruiser, 5, 8).game([]);
    assert_eq!(
        game.apply(TeamId::HOME, assault("H1", &[], "H3")),
        Err(RuleViolation::TargetNotAnOpponent { target: id("H3") })
    );
}

#[test]
fn given_a_knocked_out_target_when_assaulting_then_target_not_on_the_bowl() {
    let mut game = bruisers_face_to_face()
        .edit("A1", |goblin| goblin.presence = Presence::KnockedOut)
        .game([]);
    assert_eq!(
        game.apply(TeamId::HOME, assault("H1", &[], "A1")),
        Err(RuleViolation::TargetNotOnTheBowl { target: id("A1") })
    );
}

#[test]
fn given_an_unknown_target_when_assaulting_then_unknown_goblin() {
    let mut game = bruisers_face_to_face().game([]);
    assert_eq!(
        game.apply(TeamId::HOME, assault("H1", &[], "A8")),
        Err(RuleViolation::UnknownGoblin { goblin: id("A8") })
    );
}

#[test]
fn given_a_goblin_taking_stance_when_resolved_then_it_is_braced() {
    let mut game = bruisers_face_to_face().game([]);
    game.apply(TeamId::HOME, order("H1", &[], Action::Stance))
        .unwrap();
    assert!(game.state().goblin(id("H1")).unwrap().in_stance);
}

#[test]
fn given_a_goblin_taking_stance_when_resolved_then_stance_taken_is_reported() {
    let mut game = bruisers_face_to_face().game([]);
    let events = game
        .apply(TeamId::HOME, order("H1", &[], Action::Stance))
        .unwrap();
    assert_eq!(events, vec![GameEvent::StanceTaken { goblin: id("H1") }]);
}

#[test]
fn given_a_braced_goblin_when_the_opponent_plays_then_it_is_still_braced() {
    let mut game = bruisers_face_to_face().game([]);
    game.apply(TeamId::HOME, order("H1", &[], Action::Stance))
        .unwrap();
    game.apply(TeamId::HOME, Command::EndTurn).unwrap();
    assert!(game.state().goblin(id("H1")).unwrap().in_stance);
}

#[test]
fn given_a_braced_goblin_when_its_team_plays_again_then_the_stance_is_over() {
    let mut game = bruisers_face_to_face().game([]);
    game.apply(TeamId::HOME, order("H1", &[], Action::Stance))
        .unwrap();
    game.apply(TeamId::HOME, Command::EndTurn).unwrap();
    game.apply(TeamId::AWAY, Command::EndTurn).unwrap();
    assert!(!game.state().goblin(id("H1")).unwrap().in_stance);
}
