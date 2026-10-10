//! What the call log adds to a replayed record: the agents' own estimates, matched to the
//! activations they described, and the refused calls.

use std::collections::HashMap;

use goblins_rules::assessment::ActivationRow;
use goblins_rules::command::Command;
use goblins_rules::host::MatchRecord;
use goblins_rules::team::TeamId;
use serde_json::Value;

use crate::call_log::CallEntry;

/// How well a coach's estimates matched what happened and the true odds.
#[derive(Clone, Debug, PartialEq)]
pub struct Calibration {
    /// Activations sent with an estimate.
    pub estimated: u32,
    /// Activations sent without one.
    pub missing: u32,
    /// Mean squared distance between the estimates and the outcomes (1 success, 0 failure).
    pub brier: f64,
    /// The same for the true odds: the part due to the dice alone.
    pub brier_of_true_odds: f64,
    /// Mean squared distance between the estimates and the true odds.
    pub mean_squared_error: f64,
}

/// The team a call log seat name stands for.
pub fn team_of_seat(seat: &str) -> Option<TeamId> {
    match seat {
        "home" => Some(TeamId::HOME),
        "away" => Some(TeamId::AWAY),
        _ => None,
    }
}

/// Each estimate sent with an applied activation, by the index of that activation in `record`.
pub fn estimates_by_entry(calls: &[CallEntry], record: &MatchRecord) -> HashMap<usize, f64> {
    let mut estimates = HashMap::new();
    for call in calls {
        let dry_run = call
            .arguments
            .get("dry_run")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let (Some(team), Some(version_before)) = (team_of_seat(&call.seat), call.version_before)
        else {
            continue;
        };
        if call.tool != "play" || call.error.is_some() || dry_run {
            continue;
        }
        let sent = call
            .arguments
            .get("activations")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let from = usize::try_from(version_before).unwrap_or(usize::MAX);
        let to = usize::try_from(call.version)
            .unwrap_or(usize::MAX)
            .min(record.entries.len());
        let applied = (from..to).filter(|index| {
            record
                .entries
                .get(*index)
                .is_some_and(|entry| entry.team == team && matches!(entry.command, Command::Act(_)))
        });
        for (index, activation) in applied.zip(sent.iter()) {
            if let Some(estimate) = activation.get("estimate").and_then(Value::as_f64) {
                estimates.insert(index, estimate);
            }
        }
    }
    estimates
}

/// Calibration of `team`'s estimates over its activations, if it played any.
pub fn calibration(
    rows: &[ActivationRow],
    estimates: &HashMap<usize, f64>,
    team: TeamId,
) -> Option<Calibration> {
    let own: Vec<&ActivationRow> = rows.iter().filter(|row| row.team == team).collect();
    if own.is_empty() {
        return None;
    }
    let mut estimated = 0_u32;
    let (mut brier, mut brier_of_true_odds, mut squared_error) = (0.0, 0.0, 0.0);
    for row in &own {
        let Some(estimate) = estimates.get(&row.entry) else {
            continue;
        };
        let estimate = estimate.clamp(0.0, 1.0);
        let outcome = if row.failed { 0.0 } else { 1.0 };
        let true_odds = row.success.as_percentage() / 100.0;
        estimated += 1;
        brier += (estimate - outcome) * (estimate - outcome);
        brier_of_true_odds += (true_odds - outcome) * (true_odds - outcome);
        squared_error += (estimate - true_odds) * (estimate - true_odds);
    }
    let average = |total: f64| {
        if estimated == 0 {
            0.0
        } else {
            total / f64::from(estimated)
        }
    };
    Some(Calibration {
        estimated,
        missing: u32::try_from(own.len())
            .unwrap_or(u32::MAX)
            .saturating_sub(estimated),
        brier: average(brier),
        brier_of_true_odds: average(brier_of_true_odds),
        mean_squared_error: average(squared_error),
    })
}

/// Calls refused to `team`'s seat, whatever the tool.
pub fn refused_calls(calls: &[CallEntry], team: TeamId) -> u32 {
    let refused = calls
        .iter()
        .filter(|call| team_of_seat(&call.seat) == Some(team) && call.error.is_some())
        .count();
    u32::try_from(refused).unwrap_or(u32::MAX)
}

