//! How a coach handled risk in a recorded match: every activation replayed with its exact odds and
//! the best odds it could have had for the same purpose.

use crate::arena::Arena;
use crate::command::{Activation, Command};
use crate::dice::SeededDice;
use crate::event::{GameEvent, TurnEndReason};
use crate::game::Game;
use crate::host::{MatchRecord, ReplayError};
use crate::odds::Probability;
use crate::paths::best_chance_for;
use crate::preview::activation_odds;
use crate::roster::GoblinId;
use crate::ruleset::Ruleset;
use crate::state::GameState;
use crate::team::TeamId;

/// One activation of a record, seen just before it was resolved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActivationRow {
    /// Index of its entry in the record.
    pub entry: usize,
    /// The team acting.
    pub team: TeamId,
    /// The goblin ordered.
    pub goblin: GoblinId,
    /// Which turn of that team it belongs to, counted from 1 across both halves.
    pub turn: u32,
    /// Exact chance it ends without a knockdown or a fumble.
    pub success: Probability,
    /// Best chance of an order with the same purpose: same goblin, same action and target, or
    /// same destination, along any legal path.
    pub best_success: Probability,
    /// The team's goals minus the opponent's, before it.
    pub score_difference: i16,
    /// Whether it ended the turn with a knockdown or a fumble.
    pub failed: bool,
    /// The team's other goblins that could still act when it was played.
    pub others_ready: u8,
    /// Whether the team's previous turn ended with a knockdown or a fumble.
    pub after_failure: bool,
    /// Health the team's goblins lost through it.
    pub own_health_lost: u32,
    /// Entries of the team's risky orders earlier in this turn, by other goblins, before which this
    /// order was already certain: it could have been played first.
    pub already_certain_before: Vec<usize>,
}

/// Risk taken by one team over a match.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RiskSummary {
    /// Activations played.
    pub activations: u32,
    /// Activations with a chance below certain.
    pub risky: u32,
    /// Share of (certain, risky) pairs of two goblins' orders within a turn where the certain one
    /// came first; a goblin's own orders are not paired, and a certain order played after a risky
    /// one only counts against when it was already certain before it, so it could have come first.
    pub safe_first_index: Option<f64>,
    /// Activations that ended the turn with a failed roll.
    pub failures: u32,
    /// Failures where an order with the same purpose had a better chance.
    pub avoidable_failures: u32,
    /// Mean number of goblins still ready when a failure ended the turn.
    pub mean_exposure: Option<f64>,
    /// Mean gap, in percentage points, between the best chance for the same purpose and the chance taken.
    pub mean_gap_points: Option<f64>,
    /// Risky activations whose gap exceeded 5 points.
    pub gaps_over_five_points: u32,
    /// Mean chance of the risky activations, in percent.
    pub mean_chance_taken: Option<f64>,
    /// Mean chance of the risky activations in turns right after a failed turn, in percent.
    pub mean_chance_after_failure: Option<f64>,
    /// Mean chance of the risky activations while behind, level and ahead, in percent.
    pub mean_chance_by_score: [Option<f64>; 3],
    /// Health lost through the team's own activations.
    pub own_health_lost: u32,
}

/// The exact chance of `activation` for `team`, and the best chance of an order with the same purpose.
pub fn describe_activation(
    arena: &Arena,
    ruleset: &Ruleset,
    state: &GameState,
    team: TeamId,
    activation: &Activation,
) -> Option<(Probability, Probability)> {
    let chance = |candidate: &Activation| {
        activation_odds(arena, ruleset, state, team, candidate)
            .ok()
            .map(|odds| odds.overall_success)
    };
    let success = chance(activation)?;
    let start = state.goblin(activation.goblin)?.square()?;
    let destination = activation.path.last().copied().unwrap_or(start);
    let best = best_chance_for(
        arena,
        ruleset,
        state,
        team,
        activation.goblin,
        destination,
        activation.action,
    )
    .map_or(success, |best| best.max(success));
    Some((success, best))
}

/// A risky order of the turn being replayed, with the board it was played on.
struct EarlierRisky {
    entry: usize,
    team: TeamId,
    turn: u32,
    goblin: GoblinId,
    board: GameState,
}

fn ends_in_failure(events: &[GameEvent], team: TeamId) -> bool {
    events.iter().any(|event| {
        matches!(
            event,
            GameEvent::TurnEnded {
                team: ended,
                reason: TurnEndReason::Knockdown | TurnEndReason::Fumble,
            } if *ended == team
        )
    })
}

