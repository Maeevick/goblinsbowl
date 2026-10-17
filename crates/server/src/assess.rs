//! What the call log adds to a replayed record: the agents' own estimates, matched to the
//! activations they described, and the refused calls.

use std::collections::HashMap;

use goblins_rules::assessment::{ActivationRow, RiskSummary, summarise};
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

/// One match of a batch, reduced to what the batch report needs.
#[derive(Clone, Debug, PartialEq)]
pub struct BatchMatch {
    /// Model that coached the agent seat.
    pub model: String,
    /// The agent's team.
    pub agent: TeamId,
    /// Every activation of the match, both teams.
    pub rows: Vec<ActivationRow>,
    /// Goals per team, home then away.
    pub goals: [u8; 2],
    /// Turns each team played, home then away.
    pub turns: [u32; 2],
    /// Turns the agent lost to the clock.
    pub turns_lost_to_the_clock: u32,
    /// Calls refused to the agent.
    pub refused_calls: u32,
    /// Whether the match reached the final whistle.
    pub finished: bool,
    /// Coach runs: 1, or more when the coach was resumed.
    pub coach_runs: u32,
    /// Cost the coach runs reported, in USD.
    pub cost_usd: f64,
    /// Wall time of the match, in seconds.
    pub seconds: u64,
}

/// What a batch says about one model, its opponent bot as the yardstick.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelVerdict {
    /// The model.
    pub model: String,
    /// Matches played.
    pub matches: u32,
    /// Matches that did not reach the final whistle.
    pub unfinished: u32,
    /// The agent's wins, draws and losses.
    pub results: [u32; 3],
    /// Goals scored and conceded by the agent.
    pub goals: [u32; 2],
    /// The agent's risk taking, pooled over the matches.
    pub agent: RiskSummary,
    /// The bot's risk taking in the same matches.
    pub bot: RiskSummary,
    /// Turns the agent played.
    pub agent_turns: u32,
    /// Turns the agent lost to the clock.
    pub turns_lost_to_the_clock: u32,
    /// Calls refused to the agent.
    pub refused_calls: u32,
    /// Times a coach had to be resumed.
    pub resumed: u32,
    /// Total cost reported by the coach runs, in USD.
    pub cost_usd: f64,
    /// Mean wall time of a match, in minutes.
    pub mean_minutes: f64,
}

/// One verdict per model, in the order the models first appear in `matches`.
pub fn model_verdicts(matches: &[BatchMatch]) -> Vec<ModelVerdict> {
    let mut models: Vec<&str> = Vec::new();
    for played in matches {
        if !models.contains(&played.model.as_str()) {
            models.push(&played.model);
        }
    }
    models
        .into_iter()
        .map(|model| {
            let own: Vec<&BatchMatch> = matches
                .iter()
                .filter(|played| played.model == model)
                .collect();
            verdict_of(model, &own)
        })
        .collect()
}

fn count(items: usize) -> u32 {
    u32::try_from(items).unwrap_or(u32::MAX)
}

fn verdict_of(model: &str, own: &[&BatchMatch]) -> ModelVerdict {
    let mut pooled = Vec::new();
    let mut results = [0_u32; 3];
    let mut goals = [0_u32; 2];
    for (index, played) in own.iter().enumerate() {
        let offset = count(index).saturating_mul(1_000);
        pooled.extend(played.rows.iter().map(|row| ActivationRow {
            team: if row.team == played.agent {
                TeamId::HOME
            } else {
                TeamId::AWAY
            },
            turn: row.turn.saturating_add(offset),
            ..row.clone()
        }));
        let side = played.agent.index().min(1);
        let scored = u32::from(played.goals[side]);
        let conceded = u32::from(played.goals[1 - side]);
        goals[0] += scored;
        goals[1] += conceded;
        let outcome = match scored.cmp(&conceded) {
            std::cmp::Ordering::Greater => 0,
            std::cmp::Ordering::Equal => 1,
            std::cmp::Ordering::Less => 2,
        };
        results[outcome] += 1;
    }
    let seconds: u64 = own.iter().map(|played| played.seconds).sum();
    ModelVerdict {
        model: model.to_string(),
        matches: count(own.len()),
        unfinished: count(own.iter().filter(|played| !played.finished).count()),
        results,
        goals,
        agent: summarise(&pooled, TeamId::HOME),
        bot: summarise(&pooled, TeamId::AWAY),
        agent_turns: own
            .iter()
            .map(|played| played.turns[played.agent.index().min(1)])
            .sum(),
        turns_lost_to_the_clock: own
            .iter()
            .map(|played| played.turns_lost_to_the_clock)
            .sum(),
        refused_calls: own.iter().map(|played| played.refused_calls).sum(),
        resumed: own
            .iter()
            .map(|played| played.coach_runs.saturating_sub(1))
            .sum(),
        cost_usd: own.iter().map(|played| played.cost_usd).sum(),
        mean_minutes: if own.is_empty() {
            0.0
        } else {
            seconds as f64 / 60.0 / f64::from(count(own.len()))
        },
    }
}

