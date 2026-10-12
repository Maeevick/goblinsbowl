//! How the rules play out over many matches: statistics counted from match events, then summed.

use std::cmp::Ordering;

use crate::event::{GameEvent, TurnEndReason};
use crate::roster::GoblinId;
use crate::team::TeamId;

fn count_for(counts: &mut [u32; 2], team: TeamId) {
    if let Some(count) = counts.get_mut(team.index()) {
        *count += 1;
    }
}

fn add_pair(total: &mut [u32; 2], more: [u32; 2]) {
    total[0] += more[0];
    total[1] += more[1];
}

/// How many times something was tried, and how many of those tries succeeded.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Attempts {
    /// Tries.
    pub tried: u32,
    /// Successful tries.
    pub succeeded: u32,
}

impl Attempts {
    fn record(&mut self, success: bool) {
        self.tried += 1;
        self.succeeded += u32::from(success);
    }

    fn absorb(&mut self, other: Attempts) {
        self.tried += other.tried;
        self.succeeded += other.succeeded;
    }
}

/// What happened in one match, counted from its events. Arrays are indexed by team.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MatchStats {
    /// Goals scored.
    pub goals: [u32; 2],
    /// Goals scored in the first and in the second half, both teams together.
    pub goals_by_half: [u32; 2],
    /// Turns played.
    pub turns: [u32; 2],
    /// Turns ended by a knockdown of the team's own goblin.
    pub knockdowns: [u32; 2],
    /// Turns ended by a fumble or an interception.
    pub fumbles: [u32; 2],
    /// Passes intercepted by the team.
    pub interceptions: [u32; 2],
    /// Stances taken.
    pub stances: [u32; 2],
    /// Goblins knocked out.
    pub knock_outs: [u32; 2],
    /// Assaults made, and won by the attacker.
    pub assaults: Attempts,
    /// Dodges tried, and made.
    pub dodges: Attempts,
    /// Pick-ups tried, and made.
    pub picks: Attempts,
    /// Throws made, and caught by their receiver.
    pub throws: Attempts,
    /// The team that received the opening kick-off.
    pub first_receiver: Option<TeamId>,
}

impl MatchStats {
    /// Statistics of a whole match from all its events, in order.
    pub fn from_events(events: &[GameEvent]) -> Self {
        let mut stats = MatchStats::default();
        let mut half_index = 0;
        let mut awaited_receiver: Option<GoblinId> = None;
        for event in events {
            match *event {
                GameEvent::CoinTossed { receiving_team } => {
                    stats.first_receiver.get_or_insert(receiving_team);
                }
                GameEvent::HalfEnded { half } => half_index = usize::from(half).min(1),
                GameEvent::Scored { team, .. } => {
                    count_for(&mut stats.goals, team);
                    if let Some(count) = stats.goals_by_half.get_mut(half_index) {
                        *count += 1;
                    }
                }
                GameEvent::TurnStarted { team, .. } => count_for(&mut stats.turns, team),
                GameEvent::TurnEnded {
                    team,
                    reason: TurnEndReason::Knockdown,
                } => count_for(&mut stats.knockdowns, team),
                GameEvent::TurnEnded {
                    team,
                    reason: TurnEndReason::Fumble,
                } => count_for(&mut stats.fumbles, team),
                GameEvent::StanceTaken { goblin } => count_for(&mut stats.stances, goblin.team),
                GameEvent::KnockedOut { goblin } => count_for(&mut stats.knock_outs, goblin.team),
                GameEvent::AssaultResolved { contest, .. } => {
                    stats.assaults.record(contest.actor_won());
                }
                GameEvent::DodgeAttempted { contest, .. } => {
                    stats.dodges.record(contest.actor_won());
                }
                GameEvent::PickAttempted { roll, .. } => stats.picks.record(roll.success),
                GameEvent::ThrowAttempted { receiver, roll, .. } => {
                    stats.throws.tried += 1;
                    awaited_receiver = roll.success.then_some(receiver);
                }
                GameEvent::InterceptAttempted { goblin, roll } if roll.success => {
                    count_for(&mut stats.interceptions, goblin.team);
                    awaited_receiver = None;
                }
                GameEvent::CatchAttempted { goblin, roll } => {
                    let thrown_to_this_goblin = awaited_receiver.take() == Some(goblin);
                    stats.throws.succeeded += u32::from(thrown_to_this_goblin && roll.success);
                }
                _ => {}
            }
        }
        stats
    }

