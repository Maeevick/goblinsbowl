//! Integrated check: a match recorded on one machine replays to the same final state on any other,
//! and a match recorded under rules 0.1 still replays after the rules moved on.

use std::error::Error;
use std::path::PathBuf;

use goblins_rules::host::MatchRecord;
use goblins_rules::state::GameState;

fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read_to_string(path).unwrap_or_default()
}

fn replayed_state(record_file: &str) -> Result<GameState, Box<dyn Error>> {
    let record = MatchRecord::from_json_lines(&fixture(record_file))?;
    Ok(record.replay()?.state().clone())
}

fn golden_state(final_state_file: &str) -> Result<GameState, Box<dyn Error>> {
    Ok(serde_json::from_str(&fixture(final_state_file))?)
}

#[test]
fn given_the_golden_record_when_replayed_then_it_reaches_the_golden_final_state()
-> Result<(), Box<dyn Error>> {
    assert_eq!(
        replayed_state("golden_match.jsonl")?,
        golden_state("golden_final_state.json")?
    );
    Ok(())
}

#[test]
fn given_the_golden_record_of_rules_0_1_when_replayed_then_it_reaches_its_golden_final_state()
-> Result<(), Box<dyn Error>> {
    assert_eq!(
        replayed_state("golden_match_rules_0_1.jsonl")?,
        golden_state("golden_final_state_rules_0_1.json")?
    );
    Ok(())
}