#[derive(Default)]
struct TurnTracker {
    turns: [u32; 2],
    failed_turn_in_progress: [bool; 2],
    previous_turn_failed: [bool; 2],
}

impl TurnTracker {
    fn follow(&mut self, events: &[GameEvent]) {
        for event in events {
            match event {
                GameEvent::TurnStarted { team, .. } => {
                    let index = team.index().min(1);
                    self.turns[index] += 1;
                    self.previous_turn_failed[index] = self.failed_turn_in_progress[index];
                    self.failed_turn_in_progress[index] = false;
                }
                GameEvent::TurnEnded { team, reason } => {
                    self.failed_turn_in_progress[team.index().min(1)] =
                        matches!(reason, TurnEndReason::Knockdown | TurnEndReason::Fumble);
                }
                _ => {}
            }
        }
    }
}

/// Every activation of `record`, replayed with its odds.
///
/// # Errors
/// [`ReplayError`] when the record does not replay on this engine.
pub fn activation_rows(record: &MatchRecord) -> Result<Vec<ActivationRow>, ReplayError> {
    let (mut game, opening) = Game::start(
        record.setup.ruleset.clone(),
        record.setup.rosters,
        SeededDice::from_seed(record.setup.seed),
    )?;
    let mut tracker = TurnTracker::default();
    tracker.follow(&opening);
    let mut rows = Vec::new();
    let mut earlier_risky: Vec<EarlierRisky> = Vec::new();
    for (index, entry) in record.entries.iter().enumerate() {
        let team = entry.team;
        let side = team.index().min(1);
        earlier_risky.retain(|risky| risky.turn == tracker.turns[risky.team.index().min(1)]);
        let before = match &entry.command {
            Command::Act(activation) => {
                let state = game.state();
                let described =
                    describe_activation(game.arena(), game.ruleset(), state, team, activation);
                let others_ready = state
                    .goblins_of(team)
                    .filter(|goblin| goblin.can_act() && goblin.id != activation.goblin)
                    .count();
                let own = state.score.get(side).copied().unwrap_or_default();
                let theirs = state.score.get(1 - side).copied().unwrap_or_default();
                let turn = tracker.turns[side];
                described.map(|described| {
                    let destination = activation.path.last().copied().or_else(|| {
                        state
                            .goblin(activation.goblin)
                            .and_then(|goblin| goblin.square())
                    });
                    let already_certain_before = if described.0 == Probability::CERTAIN {
                        earlier_risky
                            .iter()
                            .filter(|risky| {
                                risky.team == team
                                    && risky.turn == turn
                                    && risky.goblin != activation.goblin
                            })
                            .filter(|risky| {
                                destination.is_some_and(|destination| {
                                    best_chance_for(
                                        game.arena(),
                                        game.ruleset(),
                                        &risky.board,
                                        team,
                                        activation.goblin,
                                        destination,
                                        activation.action,
                                    ) == Some(Probability::CERTAIN)
                                })
                            })
                            .map(|risky| risky.entry)
                            .collect()
                    } else {
                        earlier_risky.push(EarlierRisky {
                            entry: index,
                            team,
                            turn,
                            goblin: activation.goblin,
                            board: state.clone(),
                        });
                        Vec::new()
                    };
                    (
                        described,
                        u8::try_from(others_ready).unwrap_or(u8::MAX),
                        i16::from(own) - i16::from(theirs),
                        activation.goblin,
                        already_certain_before,
                    )
                })
            }
            Command::EndTurn | Command::TimeUp => None,
        };
        let events = game
            .apply(team, entry.command.clone())
            .map_err(|violation| ReplayError::Diverged { index, violation })?;
        if let Some((
            (success, best_success),
            others_ready,
            score_difference,
            ordered,
            already_certain_before,
        )) = before
        {
            rows.push(ActivationRow {
                entry: index,
                team,
                goblin: ordered,
                turn: tracker.turns[side],
                success,
                best_success,
                score_difference,
                failed: ends_in_failure(&events, team),
                others_ready,
                after_failure: tracker.previous_turn_failed[side],
                own_health_lost: events
                    .iter()
                    .map(|event| match event {
                        GameEvent::HealthLost { goblin, amount, .. } if goblin.team == team => {
                            u32::from(*amount)
                        }
                        _ => 0,
                    })
                    .sum(),
                already_certain_before,
            });
        }
        tracker.follow(&events);
    }
    Ok(rows)
}

