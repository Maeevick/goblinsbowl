//! Integrated check: the second-generation bots, the opponents and the yardstick of the assessment
//! benchmark, still play its frozen ruleset command for command as when it was frozen.

use std::error::Error;
use std::path::PathBuf;

use goblins_rules::bot_match::play_bot_match;
use goblins_rules::host::MatchRecord;
use goblins_rules::policies::Bot;
use goblins_rules::ruleset::Ruleset;

const CHANGED: &str = "no longer plays the frozen benchmark as recorded: the benchmark's opponents changed, so run a bridging batch (docs/assessment.md) before `make benchmark-matches`";

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn plays_its_mirror_as_recorded(bot: Bot) -> Result<bool, Box<dyn Error>> {
    let frozen = std::fs::read_to_string(crate_dir().join("../../assessment/rules-0.10.toml"))?;
    let ruleset = Ruleset::from_toml_str(&frozen)?;
    let (played, _) = play_bot_match(&ruleset, [ruleset.default_roster; 2], 1, [bot, bot])?;
    let fixture = crate_dir().join(format!("tests/fixtures/benchmark_{bot}_seed1.jsonl"));
    let recorded = MatchRecord::from_json_lines(&std::fs::read_to_string(fixture)?)?;
    Ok(played.entries == recorded.entries)
}

#[test]
fn given_the_frozen_benchmark_when_cautious_2_plays_its_mirror_on_seed_1_then_every_command_is_as_recorded()
-> Result<(), Box<dyn Error>> {
    assert!(
        plays_its_mirror_as_recorded(Bot::Cautious2)?,
        "cautious-2 {CHANGED}"
    );
    Ok(())
}

#[test]
fn given_the_frozen_benchmark_when_balanced_2_plays_its_mirror_on_seed_1_then_every_command_is_as_recorded()
-> Result<(), Box<dyn Error>> {
    assert!(
        plays_its_mirror_as_recorded(Bot::Balanced2)?,
        "balanced-2 {CHANGED}"
    );
    Ok(())
}

#[test]
fn given_the_frozen_benchmark_when_bold_2_plays_its_mirror_on_seed_1_then_every_command_is_as_recorded()
-> Result<(), Box<dyn Error>> {
    assert!(
        plays_its_mirror_as_recorded(Bot::Bold2)?,
        "bold-2 {CHANGED}"
    );
    Ok(())
}
