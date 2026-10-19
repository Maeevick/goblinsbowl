//! The authoritative match, running in a tokio task: it owns the [`MatchHost`], ticks the turn clock,
//! plays bot seats, and serves every other seat through a [`MatchClient`].

use std::sync::Arc;
use std::time::Duration;

use goblins_rules::arena::Arena;
use goblins_rules::command::Command;
use goblins_rules::dice::SeededDice;
use goblins_rules::event::GameEvent;
use goblins_rules::host::{HostError, MatchHost, MatchRecord, MatchSnapshot, SeatKind};
use goblins_rules::policies::Bot;
use goblins_rules::ruleset::Ruleset;
use goblins_rules::team::TeamId;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot, watch};
use tokio::time::Instant;

const REQUEST_QUEUE: usize = 64;

/// How the server task paces itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServerPacing {
    /// How often the clock is advanced and bots get a chance to play.
    pub tick: Duration,
    /// Pause between two bot commands, so people can follow.
    pub bot_delay: Duration,
    /// How the bot seats play.
    pub bot: Bot,
    /// Seed of the bots' decisions, for the random bot.
    pub bot_seed: u64,
}

impl Default for ServerPacing {
    fn default() -> Self {
        Self {
            tick: Duration::from_millis(100),
            bot_delay: Duration::from_millis(400),
            bot: Bot::Balanced2,
            bot_seed: 7,
        }
    }
}

/// Why a request to the match server failed.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ClientError {
    /// The host refused the request.
    #[error(transparent)]
    Host(#[from] HostError),
    /// The match server is gone or overloaded.
    #[error("the match server is not answering")]
    ServerUnavailable,
}

enum SeatRequest {
    Submit {
        team: TeamId,
        command: Command,
        reply: oneshot::Sender<Result<Vec<GameEvent>, HostError>>,
    },
    Ready {
        team: TeamId,
        reply: oneshot::Sender<Result<(), HostError>>,
    },
    EventsSince {
        cursor: usize,
        reply: oneshot::Sender<(Vec<GameEvent>, usize)>,
    },
    Record {
        reply: oneshot::Sender<MatchRecord>,
    },
}

/// A seat's handle on the running match: cheap to clone, usable from any task.
#[derive(Clone, Debug)]
pub struct MatchClient {
    requests: mpsc::Sender<SeatRequest>,
    snapshots: watch::Receiver<Arc<MatchSnapshot>>,
    arena: Arc<Arena>,
    ruleset: Arc<Ruleset>,
}

impl std::fmt::Debug for SeatRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SeatRequest")
    }
}

impl MatchClient {
    /// The latest published snapshot.
    pub fn latest(&self) -> Arc<MatchSnapshot> {
        self.snapshots.borrow().clone()
    }

    /// The arena of the match.
    pub fn arena(&self) -> &Arena {
        &self.arena
    }

    /// The ruleset of the match.
    pub fn ruleset(&self) -> &Ruleset {
        &self.ruleset
    }

    /// Applies a command for `team` and returns the events it caused.
    ///
    /// # Errors
    /// [`ClientError::Host`] when refused, [`ClientError::ServerUnavailable`] when the server is gone.
    pub async fn submit(
        &self,
        team: TeamId,
        command: Command,
    ) -> Result<Vec<GameEvent>, ClientError> {
        let (reply, answer) = oneshot::channel();
        self.send(SeatRequest::Submit {
            team,
            command,
            reply,
        })?;
        Ok(answer.await.map_err(|_| ClientError::ServerUnavailable)??)
    }

    /// Starts the clock of a seat waiting for its coach; does nothing if that clock already runs.
    ///
    /// # Errors
    /// [`ClientError::Host`] when it is not `team`'s turn, [`ClientError::ServerUnavailable`] when the server is gone.
    pub async fn ready(&self, team: TeamId) -> Result<(), ClientError> {
        let (reply, answer) = oneshot::channel();
        self.send(SeatRequest::Ready { team, reply })?;
        Ok(answer.await.map_err(|_| ClientError::ServerUnavailable)??)
    }

