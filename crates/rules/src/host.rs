//! A match as hosted for coaches: who sits where, the turn clock, and the record that replays it.
//!
//! The clock is data: shells report elapsed time with [`MatchHost::advance_clock`], so tests and
//! replays never depend on a real clock.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::command::Command;
use crate::dice::SeededDice;
use crate::event::GameEvent;
use crate::game::{Game, SetupError};
use crate::roster::Roster;
use crate::ruleset::Ruleset;
use crate::state::GameState;
use crate::team::TeamId;
use crate::violation::RuleViolation;

/// Who coaches a team.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeatKind {
    /// A person at the keyboard: the turn clock waits until they say they are ready.
    Human,
    /// A scripted policy.
    Bot,
    /// An AI agent connected from outside.
    Agent,
}

/// Everything needed to start, and later replay, a match.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchSetup {
    /// Rules in force.
    pub ruleset: Ruleset,
    /// Home then away roster.
    pub rosters: [Roster; 2],
    /// Seed of the dice.
    pub seed: u64,
    /// Home then away seat.
    pub seats: [SeatKind; 2],
    /// Home then away seconds per turn.
    pub turn_seconds: [u16; 2],
    /// Seconds an agent's turn waits for the agent to look at it before its clock starts anyway.
    #[serde(default = "default_agent_ready_grace_seconds")]
    pub agent_ready_grace_seconds: u16,
}

fn default_agent_ready_grace_seconds() -> u16 {
    15
}

/// State of the turn clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "clock", rename_all = "snake_case")]
pub enum TurnClock {
    /// A human seat has not said it is ready, or an agent has not looked at its turn yet; no time is spent.
    WaitingForReady {
        /// Time the turn will have once started.
        remaining_ms: u64,
    },
    /// Time is running.
    Running {
        /// Time left in the turn.
        remaining_ms: u64,
    },
    /// Stopped on request (games between humans only).
    Paused {
        /// Time left when paused.
        remaining_ms: u64,
    },
    /// The match is over.
    Stopped,
}

/// One command applied to the match, with when it happened.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordEntry {
    /// Team the command was applied for.
    pub team: TeamId,
    /// The command, including the `TimeUp` issued by the clock.
    pub command: Command,
    /// Match time (sum of reported elapsed time) when it was applied, for metrics only.
    pub at_ms: u64,
}

/// The whole story of a match: its setup and every applied command, enough to replay it exactly.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchRecord {
    /// How the match was set up.
    pub setup: MatchSetup,
    /// Applied commands, in order.
    pub entries: Vec<RecordEntry>,
}

/// Why the host refuses a request.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum HostError {
    /// The rules refuse the command.
    #[error(transparent)]
    Rule(#[from] RuleViolation),
    /// Only a team whose human seat is waiting can declare itself ready.
    #[error("there is nothing to be ready for")]
    NotWaitingForReady,
    /// Pausing is only for matches between humans.
    #[error("only matches between humans can be paused")]
    PauseNotAllowed,
}

/// Why a record cannot be replayed.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ReplayError {
    /// The record's setup is invalid.
    #[error(transparent)]
    Setup(#[from] SetupError),
    /// A recorded command is refused by the rules, so the record does not match this engine.
    #[error("entry {index} is refused: {violation}")]
    Diverged {
        /// Index of the refused entry.
        index: usize,
        /// Why it is refused.
        violation: RuleViolation,
    },
    /// A record line is not valid JSON for its place.
    #[error("line {line} is unreadable: {reason}")]
    Unreadable {
        /// One-based line number.
        line: usize,
        /// Parser message.
        reason: String,
    },
}

/// Read-only picture of the match for screens and agents.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchSnapshot {
    /// Increases with every applied command.
    pub version: u64,
    /// The match state.
    pub state: GameState,
    /// The turn clock.
    pub clock: TurnClock,
    /// Home then away seat.
    pub seats: [SeatKind; 2],
}

