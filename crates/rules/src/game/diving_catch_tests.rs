//! Diving catches: a Sprinter next to the empty square where the ball lands dives for it.

use crate::characteristics::Archetype::{Bruiser, Sprinter};
use crate::command::{Action, Activation, Command};
use crate::event::{AgilityRoll, DiveRoll, GameEvent};
use crate::game::scenario::Scenario;
use crate::roster::GoblinId;
use crate::square::Square;
use crate::state::{Ball, Presence};
use crate::team::TeamId;

const ATTACK_ROLL: u8 = 4;
const DEFENCE_ROLL: u8 = 2;
const BOUNCE_NORTH: u8 = 1;
const BOUNCE_WEST: u8 = 4;

fn id(text: &str) -> GoblinId {
    text.parse().unwrap()
}

fn sack(goblin: &str, target: &str) -> Command {
    Command::Act(Activation {
        goblin: id(goblin),
        path: Vec::new(),
        action: Some(Action::assault(id(target))),
    })
}

/// A Bruiser about to knock the enemy carrier down on (6, 7), the ball bouncing north to (6, 6),
/// a home Sprinter on its corner at (7, 5), and a Bruiser far away still to play.
fn carrier_knocked_loose_next_to_a_sprinter() -> Scenario {
    Scenario::new()
        .rules(|ruleset| {
            ruleset.sprinters_dive_for_loose_balls = true;
            ruleset.diving_catch_malus = 1;
            ruleset.missed_bounces_end_the_turn = false;
        })
        .goblin("H1", Bruiser, 5, 7)
        .goblin("H2", Sprinter, 7, 5)
        .goblin("H3", Bruiser, 2, 2)
        .goblin("A1", Bruiser, 6, 7)
        .ball_carried_by("A1")
}

fn landing() -> Square {
    Square::new(6, 6)
}

fn dive_roll(rolled: u8, malus: u8) -> AgilityRoll {
    AgilityRoll {
        faces: 4,
        rolled,
        malus,
        bonus: 0,
        target: 2,
        success: i16::from(rolled) - i16::from(malus) >= 2,
    }
}

#[test]
fn given_diving_catches_when_the_ball_lands_next_to_a_sprinter_that_rolls_well_then_it_lands_there_holding_it()
 {
    let mut game = carrier_knocked_loose_next_to_a_sprinter().game([
        ATTACK_ROLL,
        DEFENCE_ROLL,
        BOUNCE_NORTH,
        3,
    ]);

    let events = game.apply(TeamId::HOME, sack("H1", "A1")).unwrap();

    assert_eq!(
        (
            game.state().goblin(id("H2")).unwrap().presence,
            game.state().ball,
            events.contains(&GameEvent::DiveAttempted {
                goblin: id("H2"),
                from: Square::new(7, 5),
                to: landing(),
                roll: dive_roll(3, 1),
            })
        ),
        (
            Presence::Standing(landing()),
            Ball::Carried { by: id("H2") },
            true
        )
    );
}

#[test]
fn given_diving_catches_when_the_sprinter_misses_its_dive_then_it_falls_unhurt_and_the_ball_bounces_on()
 {
    let mut game = carrier_knocked_loose_next_to_a_sprinter().game([
        ATTACK_ROLL,
        DEFENCE_ROLL,
        BOUNCE_NORTH,
        2,
        BOUNCE_WEST,
    ]);

    game.apply(TeamId::HOME, sack("H1", "A1")).unwrap();

    let diver = game.state().goblin(id("H2")).unwrap();
    assert_eq!(
        (diver.presence, diver.health, game.state().ball),
        (
            Presence::Down(Square::new(7, 5)),
            diver.characteristics.health,
            Ball::Loose {
                at: Square::new(5, 6)
            }
        )
    );
}

#[test]
fn given_diving_catches_when_two_sprinters_can_dive_then_only_the_highest_roll_dives() {
    let mut game = carrier_knocked_loose_next_to_a_sprinter()
        .goblin("A2", Sprinter, 5, 5)
        .game([ATTACK_ROLL, DEFENCE_ROLL, BOUNCE_NORTH, 3, 4]);

    let events = game.apply(TeamId::HOME, sack("H1", "A1")).unwrap();

    assert_eq!(
        (
            game.state().ball,
            game.state().goblin(id("H2")).unwrap().presence,
            events.contains(&GameEvent::DiveContested {
                rolls: vec![
                    DiveRoll {
                        goblin: id("H2"),
                        roll: dive_roll(3, 1)
                    },
                    DiveRoll {
                        goblin: id("A2"),
                        roll: dive_roll(4, 1)
                    },
                ]
            })
        ),
        (
            Ball::Carried { by: id("A2") },
            Presence::Standing(Square::new(7, 5)),
            true
        )
    );
}

#[test]
fn given_diving_catches_when_two_sprinters_tie_then_they_roll_again_and_the_winner_dives_on_its_first_roll()
 {
    let mut game = carrier_knocked_loose_next_to_a_sprinter()
        .goblin("A2", Sprinter, 5, 5)
        .game([ATTACK_ROLL, DEFENCE_ROLL, BOUNCE_NORTH, 4, 4, 1, 2]);

    let events = game.apply(TeamId::HOME, sack("H1", "A1")).unwrap();

    assert_eq!(
        (
            game.state().ball,
            events.contains(&GameEvent::DiveAttempted {
                goblin: id("A2"),
                from: Square::new(5, 5),
                to: landing(),
                roll: dive_roll(4, 1),
            })
        ),
        (Ball::Carried { by: id("A2") }, true)
    );
}

#[test]
fn given_diving_catches_when_the_ball_lands_next_to_a_bruiser_only_then_it_rests_on_its_square() {
    let mut game = carrier_knocked_loose_next_to_a_sprinter()
        .edit("H2", |goblin| goblin.archetype = Bruiser)
        .game([ATTACK_ROLL, DEFENCE_ROLL, BOUNCE_NORTH]);

    game.apply(TeamId::HOME, sack("H1", "A1")).unwrap();

    assert_eq!(game.state().ball, Ball::Loose { at: landing() });
}

#[test]
fn given_rules_without_diving_catches_when_the_ball_lands_next_to_a_sprinter_then_it_rests_on_its_square()
 {
    let mut game = carrier_knocked_loose_next_to_a_sprinter()
        .rules(|ruleset| ruleset.sprinters_dive_for_loose_balls = false)
        .game([ATTACK_ROLL, DEFENCE_ROLL, BOUNCE_NORTH]);

    game.apply(TeamId::HOME, sack("H1", "A1")).unwrap();

    assert_eq!(game.state().ball, Ball::Loose { at: landing() });
}
