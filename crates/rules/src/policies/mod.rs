//! Scripted coaches: they read the state and return the next command.

use std::fmt;
use std::str::FromStr;

use crate::arena::Arena;
use crate::command::Command;
use crate::dice::DiceRoller;
use crate::ruleset::Ruleset;
use crate::state::GameState;
use crate::team::TeamId;

pub mod random;
pub mod scripted;

/// A built-in coach that can sit in a seat.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Bot {
    /// Legal commands at random.
    Random,
    /// The scripted coach with a cautious appetite.
    #[default]
    Cautious,
    /// The scripted coach with a bold appetite.
    Bold,
}

impl Bot {
    /// The next command of this bot for `team`; only the random bot rolls `dice`.
    pub fn command(
        self,
        arena: &Arena,
        ruleset: &Ruleset,
        state: &GameState,
        team: TeamId,
        dice: &mut impl DiceRoller,
    ) -> Command {
        match self {
            Bot::Random => random::random_command(arena, ruleset, state, team, dice),
            Bot::Cautious => scripted::cautious_command(arena, ruleset, state, team),
            Bot::Bold => scripted::bold_command(arena, ruleset, state, team),
        }
    }
}

/// A bot name that is not `random`, `cautious` or `bold`.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("unknown bot {0}: random, cautious or bold")]
pub struct UnknownBot(pub String);

impl FromStr for Bot {
    type Err = UnknownBot;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "random" => Ok(Bot::Random),
            "cautious" => Ok(Bot::Cautious),
            "bold" => Ok(Bot::Bold),
            other => Err(UnknownBot(other.to_string())),
        }
    }
}

impl fmt::Display for Bot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Bot::Random => "random",
            Bot::Cautious => "cautious",
            Bot::Bold => "bold",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dice::{ScriptedDice, SeededDice};
    use crate::game::Game;
    use crate::roster::Roster;

    #[test]
    fn given_the_name_bold_when_reading_a_bot_then_the_bold_coach() {
        assert_eq!("bold".parse(), Ok(Bot::Bold));
    }

    #[test]
    fn given_an_unknown_name_when_reading_a_bot_then_an_error_naming_the_choices() {
        assert_eq!(
            "clever".parse::<Bot>(),
            Err(UnknownBot("clever".to_string()))
        );
    }

    #[test]
    fn given_a_bot_when_writing_its_name_then_it_reads_back_the_same() {
        assert_eq!(Bot::Cautious.to_string().parse(), Ok(Bot::Cautious));
    }

    #[test]
    fn given_the_cautious_bot_when_choosing_then_it_plays_like_the_cautious_coach() {
        let (game, _) = Game::start(
            Ruleset::default(),
            [Roster::DEFAULT, Roster::DEFAULT],
            SeededDice::from_seed(5),
        )
        .unwrap();
        let team = game.state().active_team().unwrap();
        assert_eq!(
            Bot::Cautious.command(
                game.arena(),
                game.ruleset(),
                game.state(),
                team,
                &mut ScriptedDice::new([])
            ),
            scripted::cautious_command(game.arena(), game.ruleset(), game.state(), team)
        );
    }
}
