//! Scoring in the enemy den, then the kick-off that follows.

use crate::characteristics::Archetype::{Bruiser, Sprinter, Thrower};
use crate::command::{Action, Activation, Command};
use crate::event::{GameEvent, TurnEndReason};
use crate::game::scenario::Scenario;
use crate::roster::GoblinId;
use crate::square::Square;
use crate::state::Presence;
use crate::team::TeamId;

fn id(text: &str) -> GoblinId {
    text.parse().unwrap()
}

fn act(goblin: &str, path: &[(u8, u8)], action: Option<Action>) -> Command {
    Command::Act(Activation {
        goblin: id(goblin),
        path: path.iter().map(|(x, y)| Square::new(*x, *y)).collect(),
        action,
    })
}

fn home_carrier_near_the_away_den() -> Scenario {
    Scenario::new()
        .goblin("H1", Sprinter, 12, 7)
        .goblin("H2", Sprinter, 3, 3)
        .goblin("A1", Bruiser, 10, 11)
        .ball_carried_by("H1")
}

const KICK_OFF_BOUNCE_NORTH: u8 = 1;

#[test]
fn given_a_carrier_stepping_into_the_enemy_den_when_resolved_then_its_team_scores() {
    let mut game = home_carrier_near_the_away_den().game([KICK_OFF_BOUNCE_NORTH]);
    game.apply(TeamId::HOME, act("H1", &[(13, 7), (14, 7)], None))
        .unwrap();
    assert_eq!(game.state().score, vec![1, 0]);
}

#[test]
fn given_a_score_when_resolved_then_scored_then_turn_ended_for_score() {
    let mut game = home_carrier_near_the_away_den().game([KICK_OFF_BOUNCE_NORTH]);
    let events = game
        .apply(TeamId::HOME, act("H1", &[(13, 7), (14, 7)], None))
        .unwrap();
    let scored_then_ended = events.windows(2).any(|pair| {
        pair == [
            GameEvent::Scored {
                team: TeamId::HOME,
                scorer: id("H1"),
            },
            GameEvent::TurnEnded {
                team: TeamId::HOME,
                reason: TurnEndReason::Score,
            },
        ]
    });
    assert!(scored_then_ended);
}

#[test]
fn given_a_carrier_crossing_the_den_when_resolved_then_the_rest_of_its_path_is_dropped() {
    let mut game = home_carrier_near_the_away_den().game([KICK_OFF_BOUNCE_NORTH]);
    let events = game
        .apply(TeamId::HOME, act("H1", &[(13, 7), (14, 7), (14, 8)], None))
        .unwrap();
    let steps = events
        .iter()
        .filter(|event| matches!(event, GameEvent::Moved { .. }))
        .count();
    assert_eq!(steps, 2);
}

#[test]
fn given_a_score_when_the_ball_is_kicked_off_then_the_conceding_team_plays_first() {
    let mut game = home_carrier_near_the_away_den().game([KICK_OFF_BOUNCE_NORTH]);
    game.apply(TeamId::HOME, act("H1", &[(13, 7), (14, 7)], None))
        .unwrap();
    assert_eq!(game.state().active_team(), Some(TeamId::AWAY));
}

#[test]
fn given_a_score_when_the_ball_is_kicked_off_then_goblins_line_up_again() {
    let mut game = home_carrier_near_the_away_den().game([KICK_OFF_BOUNCE_NORTH]);
    game.apply(TeamId::HOME, act("H1", &[(13, 7), (14, 7)], None))
        .unwrap();
    assert_eq!(
        game.state().goblin(id("H1")).unwrap().presence,
        Presence::Standing(Square::new(6, 4))
    );
}

#[test]
fn given_a_goblin_without_the_ball_entering_the_enemy_den_when_resolved_then_no_score() {
    let mut game = home_carrier_near_the_away_den()
        .ball_loose_at(3, 7)
        .game([]);
    game.apply(TeamId::HOME, act("H1", &[(13, 7), (14, 7)], None))
        .unwrap();
    assert_eq!(game.state().score, vec![0, 0]);
}