/// A running match with its seats, clock and record.
#[derive(Clone, Debug)]
pub struct MatchHost {
    game: Game<SeededDice>,
    record: MatchRecord,
    clock: TurnClock,
    version: u64,
    match_time_ms: u64,
    waited_for_agent_ms: u64,
    log: Vec<GameEvent>,
}

impl MatchHost {
    /// Starts a match from its setup and returns the opening events.
    ///
    /// # Errors
    /// [`SetupError`] when the ruleset or a roster is invalid.
    pub fn start(setup: MatchSetup) -> Result<(Self, Vec<GameEvent>), SetupError> {
        let (game, events) = Game::start(
            setup.ruleset.clone(),
            setup.rosters,
            SeededDice::from_seed(setup.seed),
        )?;
        let mut host = Self {
            game,
            record: MatchRecord {
                setup,
                entries: Vec::new(),
            },
            clock: TurnClock::Stopped,
            version: 0,
            match_time_ms: 0,
            waited_for_agent_ms: 0,
            log: events.clone(),
        };
        host.set_clock_for_new_turn();
        Ok((host, events))
    }

    /// Starts the turn clock of a seat waiting for its coach: a human pressing Ready, or an agent
    /// looking at its turn.
    ///
    /// # Errors
    /// [`HostError::NotWaitingForReady`] when the clock is not waiting for `team`.
    pub fn ready_or_running(&mut self, team: TeamId) -> Result<(), HostError> {
        let is_active = self.game.state().active_team() == Some(team);
        if is_active && matches!(self.clock, TurnClock::Running { .. }) {
            return Ok(());
        }
        self.ready(team)
    }

    /// Applies a command for `team`, stamping it in the record.
    ///
    /// A command from a human seat still waiting counts as being ready.
    ///
    /// # Errors
    /// [`HostError::Rule`] when the rules refuse it; nothing changes then.
    pub fn submit(&mut self, team: TeamId, command: Command) -> Result<Vec<GameEvent>, HostError> {
        if let TurnClock::WaitingForReady { remaining_ms } = self.clock
            && self.game.state().active_team() == Some(team)
        {
            self.clock = TurnClock::Running { remaining_ms };
        }
        Ok(self.apply_and_record(team, command)?)
    }

    /// Starts the turn clock of a human seat that was waiting.
    ///
    /// # Errors
    /// [`HostError::NotWaitingForReady`] when the clock is not waiting for `team`.
    pub fn ready(&mut self, team: TeamId) -> Result<(), HostError> {
        match self.clock {
            TurnClock::WaitingForReady { remaining_ms }
                if self.game.state().active_team() == Some(team) =>
            {
                self.clock = TurnClock::Running { remaining_ms };
                Ok(())
            }
            _ => Err(HostError::NotWaitingForReady),
        }
    }

    /// Spends `elapsed_ms` of the running turn clock; when it runs out the turn ends on `TimeUp`,
    /// which is recorded like any other command.
    pub fn advance_clock(&mut self, elapsed_ms: u64) -> Vec<GameEvent> {
        self.match_time_ms = self.match_time_ms.saturating_add(elapsed_ms);
        match self.clock {
            TurnClock::WaitingForReady { remaining_ms }
                if self.active_seat() == Some(SeatKind::Agent) =>
            {
                self.waited_for_agent_ms = self.waited_for_agent_ms.saturating_add(elapsed_ms);
                let grace_ms = u64::from(self.record.setup.agent_ready_grace_seconds) * 1_000;
                if self.waited_for_agent_ms < grace_ms {
                    return Vec::new();
                }
                self.clock = TurnClock::Running { remaining_ms };
                self.spend_turn_time(self.waited_for_agent_ms - grace_ms)
            }
            TurnClock::Running { .. } => self.spend_turn_time(elapsed_ms),
            TurnClock::WaitingForReady { .. } | TurnClock::Paused { .. } | TurnClock::Stopped => {
                Vec::new()
            }
        }
    }