    /// Events from position `cursor` on, and the position after the last one.
    ///
    /// # Errors
    /// [`ClientError::ServerUnavailable`] when the server is gone.
    pub async fn events_since(
        &self,
        cursor: usize,
    ) -> Result<(Vec<GameEvent>, usize), ClientError> {
        let (reply, answer) = oneshot::channel();
        self.send(SeatRequest::EventsSince { cursor, reply })?;
        answer.await.map_err(|_| ClientError::ServerUnavailable)
    }

    /// The record of the match so far, enough to replay it.
    ///
    /// # Errors
    /// [`ClientError::ServerUnavailable`] when the server is gone.
    pub async fn record(&self) -> Result<MatchRecord, ClientError> {
        let (reply, answer) = oneshot::channel();
        self.send(SeatRequest::Record { reply })?;
        answer.await.map_err(|_| ClientError::ServerUnavailable)
    }

    /// Waits until `condition` holds for the latest snapshot or `timeout` passes, and returns that snapshot.
    pub async fn wait_until(
        &self,
        timeout: Duration,
        condition: impl Fn(&MatchSnapshot) -> bool,
    ) -> Arc<MatchSnapshot> {
        let mut snapshots = self.snapshots.clone();
        let deadline = Instant::now() + timeout;
        loop {
            let latest = snapshots.borrow_and_update().clone();
            if condition(&latest) {
                return latest;
            }
            let changed = tokio::time::timeout_at(deadline, snapshots.changed()).await;
            if !matches!(changed, Ok(Ok(()))) {
                return snapshots.borrow().clone();
            }
        }
    }

    fn send(&self, request: SeatRequest) -> Result<(), ClientError> {
        self.requests
            .try_send(request)
            .map_err(|_| ClientError::ServerUnavailable)
    }
}

/// Starts the match server on the current tokio runtime and returns a client for the seats.
pub fn spawn_match(host: MatchHost, pacing: ServerPacing) -> MatchClient {
    let (requests, inbox) = mpsc::channel(REQUEST_QUEUE);
    let (publisher, snapshots) = watch::channel(Arc::new(host.snapshot()));
    let client = MatchClient {
        requests,
        snapshots,
        arena: Arc::new(host.game().arena().clone()),
        ruleset: Arc::new(host.game().ruleset().clone()),
    };
    tokio::spawn(run_match(host, pacing, inbox, publisher));
    client
}

async fn run_match(
    mut host: MatchHost,
    pacing: ServerPacing,
    mut inbox: mpsc::Receiver<SeatRequest>,
    publisher: watch::Sender<Arc<MatchSnapshot>>,
) {
    let mut ticker = tokio::time::interval(pacing.tick);
    let mut last_tick = Instant::now();
    let mut next_bot_move = Instant::now() + pacing.bot_delay;
    let mut bot_dice = SeededDice::from_seed(pacing.bot_seed);
    loop {
        tokio::select! {
            request = inbox.recv() => match request {
                Some(request) => {
                    let reply = answer(&mut host, request);
                    publisher.send_replace(Arc::new(host.snapshot()));
                    reply.deliver();
                    continue;
                }
                None => break,
            },
            _ = ticker.tick() => {
                let now = Instant::now();
                let elapsed = u64::try_from((now - last_tick).as_millis()).unwrap_or(u64::MAX);
                last_tick = now;
                host.advance_clock(elapsed);
                if now >= next_bot_move && play_bot_move(&mut host, pacing.bot, &mut bot_dice) {
                    next_bot_move = now + pacing.bot_delay;
                }
            }
        }
        publisher.send_replace(Arc::new(host.snapshot()));
    }
}