fn count(items: usize) -> u32 {
    u32::try_from(items).unwrap_or(u32::MAX)
}

fn mean(values: &[f64]) -> Option<f64> {
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / f64::from(count(values.len())))
}

/// What `team`'s activations say about its handling of risk.
pub fn summarise(rows: &[ActivationRow], team: TeamId) -> RiskSummary {
    let own: Vec<&ActivationRow> = rows.iter().filter(|row| row.team == team).collect();
    let risky: Vec<&ActivationRow> = own
        .iter()
        .copied()
        .filter(|row| row.success < Probability::CERTAIN)
        .collect();
    let failures: Vec<&ActivationRow> = own.iter().copied().filter(|row| row.failed).collect();
    let gap = |row: &ActivationRow| row.best_success.as_percentage() - row.success.as_percentage();
    let chances = |keep: &dyn Fn(&ActivationRow) -> bool| -> Vec<f64> {
        risky
            .iter()
            .filter(|row| keep(row))
            .map(|row| row.success.as_percentage())
            .collect()
    };
    let (mut well_ordered, mut mixed_pairs) = (0_u32, 0_u32);
    let mut turns: Vec<u32> = own.iter().map(|row| row.turn).collect();
    turns.dedup();
    for turn in turns {
        let orders: Vec<&ActivationRow> =
            own.iter().copied().filter(|row| row.turn == turn).collect();
        for (position, first) in orders.iter().enumerate() {
            for later in &orders[position + 1..] {
                let certain = first.success == Probability::CERTAIN;
                let later_certain = later.success == Probability::CERTAIN;
                if certain == later_certain || first.goblin == later.goblin {
                    continue;
                }
                if certain {
                    mixed_pairs += 1;
                    well_ordered += 1;
                } else if later.already_certain_before.contains(&first.entry) {
                    mixed_pairs += 1;
                }
            }
        }
    }
    RiskSummary {
        activations: count(own.len()),
        risky: count(risky.len()),
        safe_first_index: (mixed_pairs > 0)
            .then(|| f64::from(well_ordered) / f64::from(mixed_pairs)),
        failures: count(failures.len()),
        avoidable_failures: count(
            failures
                .iter()
                .filter(|row| row.best_success > row.success)
                .count(),
        ),
        mean_exposure: mean(
            &failures
                .iter()
                .map(|row| f64::from(row.others_ready))
                .collect::<Vec<_>>(),
        ),
        mean_gap_points: mean(&risky.iter().map(|row| gap(row)).collect::<Vec<_>>()),
        gaps_over_five_points: count(risky.iter().filter(|row| gap(row) > 5.0).count()),
        mean_chance_taken: mean(&chances(&|_| true)),
        mean_chance_after_failure: mean(&chances(&|row| row.after_failure)),
        mean_chance_by_score: [
            mean(&chances(&|row| row.score_difference < 0)),
            mean(&chances(&|row| row.score_difference == 0)),
            mean(&chances(&|row| row.score_difference > 0)),
        ],
        own_health_lost: own.iter().map(|row| row.own_health_lost).sum(),
    }
}

