//! Records how each second-generation bot plays the frozen ruleset of the assessment benchmark, for
//! the test that keeps the benchmark's opponents fixed.
//!
//! Run `make benchmark-matches` only when the benchmark itself changes, after its bridging batch
//! (docs/assessment.md), then review and commit the fixtures.

use std::path::PathBuf;

use goblins_rules::bot_match::play_bot_match;
use goblins_rules::policies::Bot;
use goblins_rules::ruleset::Ruleset;

const SEED: u64 = 1;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ruleset = Ruleset::from_toml_str(&std::fs::read_to_string(
        crate_dir.join("../../assessment/rules-0.10.toml"),
    )?)?;
    for bot in [Bot::Cautious2, Bot::Balanced2, Bot::Bold2] {
        let (record, _) = play_bot_match(&ruleset, [ruleset.default_roster; 2], SEED, [bot, bot])?;
        let path = crate_dir.join(format!("tests/fixtures/benchmark_{bot}_seed{SEED}.jsonl"));
        std::fs::write(&path, record.to_json_lines())?;
        println!(
            "{bot}: {} commands in {}",
            record.entries.len(),
            path.display()
        );
    }
    Ok(())
}