    fn spend_turn_time(&mut self, spent_ms: u64) -> Vec<GameEvent> {
        let TurnClock::Running { remaining_ms } = self.clock else {
            return Vec::new();
        };
        if spent_ms < remaining_ms {
            self.clock = TurnClock::Running {
                remaining_ms: remaining_ms - spent_ms,
            };
            return Vec::new();
        }
        let Some(team) = self.game.state().active_team() else {
            return Vec::new();
        };
        self.apply_and_record(team, Command::TimeUp)
            .unwrap_or_default()
    }

    fn active_seat(&self) -> Option<SeatKind> {
        let team = self.game.state().active_team()?;
        self.record.setup.seats.get(team.index()).copied()
    }

    /// Freezes the running clock, in matches between humans only.
    ///
    /// # Errors
    /// [`HostError::PauseNotAllowed`] when a bot or an agent is seated.
    pub fn pause(&mut self) -> Result<(), HostError> {
        if self.record.setup.seats != [SeatKind::Human, SeatKind::Human] {
            return Err(HostError::PauseNotAllowed);
        }
        if let TurnClock::Running { remaining_ms } = self.clock {
            self.clock = TurnClock::Paused { remaining_ms };
        }
        Ok(())
    }

    /// Restarts a paused clock.
    pub fn resume(&mut self) {
        if let TurnClock::Paused { remaining_ms } = self.clock {
            self.clock = TurnClock::Running { remaining_ms };
        }
    }

    /// The current snapshot.
    pub fn snapshot(&self) -> MatchSnapshot {
        MatchSnapshot {
            version: self.version,
            state: self.game.state().clone(),
            clock: self.clock,
            seats: self.record.setup.seats,
        }
    }

    /// The record so far.
    pub fn record(&self) -> &MatchRecord {
        &self.record
    }

    /// The running game, for odds and paths.
    pub fn game(&self) -> &Game<SeededDice> {
        &self.game
    }

    /// Events since the start of the match, from position `cursor` on.
    pub fn events_since(&self, cursor: usize) -> &[GameEvent] {
        self.log.get(cursor..).unwrap_or_default()
    }

    /// Number of events since the start of the match.
    pub fn event_count(&self) -> usize {
        self.log.len()
    }

    fn apply_and_record(
        &mut self,
        team: TeamId,
        command: Command,
    ) -> Result<Vec<GameEvent>, RuleViolation> {
        let events = self.game.apply(team, command.clone())?;
        self.record.entries.push(RecordEntry {
            team,
            command,
            at_ms: self.match_time_ms,
        });
        self.version += 1;
        self.log.extend(events.iter().cloned());
        let turn_changed = events.iter().any(|event| {
            matches!(
                event,
                GameEvent::TurnStarted { .. } | GameEvent::GameEnded { .. }
            )
        });
        if turn_changed {
            self.set_clock_for_new_turn();
        }
        Ok(events)
    }

    fn set_clock_for_new_turn(&mut self) {
        self.waited_for_agent_ms = 0;
        let setup = &self.record.setup;
        self.clock = match self.game.state().active_team() {
            None => TurnClock::Stopped,
            Some(team) => {
                let seconds = setup.turn_seconds.get(team.index()).copied().unwrap_or(60);
                let remaining_ms = u64::from(seconds) * 1_000;
                match setup.seats.get(team.index()) {
                    Some(SeatKind::Human | SeatKind::Agent) => {
                        TurnClock::WaitingForReady { remaining_ms }
                    }
                    Some(SeatKind::Bot) | None => TurnClock::Running { remaining_ms },
                }
            }
        };
    }
}

impl MatchRecord {
    /// Replays the record from its setup and returns the resulting game.
    ///
    /// # Errors
    /// [`ReplayError`] when the setup is invalid or a command is refused.
    pub fn replay(&self) -> Result<Game<SeededDice>, ReplayError> {
        let (mut game, _) = Game::start(
            self.setup.ruleset.clone(),
            self.setup.rosters,
            SeededDice::from_seed(self.setup.seed),
        )?;
        for (index, entry) in self.entries.iter().enumerate() {
            game.apply(entry.team, entry.command.clone())
                .map_err(|violation| ReplayError::Diverged { index, violation })?;
        }
        Ok(game)
    }

