//! Integrated check: a match recorded on one machine replays to the same final state on any other.

use std::path::PathBuf;

use goblins_rules::host::MatchRecord;
use goblins_rules::state::GameState;

fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read_to_string(path).unwrap_or_default()
}

#[test]
fn given_the_golden_record_when_replayed_then_it_reaches_the_golden_final_state() {
    let record = MatchRecord::from_json_lines(&fixture("golden_match.jsonl")).unwrap();
    let expected: GameState = serde_json::from_str(&fixture("golden_final_state.json")).unwrap();
    assert_eq!(record.replay().unwrap().state(), &expected);
}