enum PendingReply {
    Submitted(
        oneshot::Sender<Result<Vec<GameEvent>, HostError>>,
        Result<Vec<GameEvent>, HostError>,
    ),
    Readied(
        oneshot::Sender<Result<(), HostError>>,
        Result<(), HostError>,
    ),
    Events(
        oneshot::Sender<(Vec<GameEvent>, usize)>,
        (Vec<GameEvent>, usize),
    ),
    Recorded(oneshot::Sender<MatchRecord>, MatchRecord),
}

impl PendingReply {
    fn deliver(self) {
        let _ = match self {
            PendingReply::Submitted(reply, outcome) => reply.send(outcome).map_err(drop),
            PendingReply::Readied(reply, outcome) => reply.send(outcome).map_err(drop),
            PendingReply::Events(reply, outcome) => reply.send(outcome).map_err(drop),
            PendingReply::Recorded(reply, record) => reply.send(record).map_err(drop),
        };
    }
}

fn answer(host: &mut MatchHost, request: SeatRequest) -> PendingReply {
    match request {
        SeatRequest::Submit {
            team,
            command,
            reply,
        } => PendingReply::Submitted(reply, host.submit(team, command)),
        SeatRequest::Ready { team, reply } => {
            PendingReply::Readied(reply, host.ready_or_running(team))
        }
        SeatRequest::EventsSince { cursor, reply } => PendingReply::Events(
            reply,
            (host.events_since(cursor).to_vec(), host.event_count()),
        ),
        SeatRequest::Record { reply } => PendingReply::Recorded(reply, host.record().clone()),
    }
}