    /// The team that won, if one did.
    pub fn winner(&self) -> Option<TeamId> {
        match self.goals[0].cmp(&self.goals[1]) {
            Ordering::Greater => Some(TeamId::HOME),
            Ordering::Less => Some(TeamId::AWAY),
            Ordering::Equal => None,
        }
    }

    fn absorb(&mut self, other: &MatchStats) {
        add_pair(&mut self.goals, other.goals);
        add_pair(&mut self.goals_by_half, other.goals_by_half);
        add_pair(&mut self.turns, other.turns);
        add_pair(&mut self.knockdowns, other.knockdowns);
        add_pair(&mut self.fumbles, other.fumbles);
        add_pair(&mut self.interceptions, other.interceptions);
        add_pair(&mut self.stances, other.stances);
        add_pair(&mut self.knock_outs, other.knock_outs);
        self.assaults.absorb(other.assaults);
        self.dodges.absorb(other.dodges);
        self.picks.absorb(other.picks);
        self.throws.absorb(other.throws);
    }
}

/// Matches summed up for a balance report.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BalanceSummary {
    /// Matches counted.
    pub matches: u32,
    /// Wins per team.
    pub wins: [u32; 2],
    /// Matches won by the team that received the opening kick-off.
    pub first_receiver_wins: u32,
    /// Every match's statistics added together.
    pub totals: MatchStats,
}

impl BalanceSummary {
    /// Counts one more match.
    pub fn add(&mut self, stats: &MatchStats) {
        self.matches += 1;
        if let Some(winner) = stats.winner() {
            count_for(&mut self.wins, winner);
            if stats.first_receiver == Some(winner) {
                self.first_receiver_wins += 1;
            }
        }
        self.totals.absorb(stats);
    }

