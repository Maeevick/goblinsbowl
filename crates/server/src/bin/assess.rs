//! Prints how each coach of a run handled risk: `goblinsbowl-assess <record.jsonl> [<calls.jsonl>]`,
//! or a whole batch of `scripts/assessment-batch.sh` as Markdown, next to the yardstick bot replaying
//! the agent's seats against the same bot: `goblinsbowl-assess --batch <dir> [--yardstick <bot>]`,
//! cautious-2 unless told otherwise.
//!
//! The call log defaults to the file saved next to the record (`<record>-calls.jsonl`).

use std::path::{Path, PathBuf};

use anyhow::Context;
use goblins_rules::assessment::{activation_rows, summarise};
use goblins_rules::host::MatchRecord;
use goblins_rules::policies::Bot;
use goblins_rules::team::TeamId;
use goblins_server::assess::{
    BatchMatch, Yardstick, batch_report, calibration, estimates_by_entry, model_verdicts,
    refused_calls, team_of_seat, turns_lost_to_the_clock, yardstick_match,
};
use goblins_server::call_log::CallEntry;

fn calls_next_to(record: &Path) -> PathBuf {
    let name = record
        .file_stem()
        .map(|stem| format!("{}-calls.jsonl", stem.to_string_lossy()))
        .unwrap_or_default();
    record.with_file_name(name)
}

fn read_calls(path: &Path) -> anyhow::Result<Vec<CallEntry>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text =
        std::fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).context("a call log line is not a call"))
        .collect()
}

fn record_in(folder: &Path) -> anyhow::Result<PathBuf> {
    std::fs::read_dir(folder)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.contains("-seed")
                        && name.ends_with(".jsonl")
                        && !name.ends_with("-calls.jsonl")
                })
        })
        .with_context(|| format!("no match record in {}", folder.display()))
}

fn batch_match(folder: &Path) -> anyhow::Result<BatchMatch> {
    let summary: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(folder.join("match.json"))?)?;
    let seat = summary["seat"].as_str().unwrap_or_default();
    let agent =
        team_of_seat(seat).with_context(|| format!("unknown seat in {}", folder.display()))?;
    let record_path = record_in(folder)?;
    let record = MatchRecord::from_json_lines(&std::fs::read_to_string(&record_path)?)?;
    let calls = read_calls(&calls_next_to(&record_path))?;
    let finished = record.replay()?;
    let state = finished.state();
    let turns_per_half = u32::from(record.setup.ruleset.turns_per_half);
    let turns = |team: TeamId| {
        u32::from(state.half).saturating_sub(1) * turns_per_half
            + u32::from(
                state
                    .turns_played
                    .get(team.index())
                    .copied()
                    .unwrap_or_default(),
            )
    };
    Ok(BatchMatch {
        model: summary["model"].as_str().unwrap_or_default().to_string(),
        agent,
        rows: activation_rows(&record)?,
        goals: [
            state.score.first().copied().unwrap_or_default(),
            state.score.get(1).copied().unwrap_or_default(),
        ],
        turns: [turns(TeamId::HOME), turns(TeamId::AWAY)],
        turns_lost_to_the_clock: turns_lost_to_the_clock(&record, agent),
        refused_calls: refused_calls(&calls, agent),
        finished: summary["finished"].as_bool().unwrap_or(false),
        coach_runs: u32::try_from(summary["coach_runs"].as_u64().unwrap_or(1)).unwrap_or(1),
        cost_usd: summary["cost_usd"].as_f64().unwrap_or_default(),
        seconds: summary["match_seconds"].as_u64().unwrap_or_default(),
    })
}

/// The yardstick's match for the batch match in `folder`, when its summary names a bot that still
/// plays; none otherwise.
fn yardstick_for(folder: &Path, yardstick: Bot) -> anyhow::Result<Option<BatchMatch>> {
    let summary: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(folder.join("match.json"))?)?;
    let Some(opponent) = summary["bot"]
        .as_str()
        .and_then(|name| name.parse::<Bot>().ok())
    else {
        return Ok(None);
    };
    let agent = team_of_seat(summary["seat"].as_str().unwrap_or_default())
        .with_context(|| format!("unknown seat in {}", folder.display()))?;
    let record = MatchRecord::from_json_lines(&std::fs::read_to_string(record_in(folder)?)?)?;
    yardstick_match(&record.setup, agent, opponent, yardstick).map(Some)
}

