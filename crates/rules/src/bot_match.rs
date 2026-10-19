//! A whole match between two bots, played to its final whistle without any clock.

use thiserror::Error;

use crate::dice::SeededDice;
use crate::event::GameEvent;
use crate::game::SetupError;
use crate::host::{HostError, MatchHost, MatchRecord, MatchSetup, SeatKind};
use crate::policies::Bot;
use crate::roster::Roster;
use crate::ruleset::Ruleset;

/// Commands a bot match may take before it counts as stuck.
const MOST_COMMANDS: usize = 4_000;

/// Why a bot match could not be played out.
#[derive(Debug, Error)]
pub enum BotMatchError {
    /// The match cannot start with this setup.
    #[error(transparent)]
    Setup(#[from] SetupError),
    /// A bot gave a command the rules refuse.
    #[error(transparent)]
    Refused(#[from] HostError),
    /// The bots did not reach the final whistle.
    #[error("the match on seed {seed} did not finish within {MOST_COMMANDS} commands")]
    Unfinished {
        /// The match's dice seed.
        seed: u64,
    },
}

/// The match `coaches`, home then away, play under `ruleset` with `rosters` on dice `seed`, to its
/// final whistle: its record and every event, in order.
///
/// # Errors
/// [`BotMatchError`] when the match cannot start, a bot is refused or the match never ends.
pub fn play_bot_match(
    ruleset: &Ruleset,
    rosters: [Roster; 2],
    seed: u64,
    coaches: [Bot; 2],
) -> Result<(MatchRecord, Vec<GameEvent>), BotMatchError> {
    let (mut host, _) = MatchHost::start(MatchSetup {
        ruleset: ruleset.clone(),
        rosters,
        seed,
        seats: [SeatKind::Bot, SeatKind::Bot],
        turn_seconds: [ruleset.turn_seconds, ruleset.turn_seconds],
        agent_ready_grace_seconds: 15,
    })?;
    let mut coach_dice = SeededDice::from_seed(seed.wrapping_add(1_000_003));
    for _ in 0..MOST_COMMANDS {
        let Some(team) = host.game().state().active_team() else {
            return Ok((host.record().clone(), host.events_since(0).to_vec()));
        };
        let game = host.game();
        let command = coaches[team.index().min(1)].command(
            game.arena(),
            game.ruleset(),
            game.state(),
            team,
            &mut coach_dice,
        );
        host.submit(team, command)?;
    }
    Err(BotMatchError::Unfinished { seed })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Phase;

    #[test]
    fn given_two_second_generation_cautious_coaches_when_they_play_then_the_record_reaches_the_final_whistle()
     {
        let ruleset = Ruleset::default();
        let (record, _) = play_bot_match(
            &ruleset,
            [ruleset.default_roster; 2],
            1,
            [Bot::Cautious2, Bot::Cautious2],
        )
        .unwrap();
        assert_eq!(record.replay().unwrap().state().phase, Phase::Finished);
    }
}
