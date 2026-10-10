//! Plays many seeded matches between scripted coaches and reports how the rules play out.
//!
//! `make balance ARGS="--games 1000 --home cautious --away bold --rules rules.toml --seed 1"`;
//! coaches are `cautious`, `bold` or `random`.

use std::error::Error;
use std::path::PathBuf;
use std::time::Instant;

use goblins_rules::balance::{BalanceSummary, MatchStats};
use goblins_rules::command::Command;
use goblins_rules::dice::SeededDice;
use goblins_rules::host::{MatchHost, MatchSetup, SeatKind};
use goblins_rules::policies::random::random_command;
use goblins_rules::policies::scripted::{bold_command, cautious_command};
use goblins_rules::ruleset::Ruleset;
use goblins_rules::team::TeamId;

const COMMAND_LIMIT: usize = 4_000;

#[derive(Clone, Copy, Debug)]
enum Coach {
    Random,
    Cautious,
    Bold,
}

impl Coach {
    fn parse(name: &str) -> Result<Self, String> {
        match name {
            "random" => Ok(Coach::Random),
            "cautious" => Ok(Coach::Cautious),
            "bold" => Ok(Coach::Bold),
            other => Err(format!("unknown coach {other}: random, cautious or bold")),
        }
    }

    fn command(self, host: &MatchHost, team: TeamId, dice: &mut SeededDice) -> Command {
        let game = host.game();
        match self {
            Coach::Random => random_command(game.arena(), game.ruleset(), game.state(), team, dice),
            Coach::Cautious => cautious_command(game.arena(), game.ruleset(), game.state(), team),
            Coach::Bold => bold_command(game.arena(), game.ruleset(), game.state(), team),
        }
    }
}

struct Options {
    games: u64,
    coaches: [Coach; 2],
    rules: Option<PathBuf>,
    seed: u64,
}

fn options() -> Result<Options, String> {
    let mut options = Options {
        games: 200,
        coaches: [Coach::Cautious, Coach::Cautious],
        rules: None,
        seed: 1,
    };
    let mut arguments = std::env::args().skip(1);
    while let Some(flag) = arguments.next() {
        let value = arguments
            .next()
            .ok_or_else(|| format!("{flag} needs a value"))?;
        match flag.as_str() {
            "--games" => {
                options.games = value.parse().map_err(|_| format!("bad --games {value}"))?
            }
            "--home" => options.coaches[0] = Coach::parse(&value)?,
            "--away" => options.coaches[1] = Coach::parse(&value)?,
            "--rules" => options.rules = Some(PathBuf::from(value)),
            "--seed" => options.seed = value.parse().map_err(|_| format!("bad --seed {value}"))?,
            other => return Err(format!("unknown flag {other}")),
        }
    }
    Ok(options)
}

fn play(ruleset: &Ruleset, seed: u64, coaches: [Coach; 2]) -> Result<MatchStats, Box<dyn Error>> {
    let (mut host, _) = MatchHost::start(MatchSetup {
        ruleset: ruleset.clone(),
        rosters: [ruleset.default_roster, ruleset.default_roster],
        seed,
        seats: [SeatKind::Bot, SeatKind::Bot],
        turn_seconds: [ruleset.turn_seconds, ruleset.turn_seconds],
        agent_ready_grace_seconds: 15,
    })?;
    let mut coach_dice = SeededDice::from_seed(seed.wrapping_add(1_000_003));
    for _ in 0..COMMAND_LIMIT {
        let Some(team) = host.game().state().active_team() else {
            return Ok(MatchStats::from_events(host.events_since(0)));
        };
        let coach = coaches[team.index()];
        let command = coach.command(&host, team, &mut coach_dice);
        host.submit(team, command)?;
    }
    Err(format!("match {seed} did not finish").into())
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = options()?;
    let ruleset = match &options.rules {
        Some(path) => Ruleset::from_toml_str(&std::fs::read_to_string(path)?)?,
        None => Ruleset::default(),
    };
    let started = Instant::now();
    let mut summary = BalanceSummary::default();
    for game in 0..options.games {
        summary.add(&play(
            &ruleset,
            options.seed.wrapping_add(game),
            options.coaches,
        )?);
    }
    println!(
        "{:?} (home) vs {:?} (away), seeds {}..{}, rules {}",
        options.coaches[0],
        options.coaches[1],
        options.seed,
        options.seed.wrapping_add(options.games),
        options
            .rules
            .as_ref()
            .map_or("0.1 defaults".to_string(), |path| path
                .display()
                .to_string())
    );
    for line in summary.report() {
        println!("{line}");
    }
    println!("Played in {:.1} s", started.elapsed().as_secs_f64());
    Ok(())
}
