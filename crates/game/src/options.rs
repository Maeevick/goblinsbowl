//! Command-line options of the desktop game.

use std::path::PathBuf;

use bevy::prelude::Resource;
use clap::{Parser, ValueEnum};
use goblins_rules::host::{MatchSetup, SeatKind};
use goblins_rules::policies::Bot;
use goblins_rules::ruleset::Ruleset;
use goblins_server::view::OddsHints;

use crate::texts::Language;

/// Who coaches a team in the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum LocalSeat {
    /// Someone at the keyboard and mouse.
    Human,
    /// The built-in bot.
    Bot,
    /// An AI agent coaching through the MCP endpoint the window opens.
    Agent,
}

impl From<LocalSeat> for SeatKind {
    fn from(seat: LocalSeat) -> Self {
        match seat {
            LocalSeat::Human => SeatKind::Human,
            LocalSeat::Bot => SeatKind::Bot,
            LocalSeat::Agent => SeatKind::Agent,
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
    /// How the bot seats play: random, cautious or bold.
    #[arg(long, default_value_t = Bot::Cautious)]
    pub bot: Bot,
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
    /// Local port of the MCP endpoint opened when a seat is an agent.
    #[arg(long, default_value_t = 7878)]
    pub port: u16,
    /// How much odds information agents receive.
    #[arg(long, value_enum, default_value_t = OddsHints::Full)]
    pub odds_hints: OddsHints,
    /// Bearer token of a home agent; random when left out.
    #[arg(long, env = "GB_HOME_TOKEN", hide_env_values = true)]
    pub home_token: Option<String>,
    /// Bearer token of an away agent; random when left out.
    #[arg(long, env = "GB_AWAY_TOKEN", hide_env_values = true)]
    pub away_token: Option<String>,
    /// Folder where match records are saved, with the tool calls of a match against an agent.
    #[arg(long, default_value = "runs")]
    pub record_dir: PathBuf,
    /// Save the record of a match without agent to the record folder, at the final whistle and
    /// when the window closes; a match against an agent is always recorded.
    #[arg(long)]
    pub record: bool,
}

impl GameOptions {
    /// Whether an agent coaches a seat, so the match runs on an embedded server.
    pub fn has_agent(&self) -> bool {
        self.home == LocalSeat::Agent || self.away == LocalSeat::Agent
    }

    /// Folder a match without agent saves its record to, when asked to record it.
    pub fn local_record_dir(&self) -> Option<PathBuf> {
        self.record.then(|| self.record_dir.clone())
    }

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
    fn given_the_record_flag_when_parsed_then_a_match_without_agent_is_saved_to_the_runs_folder() {
        assert_eq!(
            parse(&["--away", "bot", "--record"]).local_record_dir(),
            Some(PathBuf::from("runs"))
        );
    }

    #[test]
    fn given_no_record_flag_when_parsed_then_a_match_without_agent_is_not_saved() {
        assert_eq!(parse(&["--away", "bot"]).local_record_dir(), None);
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
    fn given_an_agent_away_when_building_the_match_then_human_versus_agent() {
        let setup = parse(&["--away", "agent"]).match_setup(Ruleset::default(), 3);
        assert_eq!(setup.seats, [SeatKind::Human, SeatKind::Agent]);
    }

    #[test]
    fn given_an_agent_away_when_asked_then_the_match_needs_the_server() {
        assert!(parse(&["--away", "agent"]).has_agent());
    }

    #[test]
    fn given_a_bot_away_when_asked_then_the_match_stays_local() {
        assert!(!parse(&["--away", "bot"]).has_agent());
    }

    #[test]
    fn given_no_bot_style_when_parsed_then_the_cautious_bot() {
        assert_eq!(parse(&[]).bot, Bot::Cautious);
    }

    #[test]
    fn given_a_bold_bot_when_parsed_then_the_bold_bot() {
        assert_eq!(parse(&["--bot", "bold"]).bot, Bot::Bold);
    }

    #[test]
    fn given_an_unknown_bot_when_parsed_then_refused() {
        let parsed = GameOptions::try_parse_from(["goblinsbowl", "--bot", "clever"]);
        assert!(parsed.is_err());
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