impl RiskSummary {
    /// The summary as lines of text.
    pub fn report(&self) -> Vec<String> {
        let shown = |value: Option<f64>, unit: &str| {
            value.map_or("none".to_string(), |value| format!("{value:.1}{unit}"))
        };
        vec![
            format!(
                "Safe-first index: {} ({} activations, {} risky)",
                self.safe_first_index
                    .map_or("none".to_string(), |index| format!("{index:.2}")),
                self.activations,
                self.risky
            ),
            format!(
                "Failures: {} ({} avoidable), goblins still ready per failure: {}",
                self.failures,
                self.avoidable_failures,
                shown(self.mean_exposure, "")
            ),
            format!(
                "Avoidable risk: mean gap {} points, {} risky activations more than 5 points below the best",
                shown(self.mean_gap_points, ""),
                self.gaps_over_five_points
            ),
            format!(
                "Chance taken on risky activations: {} on average, {} right after a failed turn",
                shown(self.mean_chance_taken, " %"),
                shown(self.mean_chance_after_failure, " %")
            ),
            format!(
                "By score: behind {}, level {}, ahead {}",
                shown(self.mean_chance_by_score[0], " %"),
                shown(self.mean_chance_by_score[1], " %"),
                shown(self.mean_chance_by_score[2], " %")
            ),
            format!(
                "Health lost through own activations: {}",
                self.own_health_lost
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::characteristics::Archetype::{Bruiser, Sprinter};
    use crate::command::Action;
    use crate::game::scenario::Scenario;
    use crate::roster::GoblinId;
    use crate::square::Square;

    fn id(text: &str) -> GoblinId {
        text.parse().unwrap()
    }

    fn walk(goblin: &str, squares: &[(u8, u8)], action: Option<Action>) -> Activation {
        Activation {
            goblin: id(goblin),
            path: squares.iter().map(|(x, y)| Square::new(*x, *y)).collect(),
            action,
        }
    }

    fn described(
        scenario: Scenario,
        activation: &Activation,
    ) -> Option<(Probability, Probability)> {
        let game = scenario.game([]);
        describe_activation(
            game.arena(),
            game.ruleset(),
            game.state(),
            TeamId::HOME,
            activation,
        )
    }

    #[test]
    fn given_a_free_walk_when_described_then_certain_and_nothing_better() {
        let scenario = Scenario::new()
            .goblin("H5", Sprinter, 5, 7)
            .ball_loose_at(1, 10);
        assert_eq!(
            described(scenario, &walk("H5", &[(6, 7), (7, 7)], None)),
            Some((Probability::CERTAIN, Probability::CERTAIN))
        );
    }

    #[test]
    fn given_an_assault_reached_through_a_dodge_when_described_then_a_safer_way_to_the_same_target_is_found()
     {
        let scenario = Scenario::new()
            .goblin("H5", Sprinter, 5, 7)
            .goblin("A1", Bruiser, 8, 7)
            .goblin("A2", Bruiser, 7, 8)
            .ball_loose_at(1, 10);
        let assault = walk(
            "H5",
            &[(6, 7), (7, 7), (7, 6), (8, 6)],
            Some(Action::assault(id("A1"))),
        );
        assert_eq!(
            described(scenario, &assault),
            Some((Probability::new(5, 16), Probability::new(1, 2)))
        );
    }

    fn row(turn: u32, success: Probability) -> ActivationRow {
        ActivationRow {
            entry: 0,
            team: TeamId::HOME,
            goblin: id("H1"),
            turn,
            success,
            best_success: success,
            score_difference: 0,
            failed: false,
            others_ready: 0,
            after_failure: false,
            own_health_lost: 0,
            already_certain_before: Vec::new(),
        }
    }

    fn order_of(goblin: &str, turn: u32, success: Probability) -> ActivationRow {
        ActivationRow {
            goblin: id(goblin),
            ..row(turn, success)
        }
    }

    #[test]
    fn given_a_gamble_before_an_order_that_was_already_certain_when_summarising_then_the_safe_first_index_is_zero()
     {
        let rows = [
            order_of("H1", 1, Probability::new(1, 2)),
            ActivationRow {
                entry: 1,
                already_certain_before: vec![0],
                ..order_of("H2", 1, Probability::CERTAIN)
            },
        ];
        assert_eq!(summarise(&rows, TeamId::HOME).safe_first_index, Some(0.0));
    }

    #[test]
    fn given_a_certain_order_made_possible_by_another_goblin_s_gamble_when_summarising_then_that_pair_is_left_out()
     {
        let rows = [
            order_of("H1", 1, Probability::new(1, 2)),
            ActivationRow {
                entry: 1,
                ..order_of("H2", 1, Probability::CERTAIN)
            },
        ];
        assert_eq!(summarise(&rows, TeamId::HOME).safe_first_index, None);
    }

    #[test]
    fn given_a_recorded_match_when_replayed_then_every_order_said_already_certain_follows_another_goblin_s_risky_order_of_its_turn()
     {
        let record =
            MatchRecord::from_json_lines(include_str!("../tests/fixtures/golden_match.jsonl"))
                .unwrap();
        let rows = activation_rows(&record).unwrap();
        let earlier = |entry: usize| rows.iter().find(|row| row.entry == entry);
        let pairs: Vec<(&ActivationRow, &ActivationRow)> = rows
            .iter()
            .flat_map(|row| {
                row.already_certain_before
                    .iter()
                    .filter_map(move |entry| earlier(*entry).map(|risky| (risky, row)))
            })
            .collect();
        assert!(
            !pairs.is_empty()
                && pairs.iter().all(|(risky, certain)| {
                    risky.entry < certain.entry
                        && risky.team == certain.team
                        && risky.turn == certain.turn
                        && risky.goblin != certain.goblin
                        && risky.success < Probability::CERTAIN
                        && certain.success == Probability::CERTAIN
                })
        );
    }

    #[test]
    fn given_certain_orders_first_in_every_turn_when_summarising_then_the_safe_first_index_is_one()
    {
        let rows = [
            order_of("H1", 1, Probability::CERTAIN),
            order_of("H2", 1, Probability::new(1, 2)),
            order_of("H1", 2, Probability::CERTAIN),
            order_of("H2", 2, Probability::new(3, 4)),
        ];
        assert_eq!(summarise(&rows, TeamId::HOME).safe_first_index, Some(1.0));
    }

    #[test]
    fn given_a_goblin_walking_on_after_its_own_risky_assault_when_summarising_then_that_pair_is_left_out()
     {
        let rows = [
            order_of("H2", 1, Probability::CERTAIN),
            order_of("H1", 1, Probability::new(1, 2)),
            order_of("H1", 1, Probability::CERTAIN),
        ];
        assert_eq!(summarise(&rows, TeamId::HOME).safe_first_index, Some(1.0));
    }

    #[test]
    fn given_a_turn_where_only_one_goblin_mixed_certain_and_risky_orders_when_summarising_then_no_index()
     {
        let rows = [
            order_of("H1", 1, Probability::new(1, 2)),
            order_of("H1", 1, Probability::CERTAIN),
        ];
        assert_eq!(summarise(&rows, TeamId::HOME).safe_first_index, None);
    }

    #[test]
    fn given_a_risky_order_with_a_better_way_when_summarising_then_its_gap_is_counted_in_points() {
        let mut risky = row(1, Probability::new(1, 4));
        risky.best_success = Probability::new(1, 2);
        let summary = summarise(&[risky], TeamId::HOME);
        assert_eq!(
            (summary.mean_gap_points, summary.gaps_over_five_points),
            (Some(25.0), 1)
        );
    }

    #[test]
    fn given_a_failure_with_two_goblins_still_ready_when_summarising_then_the_exposure_is_two() {
        let mut failed = row(1, Probability::new(1, 2));
        failed.failed = true;
        failed.others_ready = 2;
        assert_eq!(summarise(&[failed], TeamId::HOME).mean_exposure, Some(2.0));
    }

    #[test]
    fn given_bolder_chances_after_a_failed_turn_when_summarising_then_both_means_are_reported() {
        let mut bold = row(2, Probability::new(1, 4));
        bold.after_failure = true;
        let summary = summarise(&[row(1, Probability::new(3, 4)), bold], TeamId::HOME);
        assert_eq!(
            (summary.mean_chance_taken, summary.mean_chance_after_failure),
            (Some(50.0), Some(25.0))
        );
    }

    #[test]
    fn given_another_team_rows_when_summarising_then_they_are_left_out() {
        let mut away = row(1, Probability::new(1, 2));
        away.team = TeamId::AWAY;
        assert_eq!(summarise(&[away], TeamId::HOME).activations, 0);
    }

    #[test]
    fn given_a_summary_when_reporting_then_the_safe_first_line_shows_the_index() {
        let summary = RiskSummary {
            activations: 4,
            risky: 2,
            safe_first_index: Some(0.75),
            ..RiskSummary::default()
        };
        assert!(
            summary
                .report()
                .contains(&"Safe-first index: 0.75 (4 activations, 2 risky)".to_string())
        );
    }

    fn one_row_per_activation(record_text: &str) {
        let record = MatchRecord::from_json_lines(record_text).unwrap();
        let activations = record
            .entries
            .iter()
            .filter(|entry| matches!(entry.command, Command::Act(_)))
            .count();
        assert_eq!(activation_rows(&record).unwrap().len(), activations);
    }

    #[test]
    fn given_the_golden_record_when_replayed_into_rows_then_one_row_per_activation() {
        one_row_per_activation(include_str!("../tests/fixtures/golden_match.jsonl"));
    }

    #[test]
    fn given_the_golden_record_of_rules_0_1_when_replayed_into_rows_then_one_row_per_activation() {
        one_row_per_activation(include_str!(
            "../tests/fixtures/golden_match_rules_0_1.jsonl"
        ));
    }
}