    /// The record as JSON lines: the setup first, then one line per entry.
    pub fn to_json_lines(&self) -> String {
        std::iter::once(serde_json::to_string(&self.setup))
            .chain(self.entries.iter().map(serde_json::to_string))
            .filter_map(Result::ok)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Reads a record written by [`MatchRecord::to_json_lines`].
    ///
    /// # Errors
    /// [`ReplayError::Unreadable`] naming the first bad line.
    pub fn from_json_lines(text: &str) -> Result<MatchRecord, ReplayError> {
        let unreadable = |line: usize, error: serde_json::Error| ReplayError::Unreadable {
            line,
            reason: error.to_string(),
        };
        let mut lines = text
            .lines()
            .enumerate()
            .map(|(index, line)| (index + 1, line))
            .filter(|(_, line)| !line.trim().is_empty());
        let (setup_line, setup_text) = lines.next().ok_or(ReplayError::Unreadable {
            line: 1,
            reason: "the record is empty".to_string(),
        })?;
        let setup =
            serde_json::from_str(setup_text).map_err(|error| unreadable(setup_line, error))?;
        let entries = lines
            .map(|(line, entry)| {
                serde_json::from_str(entry).map_err(|error| unreadable(line, error))
            })
            .collect::<Result<Vec<RecordEntry>, ReplayError>>()?;
        Ok(MatchRecord { setup, entries })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Phase;

    fn setup(seats: [SeatKind; 2]) -> MatchSetup {
        MatchSetup {
            ruleset: Ruleset::default(),
            rosters: [Roster::DEFAULT, Roster::DEFAULT],
            seed: 42,
            seats,
            turn_seconds: [60, 60],
            agent_ready_grace_seconds: 15,
        }
    }

    fn agents() -> MatchHost {
        MatchHost::start(setup([SeatKind::Agent, SeatKind::Agent]))
            .unwrap()
            .0
    }

    fn active_team(host: &MatchHost) -> TeamId {
        host.snapshot().state.active_team().unwrap()
    }

    fn bots() -> MatchHost {
        MatchHost::start(setup([SeatKind::Bot, SeatKind::Bot]))
            .unwrap()
            .0
    }

    #[test]
    fn given_bot_seats_when_the_match_starts_then_the_clock_runs_a_full_minute() {
        assert_eq!(
            bots().snapshot().clock,
            TurnClock::Running {
                remaining_ms: 60_000
            }
        );
    }

    #[test]
    fn given_agent_seats_when_the_match_starts_then_the_clock_waits_for_the_agent_to_look() {
        assert_eq!(
            agents().snapshot().clock,
            TurnClock::WaitingForReady {
                remaining_ms: 60_000
            }
        );
    }

    #[test]
    fn given_an_agent_looking_at_its_turn_when_ready_then_its_clock_runs() {
        let mut host = agents();
        let team = active_team(&host);
        host.ready(team).unwrap();
        assert_eq!(
            host.snapshot().clock,
            TurnClock::Running {
                remaining_ms: 60_000
            }
        );
    }

    #[test]
    fn given_an_agent_that_does_not_look_when_the_grace_runs_out_then_its_clock_runs() {
        let mut host = agents();
        host.advance_clock(15_000);
        assert_eq!(
            host.snapshot().clock,
            TurnClock::Running {
                remaining_ms: 60_000
            }
        );
    }

    #[test]
    fn given_an_agent_that_does_not_look_when_twenty_seconds_pass_then_five_count_against_its_turn()
    {
        let mut host = agents();
        host.advance_clock(20_000);
        assert_eq!(
            host.snapshot().clock,
            TurnClock::Running {
                remaining_ms: 55_000
            }
        );
    }

    #[test]
    fn given_a_human_that_does_not_press_ready_when_a_long_time_passes_then_its_clock_still_waits()
    {
        let mut host = MatchHost::start(setup([SeatKind::Human, SeatKind::Human]))
            .unwrap()
            .0;
        host.advance_clock(600_000);
        assert_eq!(
            host.snapshot().clock,
            TurnClock::WaitingForReady {
                remaining_ms: 60_000
            }
        );
    }

    #[test]
    fn given_a_running_clock_when_its_seat_says_ready_or_running_then_accepted() {
        let mut host = bots();
        let team = active_team(&host);
        assert_eq!(host.ready_or_running(team), Ok(()));
    }

    #[test]
    fn given_the_idle_seat_when_it_says_ready_or_running_then_not_waiting_for_ready() {
        let mut host = bots();
        let idle = active_team(&host).duel_opponent();
        assert_eq!(
            host.ready_or_running(idle),
            Err(HostError::NotWaitingForReady)
        );
    }

    #[test]
    fn given_human_seats_when_the_match_starts_then_the_clock_waits_for_ready() {
        let host = MatchHost::start(setup([SeatKind::Human, SeatKind::Human]))
            .unwrap()
            .0;
        assert_eq!(
            host.snapshot().clock,
            TurnClock::WaitingForReady {
                remaining_ms: 60_000
            }
        );
    }

    #[test]
    fn given_a_waiting_clock_when_time_passes_then_no_time_is_spent() {
        let mut host = MatchHost::start(setup([SeatKind::Human, SeatKind::Human]))
            .unwrap()
            .0;
        host.advance_clock(5_000);
        assert_eq!(
            host.snapshot().clock,
            TurnClock::WaitingForReady {
                remaining_ms: 60_000
            }
        );
    }

    #[test]
    fn given_a_waiting_human_when_ready_then_the_clock_runs() {
        let mut host = MatchHost::start(setup([SeatKind::Human, SeatKind::Human]))
            .unwrap()
            .0;
        let team = active_team(&host);
        host.ready(team).unwrap();
        assert_eq!(
            host.snapshot().clock,
            TurnClock::Running {
                remaining_ms: 60_000
            }
        );
    }

    #[test]
    fn given_a_running_clock_when_ready_is_sent_then_not_waiting_for_ready() {
        let mut host = bots();
        let team = active_team(&host);
        assert_eq!(host.ready(team), Err(HostError::NotWaitingForReady));
    }

    #[test]
    fn given_a_running_clock_when_thirty_seconds_pass_then_thirty_remain() {
        let mut host = bots();
        host.advance_clock(30_000);
        assert_eq!(
            host.snapshot().clock,
            TurnClock::Running {
                remaining_ms: 30_000
            }
        );
    }

    #[test]
    fn given_a_running_clock_when_a_minute_passes_then_the_turn_ends_on_time_up() {
        let mut host = bots();
        let team = active_team(&host);
        let events = host.advance_clock(60_000);
        assert!(events.contains(&GameEvent::TurnEnded {
            team,
            reason: crate::event::TurnEndReason::TimeUp
        }));
    }

    #[test]
    fn given_a_turn_lost_on_time_when_reading_the_record_then_time_up_is_recorded() {
        let mut host = bots();
        host.advance_clock(60_000);
        assert_eq!(
            host.record()
                .entries
                .last()
                .map(|entry| entry.command.clone()),
            Some(Command::TimeUp)
        );
    }

    #[test]
    fn given_a_turn_lost_on_time_when_the_next_turn_starts_then_its_clock_is_full() {
        let mut host = bots();
        host.advance_clock(60_000);
        assert_eq!(
            host.snapshot().clock,
            TurnClock::Running {
                remaining_ms: 60_000
            }
        );
    }

    #[test]
    fn given_a_slower_home_seat_when_home_plays_then_its_clock_has_ninety_seconds() {
        let mut slow_home = setup([SeatKind::Bot, SeatKind::Bot]);
        slow_home.turn_seconds = [90, 60];
        let mut host = MatchHost::start(slow_home).unwrap().0;
        if active_team(&host) == TeamId::AWAY {
            host.submit(TeamId::AWAY, Command::EndTurn).unwrap();
        }
        assert_eq!(
            host.snapshot().clock,
            TurnClock::Running {
                remaining_ms: 90_000
            }
        );
    }

    #[test]
    fn given_an_applied_command_when_reading_the_snapshot_then_the_version_went_up() {
        let mut host = agents();
        let before = host.snapshot().version;
        let team = active_team(&host);
        host.submit(team, Command::EndTurn).unwrap();
        assert_eq!(host.snapshot().version, before + 1);
    }

    #[test]
    fn given_a_refused_command_when_submitted_then_the_version_stays() {
        let mut host = agents();
        let before = host.snapshot().version;
        let idle = active_team(&host).duel_opponent();
        let _ = host.submit(idle, Command::EndTurn);
        assert_eq!(host.snapshot().version, before);
    }

    #[test]
    fn given_humans_when_pausing_then_the_clock_is_paused() {
        let mut host = MatchHost::start(setup([SeatKind::Human, SeatKind::Human]))
            .unwrap()
            .0;
        let team = active_team(&host);
        host.ready(team).unwrap();
        host.pause().unwrap();
        host.advance_clock(10_000);
        assert_eq!(
            host.snapshot().clock,
            TurnClock::Paused {
                remaining_ms: 60_000
            }
        );
    }

    #[test]
    fn given_a_paused_clock_when_resumed_then_it_runs_again() {
        let mut host = MatchHost::start(setup([SeatKind::Human, SeatKind::Human]))
            .unwrap()
            .0;
        let team = active_team(&host);
        host.ready(team).unwrap();
        host.pause().unwrap();
        host.resume();
        host.advance_clock(10_000);
        assert_eq!(
            host.snapshot().clock,
            TurnClock::Running {
                remaining_ms: 50_000
            }
        );
    }

    #[test]
    fn given_an_agent_seated_when_pausing_then_pause_not_allowed() {
        assert_eq!(agents().pause(), Err(HostError::PauseNotAllowed));
    }

    #[test]
    fn given_a_whole_match_lost_on_time_when_replayed_then_the_same_final_state() {
        let mut host = agents();
        while host.snapshot().state.phase != Phase::Finished {
            host.advance_clock(60_000);
        }
        assert_eq!(
            host.record().replay().unwrap().state(),
            &host.snapshot().state
        );
    }

    #[test]
    fn given_a_whole_match_lost_on_time_when_it_ends_then_the_clock_stops() {
        let mut host = agents();
        while host.snapshot().state.phase != Phase::Finished {
            host.advance_clock(60_000);
        }
        assert_eq!(host.snapshot().clock, TurnClock::Stopped);
    }

    #[test]
    fn given_a_record_when_written_and_read_back_as_json_lines_then_unchanged() {
        let mut host = agents();
        host.advance_clock(60_000);
        host.advance_clock(60_000);
        let text = host.record().to_json_lines();
        assert_eq!(
            MatchRecord::from_json_lines(&text),
            Ok(host.record().clone())
        );
    }

    #[test]
    fn given_a_record_with_a_broken_line_when_read_then_unreadable_on_that_line() {
        let mut host = bots();
        host.advance_clock(60_000);
        let text = format!("{}\nnot json", host.record().to_json_lines());
        assert!(matches!(
            MatchRecord::from_json_lines(&text),
            Err(ReplayError::Unreadable { line: 3, .. })
        ));
    }

    #[test]
    fn given_events_so_far_when_reading_from_a_cursor_then_only_the_newer_ones() {
        let mut host = agents();
        let cursor = host.event_count();
        let team = active_team(&host);
        host.submit(team, Command::EndTurn).unwrap();
        assert_eq!(
            host.events_since(cursor).first(),
            Some(&GameEvent::TurnEnded {
                team,
                reason: crate::event::TurnEndReason::EndTurn
            })
        );
    }
}
