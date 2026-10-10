//! Command-line options of the desktop game.

use std::path::PathBuf;

use bevy::prelude::Resource;
use clap::{Parser, ValueEnum};
use goblins_rules::host::{MatchSetup, SeatKind};
use goblins_rules::ruleset::Ruleset;

use crate::texts::Language;

/// Who coaches a team in the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum LocalSeat {
    /// Someone at the keyboard and mouse.
    Human,
    /// The built-in bot.
    Bot,
}

impl From<LocalSeat> for SeatKind {
    fn from(seat: LocalSeat) -> Self {
        match seat {
            LocalSeat::Human => SeatKind::Human,
            LocalSeat::Bot => SeatKind::Bot,
        }
    }
}

/// Goblins Bowl: hot-seat goblin football in a bowl.
#[derive(Clone, Debug, Parser, Resource)]
#[command(name = "goblinsbowl", version, about)]
pub struct GameOptions {
    /// Who plays the home team (blue, west den).
    #[arg(long, value_enum, default_value_t = LocalSeat::Human)]
    pub home: LocalSeat,
    /// Who plays the away team (gold, east den).
    #[arg(long, value_enum, default_value_t = LocalSeat::Human)]
    pub away: LocalSeat,
    /// Dice seed; random when left out.
    #[arg(long)]
    pub seed: Option<u64>,
    /// Seconds per turn for both teams.
    #[arg(long, default_value_t = 60)]
    pub turn_seconds: u16,
    /// Seconds per turn for the home team, overriding --turn-seconds.
    #[arg(long)]
    pub home_turn_seconds: Option<u16>,
    /// Seconds per turn for the away team, overriding --turn-seconds.
    #[arg(long)]
    pub away_turn_seconds: Option<u16>,
    /// Rules file; the built-in rules are used when it does not exist.
    #[arg(long, default_value = "rules.toml")]
    pub rules: PathBuf,
    /// Language of the screens.
    #[arg(long, value_enum, default_value_t = Language::French)]
    pub lang: Language,
}

impl GameOptions {
    /// The match these options describe, with `ruleset` and dice `seed`.
    pub fn match_setup(&self, ruleset: Ruleset, seed: u64) -> MatchSetup {
        MatchSetup {
            rosters: [ruleset.default_roster; 2],
            ruleset,
            seed,
            seats: [self.home.into(), self.away.into()],
            turn_seconds: [
                self.home_turn_seconds.unwrap_or(self.turn_seconds),
                self.away_turn_seconds.unwrap_or(self.turn_seconds),
            ],
            agent_ready_grace_seconds: 15,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(arguments: &[&str]) -> GameOptions {
        GameOptions::try_parse_from(std::iter::once("goblinsbowl").chain(arguments.iter().copied()))
            .unwrap()
    }

    #[test]
    fn given_no_arguments_when_parsed_then_two_humans_in_french() {
        let options = parse(&[]);
        assert_eq!(
            (options.home, options.away, options.lang),
            (LocalSeat::Human, LocalSeat::Human, Language::French)
        );
    }

    #[test]
    fn given_ninety_seconds_for_home_when_building_the_match_then_ninety_and_sixty() {
        let setup = parse(&["--home-turn-seconds", "90"]).match_setup(Ruleset::default(), 3);
        assert_eq!(setup.turn_seconds, [90, 60]);
    }

    #[test]
    fn given_a_bot_away_when_building_the_match_then_human_versus_bot() {
        let setup = parse(&["--away", "bot"]).match_setup(Ruleset::default(), 3);
        assert_eq!(setup.seats, [SeatKind::Human, SeatKind::Bot]);
    }

    #[test]
    fn given_a_seed_when_building_the_match_then_the_dice_use_it() {
        assert_eq!(parse(&[]).match_setup(Ruleset::default(), 77).seed, 77);
    }
}
