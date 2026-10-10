//! Records the golden match used by the cross-platform replay test.
//!
//! Run `make golden` after a deliberate rules change, then review and commit both fixtures.

use std::path::PathBuf;

use goblins_rules::dice::SeededDice;
use goblins_rules::host::{MatchHost, MatchSetup, SeatKind};
use goblins_rules::policies::random::random_command;
use goblins_rules::roster::Roster;
use goblins_rules::ruleset::Ruleset;

const MATCH_SEED: u64 = 2026;
const COACH_SEED: u64 = 10;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (mut host, _) = MatchHost::start(MatchSetup {
        ruleset: Ruleset::default(),
        rosters: [Roster::DEFAULT, Roster::DEFAULT],
        seed: MATCH_SEED,
        seats: [SeatKind::Bot, SeatKind::Bot],
        turn_seconds: [60, 60],
        agent_ready_grace_seconds: 15,
    })?;
    let mut coach_dice = SeededDice::from_seed(COACH_SEED);
    while let Some(team) = host.game().state().active_team() {
        let game = host.game();
        let command = random_command(
            game.arena(),
            game.ruleset(),
            game.state(),
            team,
            &mut coach_dice,
        );
        host.submit(team, command)?;
        host.advance_clock(1_500);
    }
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    std::fs::write(
        fixtures.join("golden_match.jsonl"),
        host.record().to_json_lines() + "\n",
    )?;
    std::fs::write(
        fixtures.join("golden_final_state.json"),
        serde_json::to_string_pretty(&host.snapshot().state)? + "\n",
    )?;
    println!(
        "golden match recorded: {} commands, final score {:?}",
        host.record().entries.len(),
        host.snapshot().state.score
    );
    Ok(())
}
