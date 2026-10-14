//! The scoresheet of a match: what each team did, tallied from the match events as they happen.

use crate::balance::Attempts;
use crate::event::{GameEvent, TurnEndReason};
use crate::roster::GoblinId;
use crate::team::TeamId;

/// How a team's turns ended.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TurnEndings {
    /// Ended by its coach.
    pub by_choice: u32,
    /// Ended by the clock.
    pub by_clock: u32,
    /// Ended by a knockdown of one of its goblins.
    pub by_knockdown: u32,
    /// Ended by a fumble or an interception.
    pub by_fumble: u32,
    /// Ended by a goal, scored by either team.
    pub by_score: u32,
    /// Ended on their own once none of its goblins could act, under rules that allow it.
    pub with_no_goblin_left: u32,
}

impl TurnEndings {
    fn count(&mut self, reason: TurnEndReason) {
        let endings = match reason {
            TurnEndReason::EndTurn => &mut self.by_choice,
            TurnEndReason::TimeUp => &mut self.by_clock,
            TurnEndReason::Knockdown => &mut self.by_knockdown,
            TurnEndReason::Fumble => &mut self.by_fumble,
            TurnEndReason::Score => &mut self.by_score,
            TurnEndReason::NoGoblinCanAct => &mut self.with_no_goblin_left,
        };
        *endings += 1;
    }
}

/// What one team did in a match.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TeamSheet {
    /// Its assaults, and those it won.
    pub assaults: Attempts,
    /// Assaults it suffered with a Stance bonus, and those it held.
    pub braced_defences: Attempts,
    /// Assaults it suffered without a Stance bonus, and those it held.
    pub open_defences: Attempts,
    /// Its dodges, and those that slipped away.
    pub dodges: Attempts,
    /// Its passes, and those caught by their receiver.
    pub passes: Attempts,
    /// Its catches of passes, bounces and kick-offs, and those it held.
    pub catches: Attempts,
    /// Its pick-ups, and those it made.
    pub pickups: Attempts,
    /// Its interceptions, and those that snatched the ball.
    pub interceptions: Attempts,
    /// Squares its goblins walked holding the ball.
    pub steps_with_the_ball: u32,
    /// Squares flown by its longest completed pass.
    pub longest_pass: u8,
    /// Squares flown by all its completed passes.
    pub completed_pass_distance: u32,
    /// Its goblins knocked out.
    pub knock_outs: u32,
    /// How its turns ended.
    pub turn_endings: TurnEndings,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PassInFlight {
    team: TeamId,
    receiver: GoblinId,
    distance: u8,
}

/// Both teams' sheets, tallied event after event.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Scoresheet {
    teams: [TeamSheet; 2],
    carrier: Option<GoblinId>,
    pass_in_flight: Option<PassInFlight>,
}

impl Scoresheet {
    /// The sheet once `events` happened, in order, after those already tallied.
    pub fn tally(self, events: &[GameEvent]) -> Self {
        events.iter().fold(self, Scoresheet::after)
    }

    /// The sheet of `team`.
    pub fn team(&self, team: TeamId) -> TeamSheet {
        self.teams.get(team.index()).copied().unwrap_or_default()
    }

    fn after(mut self, event: &GameEvent) -> Self {
        match *event {
            GameEvent::AssaultResolved {
                attacker,
                defender,
                contest,
                defender_braced,
            } => {
                let attacker_won = contest.actor_won();
                self.update(attacker.team, |sheet| sheet.assaults.record(attacker_won));
                self.update(defender.team, |sheet| {
                    let defences = if defender_braced {
                        &mut sheet.braced_defences
                    } else {
                        &mut sheet.open_defences
                    };
                    defences.record(!attacker_won);
                });
            }
            GameEvent::DodgeAttempted { goblin, roll } => {
                self.update(goblin.team, |sheet| sheet.dodges.record(roll.succeeded()));
            }
            GameEvent::PickAttempted { goblin, roll } => {
                self.update(goblin.team, |sheet| sheet.pickups.record(roll.success));
                if roll.success {
                    self.carrier = Some(goblin);
                }
            }
            GameEvent::ThrowAttempted {
                thrower,
                receiver,
                roll,
                distance,
            } => {
                self.carrier = None;
                self.pass_in_flight = roll.success.then_some(PassInFlight {
                    team: thrower.team,
                    receiver,
                    distance,
                });
                if !roll.success {
                    self.update(thrower.team, |sheet| sheet.passes.record(false));
                }
            }
            GameEvent::HandedOff { giver, receiver } => {
                self.carrier = None;
                self.pass_in_flight = Some(PassInFlight {
                    team: giver.team,
                    receiver,
                    distance: 1,
                });
            }
            GameEvent::InterceptAttempted { goblin, roll } => {
                self.update(goblin.team, |sheet| {
                    sheet.interceptions.record(roll.success)
                });
                if roll.success {
                    self.carrier = Some(goblin);
                    if let Some(pass) = self.pass_in_flight.take() {
                        self.update(pass.team, |sheet| sheet.passes.record(false));
                    }
                }
            }
            GameEvent::CatchAttempted { goblin, roll } => {
                self.update(goblin.team, |sheet| sheet.catches.record(roll.success));
                if roll.success {
                    self.carrier = Some(goblin);
                }
                if let Some(pass) = self.pass_in_flight.take_if(|pass| pass.receiver == goblin) {
                    self.update(pass.team, |sheet| {
                        sheet.passes.record(roll.success);
                        if roll.success {
                            sheet.longest_pass = sheet.longest_pass.max(pass.distance);
                            sheet.completed_pass_distance += u32::from(pass.distance);
                        }
                    });
                }
            }
            GameEvent::Moved { goblin, .. } if self.carrier == Some(goblin) => {
                self.update(goblin.team, |sheet| sheet.steps_with_the_ball += 1);
            }
            GameEvent::KnockedDown { goblin, .. } if self.carrier == Some(goblin) => {
                self.carrier = None;
            }
            GameEvent::KnockedOut { goblin } => {
                self.update(goblin.team, |sheet| sheet.knock_outs += 1);
            }
            GameEvent::KickedOff { .. }
            | GameEvent::BallDropped { .. }
            | GameEvent::BallBounced { .. } => self.carrier = None,
            GameEvent::TurnEnded { team, reason } => {
                self.update(team, |sheet| sheet.turn_endings.count(reason));
            }
            _ => {}
        }
        self
    }

