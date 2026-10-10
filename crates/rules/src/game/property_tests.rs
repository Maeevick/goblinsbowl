//! Whole random matches: invariants that must hold after every command, for any seed.

use std::collections::HashSet;

use proptest::prelude::*;

use crate::dice::SeededDice;
use crate::game::Game;
use crate::host::{MatchHost, MatchSetup, SeatKind};
use crate::policies::random::random_command;
use crate::roster::Roster;
use crate::ruleset::Ruleset;
use crate::state::{Ball, GameState, Phase, Presence};

const COMMAND_LIMIT: usize = 4_000;

fn broken_invariant(game: &Game<SeededDice>) -> Option<String> {
    let state: &GameState = game.state();
    let squares: Vec<_> = state
        .goblins
        .iter()
        .filter_map(|goblin| goblin.square())
        .collect();
    let distinct: HashSet<_> = squares.iter().collect();
    if distinct.len() != squares.len() {
        return Some("two goblins share a square".into());
    }
    if squares
        .iter()
        .any(|square| !game.arena().is_in_bounds(*square))
    {
        return Some("a goblin stands out of bounds".into());
    }
    match state.ball {
        Ball::Carried { by } => {
            if !state
                .goblin(by)
                .is_some_and(|carrier| carrier.is_standing())
            {
                return Some(format!("the ball is held by {by}, who is not standing"));
            }
        }
        Ball::Loose { at } => {
            if !game.arena().is_in_bounds(at) || state.goblin_at(at).is_some() {
                return Some(format!("the loose ball lies on an invalid square {at:?}"));
            }
        }
    }
    for goblin in &state.goblins {
        let knocked_out = goblin.presence == Presence::KnockedOut;
        if goblin.health > goblin.characteristics.health || knocked_out != (goblin.health == 0) {
            return Some(format!(
                "{} has inconsistent health {}",
                goblin.id, goblin.health
            ));
        }
    }
    if state
        .turns_played
        .iter()
        .any(|played| *played > game.ruleset().turns_per_half)
    {
        return Some("a team played more turns than a half allows".into());
    }
    None
}

fn play_random_match(match_seed: u64, coach_seed: u64) -> Result<MatchHost, String> {
    let (mut host, _) = MatchHost::start(MatchSetup {
        ruleset: Ruleset::default(),
        rosters: [Roster::DEFAULT, Roster::DEFAULT],
        seed: match_seed,
        seats: [SeatKind::Bot, SeatKind::Bot],
        turn_seconds: [60, 60],
    })
    .map_err(|error| error.to_string())?;
    let mut coach_dice = SeededDice::from_seed(coach_seed);
    for _ in 0..COMMAND_LIMIT {
        let Some(team) = host.game().state().active_team() else {
            return Ok(host);
        };
        let game = host.game();
        let command = random_command(
            game.arena(),
            game.ruleset(),
            game.state(),
            team,
            &mut coach_dice,
        );
        host.submit(team, command)
            .map_err(|error| error.to_string())?;
        if let Some(problem) = broken_invariant(host.game()) {
            return Err(problem);
        }
    }
    Err("the match did not finish".into())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn given_any_seeds_when_random_coaches_play_then_every_invariant_holds_and_the_match_ends(
        match_seed in any::<u64>(),
        coach_seed in any::<u64>(),
    ) {
        let host = play_random_match(match_seed, coach_seed);
        prop_assert!(host.is_ok(), "{:?}", host.err());
    }

    #[test]
    fn given_any_seeds_when_a_random_match_is_replayed_then_the_same_final_state(
        match_seed in any::<u64>(),
        coach_seed in any::<u64>(),
    ) {
        let host = play_random_match(match_seed, coach_seed).map_err(TestCaseError::fail)?;
        let replayed = host.record().replay().map_err(|error| TestCaseError::fail(error.to_string()))?;
        prop_assert_eq!(replayed.state(), &host.snapshot().state);
    }
}

#[test]
fn given_a_random_match_when_it_ends_then_the_phase_is_finished() {
    let host = play_random_match(1, 2).unwrap();
    assert_eq!(host.game().state().phase, Phase::Finished);
}
