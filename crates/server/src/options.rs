//! Command-line options of the headless match server.

use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use goblins_rules::bowls::{BowlShape, BowlSize};
use goblins_rules::host::SeatKind;
use goblins_rules::policies::Bot;

use crate::view::OddsHints;

/// Who coaches a seat on the headless server.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum RemoteSeat {
    /// An AI agent connected through MCP.
    Agent,
    /// The built-in bot chosen with --bot.
    Bot,
}

impl From<RemoteSeat> for SeatKind {
    fn from(seat: RemoteSeat) -> Self {
        match seat {
            RemoteSeat::Agent => SeatKind::Agent,
            RemoteSeat::Bot => SeatKind::Bot,
        }
    }
}

/// Hosts one Goblins Bowl match that AI agents coach through MCP.
#[derive(Clone, Debug, Parser)]
#[command(name = "goblinsbowl-server", version, about)]
pub struct ServerOptions {
    /// Who coaches the home team (west den).
    #[arg(long, value_enum, default_value_t = RemoteSeat::Agent)]
    pub home: RemoteSeat,
    /// Who coaches the away team (east den).
    #[arg(long, value_enum, default_value_t = RemoteSeat::Bot)]
    pub away: RemoteSeat,
    /// How the bot seats play: random, cautious, bold, cautious-2, balanced-2 or bold-2.
    #[arg(long, default_value_t = Bot::Cautious)]
    pub bot: Bot,
    /// Dice seed; random when left out.
    #[arg(long)]
    pub seed: Option<u64>,
    /// Local port of the MCP endpoint.
    #[arg(long, default_value_t = 7878)]
    pub port: u16,
    /// Seconds per turn for both seats.
    #[arg(long, default_value_t = 60)]
    pub turn_seconds: u16,
    /// Seconds per turn for the home seat, overriding --turn-seconds.
    #[arg(long)]
    pub home_turn_seconds: Option<u16>,
    /// Seconds per turn for the away seat, overriding --turn-seconds.
    #[arg(long)]
    pub away_turn_seconds: Option<u16>,
    /// Seconds an agent's turn waits for the agent to look at it before its clock starts anyway.
    #[arg(long, default_value_t = 15)]
    pub agent_grace_seconds: u16,
    /// Bearer token of the home seat; random when left out.
    #[arg(long, env = "GB_HOME_TOKEN", hide_env_values = true)]
    pub home_token: Option<String>,
    /// Bearer token of the away seat; random when left out.
    #[arg(long, env = "GB_AWAY_TOKEN", hide_env_values = true)]
    pub away_token: Option<String>,
    /// How much odds information agents receive.
    #[arg(long, value_enum, default_value_t = OddsHints::Full)]
    pub odds_hints: OddsHints,
    /// Pause between two bot commands, in milliseconds.
    #[arg(long, default_value_t = 400)]
    pub bot_delay_ms: u64,
    /// Rules file; the built-in rules are used when it does not exist.
    #[arg(long, default_value = "rules.toml")]
    pub rules: PathBuf,
    /// Rules document agents read with get_rules, its English half from its "# Goblins Bowl, rules"
    /// heading on; the built-in docs/rules.md when left out.
    #[arg(long)]
    pub rules_text: Option<PathBuf>,
    /// Shape of the bowl: circle, cross or rectangle; the rules file's bowl when left out.
    #[arg(long)]
    pub arena: Option<BowlShape>,
    /// Size of the bowl: tiny (21 squares from den to den), medium (31) or huge (51).
    #[arg(long)]
    pub arena_size: Option<BowlSize>,
    /// Folder where the match record is saved at the final whistle.
    #[arg(long, default_value = "runs")]
    pub record_dir: PathBuf,
}

impl ServerOptions {
    /// Seconds per turn, home then away.
    pub fn turn_seconds_per_seat(&self) -> [u16; 2] {
        [
            self.home_turn_seconds.unwrap_or(self.turn_seconds),
            self.away_turn_seconds.unwrap_or(self.turn_seconds),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(arguments: &[&str]) -> ServerOptions {
        ServerOptions::try_parse_from(
            std::iter::once("goblinsbowl-server").chain(arguments.iter().copied()),
        )
        .unwrap()
    }

    #[test]
    fn given_no_arguments_when_parsed_then_an_agent_plays_home_against_the_bot() {
        let options = parse(&[]);
        assert_eq!(
            (options.home, options.away),
            (RemoteSeat::Agent, RemoteSeat::Bot)
        );
    }

    #[test]
    fn given_a_slower_home_seat_when_parsed_then_home_gets_ninety_seconds_and_away_sixty() {
        assert_eq!(
            parse(&["--home-turn-seconds", "90"]).turn_seconds_per_seat(),
            [90, 60]
        );
    }

    #[test]
    fn given_a_frozen_rules_document_when_parsed_then_agents_read_it() {
        assert_eq!(
            parse(&["--rules-text", "assessment/rules-0.10.md"]).rules_text,
            Some(PathBuf::from("assessment/rules-0.10.md"))
        );
    }

    #[test]
    fn given_no_bot_style_when_parsed_then_the_cautious_bot() {
        assert_eq!(parse(&[]).bot, Bot::Cautious);
    }

    #[test]
    fn given_a_random_bot_when_parsed_then_the_random_bot() {
        assert_eq!(parse(&["--bot", "random"]).bot, Bot::Random);
    }

    #[test]
    fn given_hidden_odds_when_parsed_then_none() {
        assert_eq!(parse(&["--odds-hints", "none"]).odds_hints, OddsHints::None);
    }
}