fn shown(value: Option<f64>, unit: &str) -> String {
    value.map_or("n/a".to_string(), |value| format!("{value:.1}{unit}"))
}

fn per_match(total: u32, matches: u32) -> String {
    if matches == 0 {
        "n/a".to_string()
    } else {
        format!("{:.1}", f64::from(total) / f64::from(matches))
    }
}

fn mark(check: Option<bool>) -> &'static str {
    match check {
        Some(true) => "pass",
        Some(false) => "fail",
        None => "n/a",
    }
}

fn risk_cells(summary: &RiskSummary, matches: u32) -> String {
    format!(
        "{} | {} | {} | {} | {} | {} | {}",
        summary
            .safe_first_index
            .map_or("n/a".to_string(), |index| format!("{index:.2}")),
        per_match(summary.failures, matches),
        shown(summary.mean_exposure, ""),
        shown(summary.mean_gap_points, " pts"),
        summary.gaps_over_five_points,
        shown(summary.mean_chance_taken, " %"),
        shown(summary.mean_chance_after_failure, " %"),
    )
}

/// The verdicts as Markdown: a table of measures, then the protocol's criteria for each model.
pub fn batch_report(verdicts: &[ModelVerdict]) -> Vec<String> {
    let mut lines = vec![
        "| Model | Matches | W-D-L | Goals | Safe-first | Failures per match | Ready at failure | Mean gap | Risky gaps over 5 pts | Chance taken | After a failure | Turns lost to the clock | Refused calls | Resumed | Unfinished | Cost | Minutes per match |".to_string(),
        "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|".to_string(),
    ];
    for verdict in verdicts {
        lines.push(format!(
            "| {} | {} | {}-{}-{} | {}-{} | {} | {} | {} | {} | {} | ${:.2} | {:.1} |",
            verdict.model,
            verdict.matches,
            verdict.results[0],
            verdict.results[1],
            verdict.results[2],
            verdict.goals[0],
            verdict.goals[1],
            risk_cells(&verdict.agent, verdict.matches),
            verdict.turns_lost_to_the_clock,
            verdict.refused_calls,
            verdict.resumed,
            verdict.unfinished,
            verdict.cost_usd,
            verdict.mean_minutes
        ));
    }
    lines.push(String::new());
    lines.push("The opponent bot in the same matches:".to_string());
    lines.push(String::new());
    lines.push("| Against | Safe-first | Failures per match | Ready at failure | Mean gap | Risky gaps over 5 pts | Chance taken | After a failure |".to_string());
    lines.push("|---|---|---|---|---|---|---|---|".to_string());
    for verdict in verdicts {
        lines.push(format!(
            "| {} | {} |",
            verdict.model,
            risk_cells(&verdict.bot, verdict.matches)
        ));
    }
    lines.push(String::new());
    lines.push("Criteria fixed before the batch (docs/assessment.md):".to_string());
    lines.push(String::new());
    lines.push("| Model | Safe-first at least 0.90 | Mean gap under 5 pts | Under 10 % of risky orders 5 pts below the best | No tilt (within 5 pts after a failure) | Under one refused call per two turns | Goal difference not negative |".to_string());
    lines.push("|---|---|---|---|---|---|---|".to_string());
    for verdict in verdicts {
        let agent = &verdict.agent;
        lines.push(format!(
            "| {} | {} | {} | {} | {} | {} | {} |",
            verdict.model,
            mark(agent.safe_first_index.map(|index| index >= 0.9)),
            mark(agent.mean_gap_points.map(|gap| gap < 5.0)),
            mark((agent.risky > 0).then(|| {
                f64::from(agent.gaps_over_five_points) < 0.1 * f64::from(agent.risky)
            })),
            mark(
                agent
                    .mean_chance_after_failure
                    .zip(agent.mean_chance_taken)
                    .map(|(after, overall)| (after - overall).abs() <= 5.0)
            ),
            mark((verdict.agent_turns > 0).then(|| {
                f64::from(verdict.refused_calls) < 0.5 * f64::from(verdict.agent_turns)
            })),
            mark(Some(verdict.goals[0] >= verdict.goals[1])),
        ));
    }
    lines
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
            goblin: "A1".parse().unwrap(),
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

    fn batch_match(
        model: &str,
        agent: TeamId,
        goals: [u8; 2],
        rows: Vec<ActivationRow>,
    ) -> BatchMatch {
        BatchMatch {
            model: model.to_string(),
            agent,
            rows,
            goals,
            turns: [20, 20],
            turns_lost_to_the_clock: 0,
            refused_calls: 0,
            finished: true,
            coach_runs: 1,
            cost_usd: 1.5,
            seconds: 900,
        }
    }

    fn turn_row(team: TeamId, goblin: &str, turn: u32, success: Probability) -> ActivationRow {
        ActivationRow {
            team,
            goblin: goblin.parse().unwrap(),
            turn,
            ..row(0, success, false)
        }
    }

    #[test]
    fn given_an_away_agent_winning_when_judging_the_batch_then_a_win_for_its_model() {
        let matches = [batch_match("haiku", TeamId::AWAY, [0, 1], Vec::new())];
        assert_eq!(model_verdicts(&matches)[0].results, [1, 0, 0]);
    }

    #[test]
    fn given_two_models_when_judging_the_batch_then_one_verdict_each_in_order_of_appearance() {
        let matches = [
            batch_match("sonnet", TeamId::HOME, [0, 0], Vec::new()),
            batch_match("haiku", TeamId::AWAY, [0, 0], Vec::new()),
            batch_match("sonnet", TeamId::AWAY, [0, 0], Vec::new()),
        ];
        let verdicts = model_verdicts(&matches);
        assert_eq!(
            verdicts
                .iter()
                .map(|verdict| (verdict.model.as_str(), verdict.matches))
                .collect::<Vec<_>>(),
            [("sonnet", 2), ("haiku", 1)]
        );
    }

    #[test]
    fn given_the_same_turn_number_in_two_matches_when_pooling_then_their_orders_are_not_mixed() {
        let well_ordered = vec![
            turn_row(TeamId::HOME, "H1", 1, Probability::CERTAIN),
            turn_row(TeamId::HOME, "H2", 1, Probability::new(1, 2)),
        ];
        let badly_ordered = vec![
            turn_row(TeamId::AWAY, "A1", 1, Probability::new(1, 2)),
            turn_row(TeamId::AWAY, "A2", 1, Probability::CERTAIN),
        ];
        let matches = [
            batch_match("haiku", TeamId::HOME, [0, 0], well_ordered),
            batch_match("haiku", TeamId::AWAY, [0, 0], badly_ordered),
        ];
        assert_eq!(
            model_verdicts(&matches)[0].agent.safe_first_index,
            Some(0.5)
        );
    }

    #[test]
    fn given_the_bot_rows_when_judging_the_batch_then_they_feed_the_yardstick_not_the_agent() {
        let rows = vec![
            turn_row(TeamId::HOME, "H1", 1, Probability::CERTAIN),
            turn_row(TeamId::AWAY, "A1", 1, Probability::new(1, 2)),
        ];
        let verdict = &model_verdicts(&[batch_match("haiku", TeamId::HOME, [0, 0], rows)])[0];
        assert_eq!((verdict.agent.activations, verdict.bot.activations), (1, 1));
    }

    #[test]
    fn given_a_verdict_when_reporting_the_batch_then_its_model_has_a_row_in_the_table() {
        let verdicts = model_verdicts(&[batch_match("haiku", TeamId::HOME, [1, 0], Vec::new())]);
        assert!(
            batch_report(&verdicts)
                .iter()
                .any(|line| line.starts_with("| haiku | 1 | 1-0-0 |"))
        );
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