    /// The summary as lines of text, rates given per match or per team turn.
    pub fn report(&self) -> Vec<String> {
        let totals = &self.totals;
        let matches = f64::from(self.matches.max(1));
        let team_turns = f64::from((totals.turns[0] + totals.turns[1]).max(1));
        let share = |count: u32| 100.0 * f64::from(count) / matches;
        let per_match = |count: u32| f64::from(count) / matches;
        let per_turn = |counts: [u32; 2]| f64::from(counts[0] + counts[1]) / team_turns;
        let rate = |attempts: Attempts| {
            if attempts.tried == 0 {
                "none".to_string()
            } else {
                format!(
                    "{:.1} %",
                    100.0 * f64::from(attempts.succeeded) / f64::from(attempts.tried)
                )
            }
        };
        let draws = self
            .matches
            .saturating_sub(self.wins[0])
            .saturating_sub(self.wins[1]);
        vec![
            format!("Matches: {}", self.matches),
            format!(
                "Results: home {:.1} %, away {:.1} %, draws {:.1} %",
                share(self.wins[0]),
                share(self.wins[1]),
                share(draws)
            ),
            format!(
                "Opening receiver wins: {:.1} %",
                share(self.first_receiver_wins)
            ),
            format!(
                "Goals per match: {:.2} (home {:.2}, away {:.2}; first half {:.2}, second half {:.2})",
                per_match(totals.goals[0] + totals.goals[1]),
                per_match(totals.goals[0]),
                per_match(totals.goals[1]),
                per_match(totals.goals_by_half[0]),
                per_match(totals.goals_by_half[1])
            ),
            format!(
                "Per team turn: knockdowns {:.3}, fumbles {:.3}, interceptions {:.3}, stances {:.2}",
                per_turn(totals.knockdowns),
                per_turn(totals.fumbles),
                per_turn(totals.interceptions),
                per_turn(totals.stances)
            ),
            format!(
                "Per match: assaults {:.1} (won {}), dodges {:.1} (made {}), pick-ups {:.1} (made {}), throws {:.1} (caught {}), knock-outs {:.2}",
                per_match(totals.assaults.tried),
                rate(totals.assaults),
                per_match(totals.dodges.tried),
                rate(totals.dodges),
                per_match(totals.picks.tried),
                rate(totals.picks),
                per_match(totals.throws.tried),
                rate(totals.throws),
                per_match(totals.knock_outs[0] + totals.knock_outs[1])
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{BallRoll, ContestRoll};

    fn id(text: &str) -> GoblinId {
        text.parse().unwrap()
    }

    fn roll(success: bool) -> BallRoll {
        BallRoll {
            faces: 4,
            rolled: if success { 3 } else { 1 },
            malus: 0,
            target: 2,
            success,
        }
    }

    #[test]
    fn given_a_goal_after_half_time_when_counting_then_a_goal_for_the_scorer_in_the_second_half() {
        let events = [
            GameEvent::HalfEnded { half: 1 },
            GameEvent::Scored {
                team: TeamId::AWAY,
                scorer: id("A5"),
            },
        ];
        let stats = MatchStats::from_events(&events);
        assert_eq!((stats.goals, stats.goals_by_half), ([0, 1], [0, 1]));
    }

    #[test]
    fn given_a_turn_ended_by_a_knockdown_when_counting_then_a_knockdown_for_that_team() {
        let events = [GameEvent::TurnEnded {
            team: TeamId::HOME,
            reason: TurnEndReason::Knockdown,
        }];
        assert_eq!(MatchStats::from_events(&events).knockdowns, [1, 0]);
    }

    #[test]
    fn given_a_throw_caught_by_its_receiver_when_counting_then_one_throw_caught() {
        let events = [
            GameEvent::ThrowAttempted {
                thrower: id("H9"),
                receiver: id("H5"),
                roll: roll(true),
                distance: 5,
            },
            GameEvent::CatchAttempted {
                goblin: id("H5"),
                roll: roll(true),
            },
        ];
        assert_eq!(
            MatchStats::from_events(&events).throws,
            Attempts {
                tried: 1,
                succeeded: 1
            }
        );
    }

    #[test]
    fn given_a_failed_dodge_when_counting_then_a_dodge_tried_but_not_made() {
        let events = [GameEvent::DodgeAttempted {
            goblin: id("H5"),
            tackler: id("A1"),
            contest: ContestRoll {
                actor_faces: 4,
                actor_rolled: 1,
                opponent_faces: 4,
                opponent_rolled: 3,
                opponent_modifier: 0,
                margin: -2,
            },
        }];
        assert_eq!(
            MatchStats::from_events(&events).dodges,
            Attempts {
                tried: 1,
                succeeded: 0
            }
        );
    }

    #[test]
    fn given_the_opening_coin_toss_when_counting_then_the_first_receiver_is_known() {
        let events = [GameEvent::CoinTossed {
            receiving_team: TeamId::AWAY,
        }];
        assert_eq!(
            MatchStats::from_events(&events).first_receiver,
            Some(TeamId::AWAY)
        );
    }

    #[test]
    fn given_level_goals_when_naming_the_winner_then_nobody() {
        let stats = MatchStats {
            goals: [1, 1],
            ..MatchStats::default()
        };
        assert_eq!(stats.winner(), None);
    }

    #[test]
    fn given_a_home_win_by_the_first_receiver_and_a_draw_when_summing_then_both_are_counted() {
        let mut summary = BalanceSummary::default();
        summary.add(&MatchStats {
            goals: [1, 0],
            first_receiver: Some(TeamId::HOME),
            ..MatchStats::default()
        });
        summary.add(&MatchStats::default());
        assert_eq!(
            (summary.matches, summary.wins, summary.first_receiver_wins),
            (2, [1, 0], 1)
        );
    }

    #[test]
    fn given_a_home_win_and_a_draw_when_reporting_then_the_results_line_shows_the_shares() {
        let mut summary = BalanceSummary::default();
        summary.add(&MatchStats {
            goals: [1, 0],
            ..MatchStats::default()
        });
        summary.add(&MatchStats::default());
        assert!(
            summary
                .report()
                .contains(&"Results: home 50.0 %, away 0.0 %, draws 50.0 %".to_string())
        );
    }
}