fn play_bot_move(host: &mut MatchHost, bot: Bot, bot_dice: &mut SeededDice) -> bool {
    let snapshot = host.snapshot();
    let Some(team) = snapshot.state.active_team() else {
        return false;
    };
    if snapshot.seats.get(team.index()) != Some(&SeatKind::Bot) {
        return false;
    }
    let game = host.game();
    let command = bot.command(game.arena(), game.ruleset(), game.state(), team, bot_dice);
    host.submit(team, command).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use goblins_rules::event::TurnEndReason;
    use goblins_rules::host::{MatchSetup, TurnClock};
    use goblins_rules::roster::Roster;
    use goblins_rules::state::Phase;

    fn host(seats: [SeatKind; 2]) -> MatchHost {
        MatchHost::start(MatchSetup {
            ruleset: Ruleset::default(),
            rosters: [Roster::DEFAULT, Roster::DEFAULT],
            seed: 42,
            seats,
            turn_seconds: [60, 60],
            agent_ready_grace_seconds: 15,
        })
        .unwrap()
        .0
    }

    fn active_team(client: &MatchClient) -> TeamId {
        client.latest().state.active_team().unwrap()
    }

    #[tokio::test(start_paused = true)]
    async fn given_a_new_match_between_agents_when_reading_the_latest_snapshot_then_the_clock_waits_for_the_agent()
     {
        let client = spawn_match(
            host([SeatKind::Agent, SeatKind::Agent]),
            ServerPacing::default(),
        );
        assert_eq!(
            client.latest().clock,
            TurnClock::WaitingForReady {
                remaining_ms: 60_000
            }
        );
    }

    #[tokio::test(start_paused = true)]
    async fn given_an_agent_turn_when_it_ends_its_turn_then_the_other_team_plays() {
        let client = spawn_match(
            host([SeatKind::Agent, SeatKind::Agent]),
            ServerPacing::default(),
        );
        let team = active_team(&client);
        client.submit(team, Command::EndTurn).await.unwrap();
        let snapshot = client
            .wait_until(Duration::from_secs(1), |snapshot| snapshot.version == 1)
            .await;
        assert_eq!(snapshot.state.active_team(), Some(team.duel_opponent()));
    }

    #[tokio::test(start_paused = true)]
    async fn given_a_command_from_the_idle_team_when_submitted_then_refused() {
        let client = spawn_match(
            host([SeatKind::Agent, SeatKind::Agent]),
            ServerPacing::default(),
        );
        let idle = active_team(&client).duel_opponent();
        assert!(matches!(
            client.submit(idle, Command::EndTurn).await,
            Err(ClientError::Host(HostError::Rule(_)))
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn given_an_agent_that_never_looks_when_the_grace_and_a_minute_pass_then_its_turn_ends_on_time_up()
     {
        let client = spawn_match(
            host([SeatKind::Agent, SeatKind::Agent]),
            ServerPacing::default(),
        );
        let team = active_team(&client);
        tokio::time::sleep(Duration::from_millis(75_200)).await;
        let (events, _) = client.events_since(0).await.unwrap();
        assert!(events.contains(&GameEvent::TurnEnded {
            team,
            reason: TurnEndReason::TimeUp
        }));
    }

    #[tokio::test(start_paused = true)]
    async fn given_two_bots_when_time_passes_then_they_play_the_match_to_the_end() {
        let client = spawn_match(
            host([SeatKind::Bot, SeatKind::Bot]),
            ServerPacing::default(),
        );
        let snapshot = client
            .wait_until(Duration::from_secs(3_600), |snapshot| {
                snapshot.state.phase == Phase::Finished
            })
            .await;
        assert_eq!(snapshot.state.phase, Phase::Finished);
    }

    #[tokio::test(start_paused = true)]
    async fn given_a_waiting_human_when_ready_then_its_clock_runs() {
        let client = spawn_match(
            host([SeatKind::Human, SeatKind::Human]),
            ServerPacing::default(),
        );
        let team = active_team(&client);
        client.ready(team).await.unwrap();
        let snapshot = client
            .wait_until(Duration::from_secs(1), |snapshot| {
                matches!(snapshot.clock, TurnClock::Running { .. })
            })
            .await;
        assert!(matches!(snapshot.clock, TurnClock::Running { .. }));
    }

    #[tokio::test(start_paused = true)]
    async fn given_events_so_far_when_reading_from_the_end_then_nothing_new() {
        let client = spawn_match(
            host([SeatKind::Agent, SeatKind::Agent]),
            ServerPacing::default(),
        );
        let (_, end) = client.events_since(0).await.unwrap();
        let (newer, _) = client.events_since(end).await.unwrap();
        assert!(newer.is_empty());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn given_a_submitted_command_when_it_returns_then_the_latest_snapshot_already_shows_it() {
        let client = spawn_match(
            host([SeatKind::Agent, SeatKind::Agent]),
            ServerPacing::default(),
        );
        let mut versions_seen_late = 0;
        for expected_version in 1..=40 {
            let team = active_team(&client);
            client.submit(team, Command::EndTurn).await.unwrap();
            if client.latest().version != expected_version {
                versions_seen_late += 1;
            }
        }
        assert_eq!(versions_seen_late, 0);
    }

    #[tokio::test(start_paused = true)]
    async fn given_a_command_when_reading_the_record_then_it_is_recorded() {
        let client = spawn_match(
            host([SeatKind::Agent, SeatKind::Agent]),
            ServerPacing::default(),
        );
        let team = active_team(&client);
        client.submit(team, Command::EndTurn).await.unwrap();
        assert_eq!(
            client
                .record()
                .await
                .unwrap()
                .entries
                .last()
                .map(|entry| entry.command.clone()),
            Some(Command::EndTurn)
        );
    }

    #[tokio::test(start_paused = true)]
    async fn given_a_running_clock_when_its_seat_says_ready_again_then_accepted() {
        let client = spawn_match(
            host([SeatKind::Agent, SeatKind::Agent]),
            ServerPacing::default(),
        );
        let team = active_team(&client);
        client.ready(team).await.unwrap();
        assert_eq!(client.ready(team).await, Ok(()));
    }
}
