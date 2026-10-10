//! Prints how each coach of a run handled risk: `goblinsbowl-assess <record.jsonl> [<calls.jsonl>]`.
//!
//! The call log defaults to the file saved next to the record (`<record>-calls.jsonl`).

use std::path::{Path, PathBuf};

use anyhow::Context;
use goblins_rules::assessment::{activation_rows, summarise};
use goblins_rules::host::MatchRecord;
use goblins_rules::team::TeamId;
use goblins_server::assess::{
    calibration, estimates_by_entry, refused_calls, turns_lost_to_the_clock,
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

fn main() -> anyhow::Result<()> {
    let mut arguments = std::env::args().skip(1);
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
