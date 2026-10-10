//! Turn order across a match: per-team counters, half-time and the final whistle.

use crate::characteristics::Archetype::{Bruiser, Sprinter};
use crate::command::Command;
use crate::event::{GameEvent, TurnEndReason};
use crate::game::scenario::Scenario;
use crate::square::Square;
use crate::state::{Phase, Presence};
use crate::team::TeamId;
use crate::violation::RuleViolation;

const KICK_OFF_BOUNCE_NORTH: u8 = 1;

fn small_match() -> Scenario {
    Scenario::new()
        .goblin("H1", Sprinter, 3, 7)
        .goblin("A1", Bruiser, 11, 7)
}

#[test]
fn given_home_on_its_tenth_turn_when_it_ends_then_away_plays_its_tenth() {
    let mut game = small_match().turns(10, 9).game([]);
    game.apply(TeamId::HOME, Command::EndTurn).unwrap();
    assert_eq!(game.state().turns_played, vec![10, 10]);
}

#[test]
fn given_both_teams_on_their_tenth_turn_when_away_ends_then_the_half_ends() {
    let mut game = small_match()
        .turns(10, 10)
        .active(TeamId::AWAY)
        .game([KICK_OFF_BOUNCE_NORTH]);
    let events = game.apply(TeamId::AWAY, Command::EndTurn).unwrap();
    assert!(events.contains(&GameEvent::HalfEnded { half: 1 }));
}

#[test]
fn given_the_first_half_ends_when_the_second_starts_then_the_other_team_receives() {
    let mut game = small_match()
        .turns(10, 10)
        .active(TeamId::AWAY)
        .game([KICK_OFF_BOUNCE_NORTH]);
    game.apply(TeamId::AWAY, Command::EndTurn).unwrap();
    assert_eq!(
        (
            game.state().half,
            game.state().active_team(),
            game.state().turns_played.clone()
        ),
        (2, Some(TeamId::AWAY), vec![0, 1])
    );
}

#[test]
fn given_a_hurt_goblin_when_the_second_half_starts_then_it_is_healed() {
    let mut game = small_match()
        .edit("A1", |goblin| goblin.health = 1)
        .turns(10, 10)
        .active(TeamId::AWAY)
        .game([KICK_OFF_BOUNCE_NORTH]);
    game.apply(TeamId::AWAY, Command::EndTurn).unwrap();
    assert_eq!(
        game.state().goblin("A1".parse().unwrap()).unwrap().health,
        6
    );
}

#[test]
fn given_the_last_turn_of_the_second_half_when_it_ends_then_the_match_is_finished() {
    let mut game = small_match()
        .half(2)
        .turns(10, 10)
        .active(TeamId::AWAY)
        .game([]);
    game.apply(TeamId::AWAY, Command::EndTurn).unwrap();
    assert_eq!(game.state().phase, Phase::Finished);
}

#[test]
fn given_the_last_turn_of_the_second_half_when_it_ends_then_the_final_score_is_reported() {
    let mut game = small_match()
        .half(2)
        .turns(10, 10)
        .active(TeamId::AWAY)
        .game([]);
    let events = game.apply(TeamId::AWAY, Command::EndTurn).unwrap();
    assert_eq!(
        events.last(),
        Some(&GameEvent::GameEnded { score: vec![0, 0] })
    );
}

#[test]
fn given_a_finished_match_when_a_command_arrives_then_match_finished() {
    let mut game = small_match()
        .half(2)
        .turns(10, 10)
        .active(TeamId::AWAY)
        .game([]);
    game.apply(TeamId::AWAY, Command::EndTurn).unwrap();
    assert_eq!(
        game.apply(TeamId::HOME, Command::EndTurn),
        Err(RuleViolation::MatchFinished)
    );
}

#[test]
fn given_home_out_of_turns_when_away_ends_its_turn_then_away_plays_again() {
    let mut game = small_match().turns(10, 8).active(TeamId::AWAY).game([]);
    game.apply(TeamId::AWAY, Command::EndTurn).unwrap();
    assert_eq!(
        (
            game.state().active_team(),
            game.state().turns_played.clone()
        ),
        (Some(TeamId::AWAY), vec![10, 9])
    );
}

#[test]
fn given_every_away_goblin_down_when_away_turn_starts_then_it_ends_at_once() {
    let mut game = small_match()
        .edit("A1", |goblin| {
            goblin.presence = Presence::Down(Square::new(11, 7))
        })
        .game([]);
    let events = game.apply(TeamId::HOME, Command::EndTurn).unwrap();
    assert!(events.contains(&GameEvent::TurnEnded {
        team: TeamId::AWAY,
        reason: TurnEndReason::NoGoblinCanAct
    }));
}

#[test]
fn given_every_away_goblin_down_when_away_loses_its_turn_then_home_plays_its_second() {
    let mut game = small_match()
        .edit("A1", |goblin| {
            goblin.presence = Presence::Down(Square::new(11, 7))
        })
        .game([]);
    game.apply(TeamId::HOME, Command::EndTurn).unwrap();
    assert_eq!(game.state().turns_played, vec![2, 1]);
}