fn print_batch(folder: &Path, yardstick: Bot) -> anyhow::Result<()> {
    let mut folders: Vec<PathBuf> = std::fs::read_dir(folder)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .flat_map(|model| {
            std::fs::read_dir(&model)
                .into_iter()
                .flatten()
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.join("match.json").exists())
                .collect::<Vec<_>>()
        })
        .collect();
    folders.sort();
    let matches = folders
        .iter()
        .map(|match_folder| batch_match(match_folder))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let mut seats_played = Vec::new();
    let mut yardstick_matches = Vec::new();
    for match_folder in &folders {
        let seat = match_folder
            .file_name()
            .map(|name| name.to_string_lossy().to_string());
        if seats_played.contains(&seat) {
            continue;
        }
        seats_played.push(seat);
        match yardstick_for(match_folder, yardstick)? {
            Some(played) => yardstick_matches.push(played),
            None => {
                yardstick_matches.clear();
                break;
            }
        }
    }
    let yardstick_verdict = model_verdicts(&yardstick_matches).into_iter().next();
    let yardstick = match (opponent_agent(&folders)?, &yardstick_verdict) {
        (Some(agent), _) => Yardstick::AgentOpponent(agent),
        (None, Some(verdict)) => Yardstick::Bot(verdict),
        (None, None) => Yardstick::Unavailable,
    };
    for line in batch_report(&model_verdicts(&matches), &yardstick) {
        println!("{line}");
    }
    Ok(())
}

/// The model coaching the other seat in the matches of `folders`, when an agent rather than a bot
/// played against the model assessed.
fn opponent_agent(folders: &[PathBuf]) -> anyhow::Result<Option<String>> {
    for folder in folders {
        let summary: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(folder.join("match.json"))?)?;
        if let Some(model) = summary["opponent_model"].as_str() {
            return Ok(Some(model.to_string()));
        }
    }
    Ok(None)
}

fn main() -> anyhow::Result<()> {
    let mut arguments = std::env::args().skip(1);
    let first = arguments.next();
    if first.as_deref() == Some("--batch") {
        let usage = "usage: goblinsbowl-assess --batch <batch-folder> [--yardstick <bot>]";
        let folder = arguments.next().context(usage)?;
        let yardstick = match (arguments.next().as_deref(), arguments.next()) {
            (None, _) => Bot::Cautious2,
            (Some("--yardstick"), Some(bot)) => bot.parse().map_err(anyhow::Error::msg)?,
            _ => anyhow::bail!(usage),
        };
        return print_batch(Path::new(&folder), yardstick);
    }
    let mut arguments = first.into_iter().chain(arguments);
    let record_path = PathBuf::from(
        arguments
            .next()
            .context("usage: goblinsbowl-assess <record.jsonl> [<calls.jsonl>]")?,
    );
    let calls_path = arguments
        .next()
        .map_or_else(|| calls_next_to(&record_path), PathBuf::from);
    let record = MatchRecord::from_json_lines(
        &std::fs::read_to_string(&record_path)
            .with_context(|| format!("cannot read {}", record_path.display()))?,
    )?;
    let calls = read_calls(&calls_path)?;
    let rows = activation_rows(&record)?;
    let finished = record.replay()?;
    let score = &finished.state().score;
    println!(
        "{} (seed {}): home {:?} {} - {} away {:?}, {} tool calls",
        record_path.display(),
        record.setup.seed,
        record.setup.seats[0],
        score.first().copied().unwrap_or_default(),
        score.get(1).copied().unwrap_or_default(),
        record.setup.seats[1],
        calls.len()
    );
    let estimates = estimates_by_entry(&calls, &record);
    for (name, team) in [("Home", TeamId::HOME), ("Away", TeamId::AWAY)] {
        println!("{name}");
        for line in summarise(&rows, team).report() {
            println!("  {line}");
        }
        if let Some(calibration) =
            calibration(&rows, &estimates, team).filter(|calibration| calibration.estimated > 0)
        {
            println!(
                "  Calibration: {} estimates ({} missing), Brier {:.3} against {:.3} for the true odds, mean squared error {:.3}",
                calibration.estimated,
                calibration.missing,
                calibration.brier,
                calibration.brier_of_true_odds,
                calibration.mean_squared_error
            );
        }
        println!(
            "  Refused calls: {}, turns lost to the clock: {}",
            refused_calls(&calls, team),
            turns_lost_to_the_clock(&record, team)
        );
    }
    Ok(())
}