#[test]
fn given_a_carrier_entering_its_own_den_when_resolved_then_no_score() {
    let mut game = Scenario::new()
        .goblin("H1", Sprinter, 2, 7)
        .goblin("H2", Sprinter, 3, 3)
        .ball_carried_by("H1")
        .game([]);
    game.apply(TeamId::HOME, act("H1", &[(1, 7), (0, 7)], None))
        .unwrap();
    assert_eq!(game.state().score, vec![0, 0]);
}

#[test]
fn given_the_ball_lying_in_the_enemy_den_when_picked_up_there_then_its_team_scores() {
    let mut game = home_carrier_near_the_away_den()
        .ball_loose_at(14, 7)
        .edit("H1", |goblin| {
            goblin.presence = Presence::Standing(Square::new(13, 7))
        })
        .game([3, KICK_OFF_BOUNCE_NORTH]);
    game.apply(TeamId::HOME, act("H1", &[(14, 7)], None))
        .unwrap();
    assert_eq!(game.state().score, vec![1, 0]);
}

#[test]
fn given_a_throw_caught_in_the_enemy_den_when_resolved_then_its_team_scores() {
    let mut game = Scenario::new()
        .goblin("H1", Thrower, 9, 5)
        .goblin("H2", Sprinter, 14, 5)
        .goblin("H3", Sprinter, 3, 3)
        .ball_carried_by("H1")
        .game([3, 3, KICK_OFF_BOUNCE_NORTH]);
    game.apply(
        TeamId::HOME,
        act("H1", &[], Some(Action::Throw { receiver: id("H2") })),
    )
    .unwrap();
    assert_eq!(game.state().score, vec![1, 0]);
}

#[test]
fn given_a_fumbled_ball_caught_by_an_opponent_in_its_enemy_den_when_resolved_then_the_opponent_scores()
 {
    let mut game = Scenario::new()
        .goblin("H1", Sprinter, 2, 7)
        .goblin("H2", Sprinter, 3, 3)
        .goblin("A1", Sprinter, 0, 6)
        .ball_loose_at(1, 6)
        .game([1, 4, 4, KICK_OFF_BOUNCE_NORTH]);
    game.apply(TeamId::HOME, act("H1", &[(1, 7), (1, 6)], None))
        .unwrap();
    assert_eq!(game.state().score, vec![0, 1]);
}

#[test]
fn given_the_opponent_scored_during_home_turn_when_kicking_off_then_home_plays_again() {
    let mut game = Scenario::new()
        .goblin("H1", Sprinter, 2, 7)
        .goblin("H2", Sprinter, 3, 3)
        .goblin("A1", Sprinter, 0, 6)
        .ball_loose_at(1, 6)
        .game([1, 4, 4, KICK_OFF_BOUNCE_NORTH]);
    game.apply(TeamId::HOME, act("H1", &[(1, 7), (1, 6)], None))
        .unwrap();
    assert_eq!(game.state().turns_played, vec![2, 0]);
}

#[test]
fn given_a_knocked_out_goblin_when_the_next_kick_off_comes_then_it_returns_with_rolled_health() {
    let mut game = home_carrier_near_the_away_den()
        .goblin("A2", Bruiser, 10, 3)
        .edit("A2", |goblin| {
            goblin.presence = Presence::KnockedOut;
            goblin.health = 0;
        })
        .game([4, KICK_OFF_BOUNCE_NORTH]);
    game.apply(TeamId::HOME, act("H1", &[(13, 7), (14, 7)], None))
        .unwrap();
    let returned = game.state().goblin(id("A2")).unwrap();
    assert_eq!((returned.is_standing(), returned.health), (true, 4));
}

#[test]
fn given_a_hurt_goblin_when_the_next_kick_off_comes_then_it_is_back_to_full_health() {
    let mut game = home_carrier_near_the_away_den()
        .edit("A1", |goblin| goblin.health = 2)
        .game([KICK_OFF_BOUNCE_NORTH]);
    game.apply(TeamId::HOME, act("H1", &[(13, 7), (14, 7)], None))
        .unwrap();
    assert_eq!(game.state().goblin(id("A1")).unwrap().health, 6);
}
