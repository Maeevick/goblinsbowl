//! Scripted coaches: they read the state and return the next command.

use std::fmt;
use std::str::FromStr;

use crate::arena::Arena;
use crate::command::Command;
use crate::dice::DiceRoller;
use crate::ruleset::Ruleset;
use crate::state::GameState;
use crate::team::TeamId;

use scripted::{Appetite, Style};

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
    /// The second-generation scripted coach with a cautious appetite.
    Cautious2,
    /// The second-generation scripted coach with a bold appetite.
    Bold2,
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
            Bot::Cautious2 => scripted::styled_command(
                arena,
                ruleset,
                state,
                team,
                Style::second(Appetite::cautious()),
            ),
            Bot::Bold2 => scripted::styled_command(
                arena,
                ruleset,
                state,
                team,
                Style::second(Appetite::bold()),
            ),
        }
    }
}

/// A bot name that is not one of the built-in coaches.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("unknown bot {0}: random, cautious, bold, cautious-2 or bold-2")]
pub struct UnknownBot(pub String);

impl FromStr for Bot {
    type Err = UnknownBot;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "random" => Ok(Bot::Random),
            "cautious" => Ok(Bot::Cautious),
            "bold" => Ok(Bot::Bold),
            "cautious-2" => Ok(Bot::Cautious2),
            "bold-2" => Ok(Bot::Bold2),
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
            Bot::Cautious2 => "cautious-2",
            Bot::Bold2 => "bold-2",
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
    fn given_the_second_cautious_coach_when_writing_its_name_then_cautious_2_reads_back() {
        assert_eq!(
            (Bot::Cautious2.to_string(), "cautious-2".parse()),
            ("cautious-2".to_string(), Ok(Bot::Cautious2))
        );
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
