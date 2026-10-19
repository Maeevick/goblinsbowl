//! Plays many seeded matches between scripted coaches and reports how the rules play out.
//!
//! `make balance ARGS="--games 1000 --home cautious --away bold --rules rules.toml --seed 1"`;
//! coaches are `random`, `cautious`, `bold`, `cautious-2`, `balanced-2` or `bold-2`. With `--risk`,
//! every match is also replayed to report each coach's own safe-first index, as the assessment
//! reads it for agents. `--arena circle|cross|rectangle` and `--arena-size tiny|medium|huge` play
//! in another bowl than the rules file's.

use std::error::Error;
use std::path::PathBuf;
use std::time::Instant;

use goblins_rules::assessment::{ActivationRow, activation_rows, summarise};
use goblins_rules::balance::{BalanceSummary, MatchStats};
use goblins_rules::bot_match::play_bot_match;
use goblins_rules::bowls::{BowlShape, BowlSize, with_bowl};
use goblins_rules::host::MatchRecord;
use goblins_rules::policies::Bot;
use goblins_rules::ruleset::Ruleset;
use goblins_rules::team::TeamId;

struct Options {
    games: u64,
    coaches: [Bot; 2],
    rules: Option<PathBuf>,
    seed: u64,
    risk: bool,
    arena: Option<BowlShape>,
    arena_size: Option<BowlSize>,
}

fn options() -> Result<Options, String> {
    let mut options = Options {
        games: 200,
        coaches: [Bot::Cautious, Bot::Cautious],
        rules: None,
        seed: 1,
        risk: false,
        arena: None,
        arena_size: None,
    };
    let mut arguments = std::env::args().skip(1);
    while let Some(flag) = arguments.next() {
        if flag == "--risk" {
            options.risk = true;
            continue;
        }
        let value = arguments
            .next()
            .ok_or_else(|| format!("{flag} needs a value"))?;
        match flag.as_str() {
            "--games" => {
                options.games = value.parse().map_err(|_| format!("bad --games {value}"))?
            }
            "--home" => options.coaches[0] = value.parse().map_err(|error| format!("{error}"))?,
            "--away" => options.coaches[1] = value.parse().map_err(|error| format!("{error}"))?,
            "--rules" => options.rules = Some(PathBuf::from(value)),
            "--seed" => options.seed = value.parse().map_err(|_| format!("bad --seed {value}"))?,
            "--arena" => options.arena = Some(value.parse().map_err(|error| format!("{error}"))?),
            "--arena-size" => {
                options.arena_size = Some(value.parse().map_err(|error| format!("{error}"))?)
            }
            other => return Err(format!("unknown flag {other}")),
        }
    }
    Ok(options)
}

fn play(
    ruleset: &Ruleset,
    seed: u64,
    coaches: [Bot; 2],
) -> Result<(MatchStats, MatchRecord), Box<dyn Error>> {
    let (record, events) = play_bot_match(ruleset, [ruleset.default_roster; 2], seed, coaches)?;
    Ok((MatchStats::from_events(&events), record))
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = options()?;
    let ruleset = with_bowl(
        match &options.rules {
            Some(path) => Ruleset::from_toml_str(&std::fs::read_to_string(path)?)?,
            None => Ruleset::default(),
        },
        options.arena,
        options.arena_size,
    );
    let started = Instant::now();
    let mut summary = BalanceSummary::default();
    let mut rows_by_team: [Vec<ActivationRow>; 2] = [Vec::new(), Vec::new()];
    for game in 0..options.games {
        let (stats, record) = play(&ruleset, options.seed.wrapping_add(game), options.coaches)?;
        summary.add(&stats);
        if options.risk {
            let turns_apart = u32::try_from(game)
                .unwrap_or(u32::MAX)
                .saturating_mul(1_000);
            for row in activation_rows(&record)? {
                rows_by_team[row.team.index().min(1)].push(ActivationRow {
                    turn: row.turn.saturating_add(turns_apart),
                    ..row
                });
            }
        }
    }
    println!(
        "{} (home) vs {} (away), seeds {}..{}, rules {}",
        options.coaches[0],
        options.coaches[1],
        options.seed,
        options.seed.wrapping_add(options.games),
        options
            .rules
            .as_ref()
            .map_or("built-in defaults".to_string(), |path| path
                .display()
                .to_string())
    );
    for line in summary.report() {
        println!("{line}");
    }
    if options.risk {
        let index = |team: TeamId| {
            summarise(&rows_by_team[team.index().min(1)], team)
                .safe_first_index
                .map_or("none".to_string(), |index| format!("{index:.2}"))
        };
        println!(
            "Safe-first index: home {}, away {}",
            index(TeamId::HOME),
            index(TeamId::AWAY)
        );
    }
    println!("Played in {:.1} s", started.elapsed().as_secs_f64());
    Ok(())
}
