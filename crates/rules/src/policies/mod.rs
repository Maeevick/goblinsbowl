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
    Cautious2,
    /// The scripted coach with a bold appetite.
    Bold2,
    /// The scripted coach cautious while many goblins are left to play and bold once few are.
    #[default]
    Balanced2,
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
            Bot::Cautious2 => scripted::styled_command(
                arena,
                ruleset,
                state,
                team,
                Style::fixed(Appetite::cautious()),
            ),
            Bot::Bold2 => scripted::styled_command(
                arena,
                ruleset,
                state,
                team,
                Style::fixed(Appetite::bold()),
            ),
            Bot::Balanced2 => {
                scripted::styled_command(arena, ruleset, state, team, Style::balanced())
            }
        }
    }
}

/// A bot name that is not one of the built-in coaches.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("unknown bot {0}: random, cautious-2, balanced-2 or bold-2")]
pub struct UnknownBot(pub String);

impl FromStr for Bot {
    type Err = UnknownBot;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "random" => Ok(Bot::Random),
            "cautious-2" => Ok(Bot::Cautious2),
            "bold-2" => Ok(Bot::Bold2),
            "balanced-2" => Ok(Bot::Balanced2),
            other => Err(UnknownBot(other.to_string())),
        }
    }
}

impl fmt::Display for Bot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Bot::Random => "random",
            Bot::Cautious2 => "cautious-2",
            Bot::Bold2 => "bold-2",
            Bot::Balanced2 => "balanced-2",
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
    fn given_the_name_bold_2_when_reading_a_bot_then_the_bold_coach() {
        assert_eq!("bold-2".parse(), Ok(Bot::Bold2));
    }

    #[test]
    fn given_the_retired_first_cautious_coach_when_reading_its_name_then_no_bot() {
        assert_eq!(
            "cautious".parse::<Bot>(),
            Err(UnknownBot("cautious".to_string()))
        );
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
        assert_eq!(Bot::Balanced2.to_string().parse(), Ok(Bot::Balanced2));
    }

    #[test]
    fn given_the_second_cautious_coach_when_writing_its_name_then_cautious_2_reads_back() {
        assert_eq!(
            (Bot::Cautious2.to_string(), "cautious-2".parse()),
            ("cautious-2".to_string(), Ok(Bot::Cautious2))
        );
    }

    #[test]
    fn given_the_cautious_bot_when_choosing_then_it_plays_like_the_coach_with_a_cautious_appetite()
    {
        let (game, _) = Game::start(
            Ruleset::default(),
            [Roster::DEFAULT, Roster::DEFAULT],
            SeededDice::from_seed(5),
        )
        .unwrap();
        let team = game.state().active_team().unwrap();
        assert_eq!(
            Bot::Cautious2.command(
                game.arena(),
                game.ruleset(),
                game.state(),
                team,
                &mut ScriptedDice::new([])
            ),
            scripted::styled_command(
                game.arena(),
                game.ruleset(),
                game.state(),
                team,
                Style::fixed(Appetite::cautious())
            )
        );
    }
}