    fn update(&mut self, team: TeamId, change: impl FnOnce(&mut TeamSheet)) {
        if let Some(sheet) = self.teams.get_mut(team.index()) {
            change(sheet);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{AgilityRoll, ContestRoll, DodgeRoll};
    use crate::square::Square;

    fn id(text: &str) -> GoblinId {
        text.parse().unwrap()
    }

    fn roll(success: bool) -> AgilityRoll {
        AgilityRoll {
            faces: 4,
            rolled: if success { 3 } else { 1 },
            malus: 0,
            target: 2,
            success,
        }
    }

    fn contest(margin: i8) -> ContestRoll {
        ContestRoll {
            actor_faces: 4,
            actor_rolled: 3,
            opponent_faces: 4,
            opponent_rolled: 3,
            opponent_modifier: -margin,
            margin,
        }
    }

    fn attempts(tried: u32, succeeded: u32) -> Attempts {
        Attempts { tried, succeeded }
    }

    fn sheet_of(team: TeamId, events: &[GameEvent]) -> TeamSheet {
        Scoresheet::default().tally(events).team(team)
    }

    fn assault(attacker: &str, defender: &str, margin: i8, defender_braced: bool) -> GameEvent {
        GameEvent::AssaultResolved {
            attacker: id(attacker),
            defender: id(defender),
            contest: contest(margin),
            defender_braced,
        }
    }

    fn pass(thrower: &str, receiver: &str, success: bool, distance: u8) -> GameEvent {
        GameEvent::ThrowAttempted {
            thrower: id(thrower),
            receiver: id(receiver),
            roll: roll(success),
            distance,
        }
    }

    fn catch(goblin: &str, success: bool) -> GameEvent {
        GameEvent::CatchAttempted {
            goblin: id(goblin),
            roll: roll(success),
        }
    }

    fn step(goblin: &str, x: u8) -> GameEvent {
        GameEvent::Moved {
            goblin: id(goblin),
            from: Square::new(x, 10),
            to: Square::new(x + 1, 10),
        }
    }

    #[test]
    fn given_an_assault_won_by_home_when_tallied_then_home_won_an_assault() {
        let events = [assault("H1", "A1", 2, false)];

        let home = sheet_of(TeamId::HOME, &events);

        assert_eq!(home.assaults, attempts(1, 1));
    }

    #[test]
    fn given_an_assault_won_by_home_when_tallied_then_away_lost_a_defence_without_stance() {
        let events = [assault("H1", "A1", 2, false)];

        let away = sheet_of(TeamId::AWAY, &events);

        assert_eq!(
            (away.open_defences, away.braced_defences),
            (attempts(1, 0), attempts(0, 0))
        );
    }

    #[test]
    fn given_a_braced_defender_holding_an_assault_when_tallied_then_away_held_a_defence_in_stance()
    {
        let events = [assault("H1", "A1", -1, true)];

        let away = sheet_of(TeamId::AWAY, &events);

        assert_eq!(away.braced_defences, attempts(1, 1));
    }

    #[test]
    fn given_a_failed_dodge_when_tallied_then_its_team_failed_one_dodge() {
        let events = [GameEvent::DodgeAttempted {
            goblin: id("A5"),
            roll: DodgeRoll::Agility(roll(false)),
        }];

        let away = sheet_of(TeamId::AWAY, &events);

        assert_eq!(away.dodges, attempts(1, 0));
    }

    #[test]
    fn given_a_hand_off_caught_when_tallied_then_a_completed_pass_of_one_square() {
        let events = [
            GameEvent::HandedOff {
                giver: id("H9"),
                receiver: id("H5"),
            },
            catch("H5", true),
        ];

        let home = sheet_of(TeamId::HOME, &events);

        assert_eq!(
            (home.passes, home.longest_pass, home.completed_pass_distance),
            (attempts(1, 1), 1, 1)
        );
    }

    #[test]
    fn given_a_seven_square_pass_caught_when_tallied_then_a_completed_pass_of_seven_squares() {
        let events = [pass("H9", "H5", true, 7), catch("H5", true)];

        let home = sheet_of(TeamId::HOME, &events);

        assert_eq!(
            (
                home.passes,
                home.longest_pass,
                home.completed_pass_distance,
                home.catches
            ),
            (attempts(1, 1), 7, 7, attempts(1, 1))
        );
    }

    #[test]
    fn given_two_completed_passes_when_tallied_then_the_longest_and_their_total_distance() {
        let events = [
            pass("H9", "H5", true, 7),
            catch("H5", true),
            pass("H5", "H6", true, 4),
            catch("H6", true),
        ];

        let home = sheet_of(TeamId::HOME, &events);

        assert_eq!((home.longest_pass, home.completed_pass_distance), (7, 11));
    }

    #[test]
    fn given_an_intercepted_pass_when_tallied_then_the_pass_failed_and_the_other_team_intercepted()
    {
        let events = [
            pass("H9", "H5", true, 7),
            GameEvent::InterceptAttempted {
                goblin: id("A2"),
                roll: roll(true),
            },
        ];

        let sheet = Scoresheet::default().tally(&events);

        assert_eq!(
            (
                sheet.team(TeamId::HOME).passes,
                sheet.team(TeamId::AWAY).interceptions
            ),
            (attempts(1, 0), attempts(1, 1))
        );
    }

    #[test]
    fn given_a_pass_dropped_by_its_receiver_when_tallied_then_a_failed_pass_and_a_failed_catch() {
        let events = [pass("H9", "H5", true, 3), catch("H5", false)];

        let home = sheet_of(TeamId::HOME, &events);

        assert_eq!(
            (home.passes, home.catches, home.longest_pass),
            (attempts(1, 0), attempts(1, 0), 0)
        );
    }

    #[test]
    fn given_a_throw_roll_that_failed_when_tallied_then_a_failed_pass() {
        let events = [pass("H9", "H5", false, 3)];

        let home = sheet_of(TeamId::HOME, &events);

        assert_eq!(home.passes, attempts(1, 0));
    }

    #[test]
    fn given_a_goblin_picking_the_ball_up_then_walking_two_squares_when_tallied_then_two_steps_with_the_ball()
     {
        let events = [
            GameEvent::PickAttempted {
                goblin: id("A6"),
                roll: roll(true),
            },
            step("A6", 9),
            step("A6", 10),
        ];

        let away = sheet_of(TeamId::AWAY, &events);

        assert_eq!(
            (away.pickups, away.steps_with_the_ball),
            (attempts(1, 1), 2)
        );
    }

    #[test]
    fn given_a_goblin_walking_without_the_ball_when_tallied_then_no_step_with_the_ball() {
        let events = [step("H3", 9)];

        let home = sheet_of(TeamId::HOME, &events);

        assert_eq!(home.steps_with_the_ball, 0);
    }

    #[test]
    fn given_a_carrier_knocked_down_when_it_walks_again_then_those_steps_are_not_with_the_ball() {
        let events = [
            catch("H5", true),
            step("H5", 9),
            GameEvent::KnockedDown {
                goblin: id("H5"),
                at: Square::new(10, 10),
            },
            step("H5", 10),
        ];

        let home = sheet_of(TeamId::HOME, &events);

        assert_eq!(home.steps_with_the_ball, 1);
    }

    #[test]
    fn given_a_knocked_out_goblin_when_tallied_then_its_team_counts_one_knock_out() {
        let events = [GameEvent::KnockedOut { goblin: id("A1") }];

        let away = sheet_of(TeamId::AWAY, &events);

        assert_eq!(away.knock_outs, 1);
    }

    #[test]
    fn given_turns_ended_by_choice_clock_knockdown_and_fumble_when_tallied_then_each_counts_once() {
        let events = [
            TurnEndReason::EndTurn,
            TurnEndReason::TimeUp,
            TurnEndReason::Knockdown,
            TurnEndReason::Fumble,
        ]
        .map(|reason| GameEvent::TurnEnded {
            team: TeamId::HOME,
            reason,
        });

        let home = sheet_of(TeamId::HOME, &events);

        assert_eq!(
            home.turn_endings,
            TurnEndings {
                by_choice: 1,
                by_clock: 1,
                by_knockdown: 1,
                by_fumble: 1,
                by_score: 0,
                with_no_goblin_left: 0,
            }
        );
    }

    #[test]
    fn given_events_tallied_in_two_batches_when_reading_the_sheet_then_it_matches_one_batch() {
        let events = [pass("H9", "H5", true, 7), catch("H5", true), step("H5", 9)];

        let in_two = Scoresheet::default()
            .tally(&events[..1])
            .tally(&events[1..]);

        assert_eq!(in_two, Scoresheet::default().tally(&events));
    }
}