/// Turns `team` lost to the clock.
pub fn turns_lost_to_the_clock(record: &MatchRecord, team: TeamId) -> u32 {
    let lost = record
        .entries
        .iter()
        .filter(|entry| entry.team == team && entry.command == Command::TimeUp)
        .count();
    u32::try_from(lost).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use goblins_rules::command::Activation;
    use goblins_rules::host::{MatchSetup, RecordEntry, SeatKind};
    use goblins_rules::odds::Probability;
    use goblins_rules::roster::Roster;
    use goblins_rules::ruleset::Ruleset;
    use serde_json::json;

    fn act(team: TeamId, goblin: &str) -> RecordEntry {
        RecordEntry {
            team,
            command: Command::Act(Activation {
                goblin: goblin.parse().unwrap(),
                path: Vec::new(),
                action: None,
            }),
            at_ms: 0,
        }
    }

    fn record(entries: Vec<RecordEntry>) -> MatchRecord {
        MatchRecord {
            setup: MatchSetup {
                ruleset: Ruleset::default(),
                rosters: [Roster::DEFAULT, Roster::DEFAULT],
                seed: 1,
                seats: [SeatKind::Bot, SeatKind::Agent],
                turn_seconds: [60, 60],
                agent_ready_grace_seconds: 15,
            },
            entries,
        }
    }

    fn play(version_before: u64, version: u64, arguments: Value) -> CallEntry {
        CallEntry {
            at_ms: 0,
            seat: "away".to_string(),
            tool: "play".to_string(),
            arguments,
            latency_ms: 10,
            error: None,
            version_before: Some(version_before),
            version,
        }
    }

    fn row(entry: usize, success: Probability, failed: bool) -> ActivationRow {
        ActivationRow {
            entry,
            team: TeamId::AWAY,
            turn: 1,
            success,
            best_success: success,
            score_difference: 0,
            failed,
            others_ready: 0,
            after_failure: false,
            own_health_lost: 0,
        }
    }

    #[test]
    fn given_a_play_call_applying_two_activations_when_matching_estimates_then_each_finds_its_entry()
     {
        let record = record(vec![
            act(TeamId::HOME, "H1"),
            act(TeamId::AWAY, "A5"),
            act(TeamId::AWAY, "A6"),
        ]);
        let calls = [play(
            1,
            3,
            json!({"activations": [{"goblin": "A5", "estimate": 0.5}, {"goblin": "A6", "estimate": 0.75}]}),
        )];
        assert_eq!(
            estimates_by_entry(&calls, &record),
            HashMap::from([(1, 0.5), (2, 0.75)])
        );
    }

    #[test]
    fn given_a_dry_run_when_matching_estimates_then_nothing_is_matched() {
        let record = record(vec![act(TeamId::AWAY, "A5")]);
        let calls = [play(
            0,
            1,
            json!({"dry_run": true, "activations": [{"goblin": "A5", "estimate": 0.5}]}),
        )];
        assert_eq!(estimates_by_entry(&calls, &record), HashMap::new());
    }

    #[test]
    fn given_honest_estimates_when_calibrating_then_no_error_against_the_true_odds() {
        let rows = [
            row(0, Probability::new(3, 4), false),
            row(1, Probability::new(1, 2), true),
        ];
        let estimates = HashMap::from([(0, 0.75), (1, 0.5)]);
        let calibration = calibration(&rows, &estimates, TeamId::AWAY).unwrap();
        assert_eq!(
            (
                calibration.mean_squared_error,
                calibration.brier,
                calibration.brier_of_true_odds
            ),
            (0.0, 0.15625, 0.15625)
        );
    }

    #[test]
    fn given_an_activation_without_estimate_when_calibrating_then_it_is_counted_as_missing() {
        let rows = [row(0, Probability::CERTAIN, false)];
        assert_eq!(
            calibration(&rows, &HashMap::new(), TeamId::AWAY)
                .map(|calibration| calibration.missing),
            Some(1)
        );
    }

    #[test]
    fn given_a_refused_call_when_counting_then_it_belongs_to_its_seat() {
        let mut refused = play(0, 0, json!({}));
        refused.error = Some("stale plan".to_string());
        assert_eq!(refused_calls(&[refused], TeamId::AWAY), 1);
    }

    #[test]
    fn given_a_turn_ended_by_the_clock_when_counting_then_it_is_lost_to_the_clock() {
        let record = record(vec![RecordEntry {
            team: TeamId::AWAY,
            command: Command::TimeUp,
            at_ms: 60_000,
        }]);
        assert_eq!(turns_lost_to_the_clock(&record, TeamId::AWAY), 1);
    }
}
