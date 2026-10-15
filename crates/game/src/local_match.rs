//! The match running inside the app for hot-seat play: the host as a resource, its clock fed by Bevy time.
//!
//! Screens only read [`MatchView`] and send [`MatchOrder`] messages, so the source of the match can
//! later be swapped for an embedded server without touching them.

use std::path::PathBuf;
use std::time::Duration;

use bevy::prelude::*;
use goblins_rules::arena::Arena;
use goblins_rules::command::Command;
use goblins_rules::event::GameEvent;
use goblins_rules::host::{MatchHost, MatchRecord, MatchSnapshot};
use goblins_rules::ruleset::Ruleset;
use goblins_rules::state::Phase;
use goblins_rules::team::TeamId;
use goblins_server::runs::{record_file, write_record};

/// What screens read: the latest snapshot and what happened during this frame.
#[derive(Resource, Clone, Debug)]
pub struct MatchView {
    /// The latest snapshot of the match.
    pub snapshot: MatchSnapshot,
    /// The arena of the match.
    pub arena: Arena,
    /// The rules of the match.
    pub ruleset: Ruleset,
    /// Events produced during this frame, in order.
    pub fresh_events: Vec<GameEvent>,
    /// Why the last order of this frame was refused, if it was.
    pub refusal: Option<String>,
}

/// A request from a screen or a bot to the match.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub enum MatchOrder {
    /// Apply a command for a team.
    Command {
        /// The team acting.
        team: TeamId,
        /// What it does.
        command: Command,
    },
    /// A human team pressed Ready.
    Ready {
        /// The team.
        team: TeamId,
    },
    /// Freeze or unfreeze the clock (matches between humans only).
    TogglePause,
    /// Start a new match with the same teams and seats, and the next dice seed.
    Restart,
}

/// A new match replaced the one that was played: screens forget what they kept of it.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct MatchRestarted;

/// Present when the match runs inside the app and can be started again.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct RestartableMatch;

/// Ordering of the match systems within a frame: orders are applied before screens present them.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MatchSystems {
    /// Orders, bots and the clock change the match.
    Apply,
    /// Screens read the result.
    Present,
}

#[derive(Resource)]
struct LocalMatch {
    host: MatchHost,
    unspent_time: Duration,
    clock_ran_before_orders: bool,
}

/// The record of the match being played, saved to a file named after its seed.
#[derive(Resource)]
struct Recording {
    folder: PathBuf,
    file: PathBuf,
    saved: bool,
}

impl Recording {
    fn of_match(folder: PathBuf, seed: u64) -> Self {
        let file = record_file(&folder, seed);
        Self {
            folder,
            file,
            saved: false,
        }
    }

    fn save_unless_saved(&mut self, record: &MatchRecord, moment: &str) {
        if self.saved {
            return;
        }
        match write_record(record, &self.file) {
            Ok(()) => {
                self.saved = true;
                println!("{moment}: record saved to {}", self.file.display());
            }
            Err(error) => eprintln!("{moment}: the record could not be saved: {error:#}"),
        }
    }

    fn start_over(&mut self, seed: u64) {
        self.file = record_file(&self.folder, seed);
        self.saved = false;
    }
}

/// Runs a match hosted inside the app.
pub struct LocalMatchPlugin {
    /// The started match.
    pub host: MatchHost,
    /// Events of its opening (coin toss, kick-off), shown on the first frame.
    pub opening_events: Vec<GameEvent>,
    /// Folder its record is saved to at the final whistle, on a restart and when the app closes;
    /// nothing is saved without one.
    pub record_to: Option<PathBuf>,
}

impl Plugin for LocalMatchPlugin {
    fn build(&self, app: &mut App) {
        let host = self.host.clone();
        let view = MatchView {
            snapshot: host.snapshot(),
            arena: host.game().arena().clone(),
            ruleset: host.game().ruleset().clone(),
            fresh_events: Vec::new(),
            refusal: None,
        };
        app.add_message::<MatchOrder>()
            .add_message::<MatchRestarted>()
            .init_resource::<RestartableMatch>()
            .insert_resource(view)
            .insert_resource(LocalMatch {
                host,
                unspent_time: Duration::ZERO,
                clock_ran_before_orders: false,
            })
            .insert_resource(OpeningEvents(self.opening_events.clone()))
            .configure_sets(Update, (MatchSystems::Apply, MatchSystems::Present).chain())
            .add_systems(
                Update,
                (apply_orders, run_clock, refresh_view, save_at_final_whistle)
                    .chain()
                    .in_set(MatchSystems::Apply),
            )
            .add_systems(Last, save_on_exit);
        if let Some(folder) = &self.record_to {
            app.insert_resource(Recording::of_match(
                folder.clone(),
                self.host.record().setup.seed,
            ));
        }
    }
}

#[derive(Resource)]
struct OpeningEvents(Vec<GameEvent>);

fn apply_orders(
    mut orders: MessageReader<MatchOrder>,
    mut local: ResMut<LocalMatch>,
    mut view: ResMut<MatchView>,
    opening: Option<ResMut<OpeningEvents>>,
    mut restarts: MessageWriter<MatchRestarted>,
    mut recording: Option<ResMut<Recording>>,
) {
    view.fresh_events.clear();
    view.refusal = None;
    local.clock_ran_before_orders = matches!(
        local.host.snapshot().clock,
        goblins_rules::host::TurnClock::Running { .. }
    );
    if let Some(mut opening) = opening {
        view.fresh_events.append(&mut opening.0);
    }
    for order in orders.read() {
        let outcome = match order {
            MatchOrder::Command { team, command } => local
                .host
                .submit(*team, command.clone())
                .map(|events| view.fresh_events.extend(events)),
            MatchOrder::Ready { team } => local.host.ready(*team),
            MatchOrder::TogglePause => {
                if matches!(
                    local.host.snapshot().clock,
                    goblins_rules::host::TurnClock::Paused { .. }
                ) {
                    local.host.resume();
                    Ok(())
                } else {
                    local.host.pause()
                }
            }
            MatchOrder::Restart => {
                if let Some(recording) = recording.as_mut() {
                    recording.save_unless_saved(local.host.record(), "Restarted");
                }
                let mut setup = local.host.record().setup.clone();
                setup.seed = setup.seed.wrapping_add(1);
                if let Some(recording) = recording.as_mut() {
                    recording.start_over(setup.seed);
                }
                if let Ok((host, opening_events)) = MatchHost::start(setup) {
                    local.host = host;
                    local.clock_ran_before_orders = false;
                    local.unspent_time = Duration::ZERO;
                    view.fresh_events = opening_events;
                    restarts.write(MatchRestarted);
                }
                Ok(())
            }
        };
        if let Err(refusal) = outcome {
            view.refusal = Some(refusal.to_string());
        }
    }
}

fn run_clock(time: Res<Time>, mut local: ResMut<LocalMatch>, mut view: ResMut<MatchView>) {
    if !local.clock_ran_before_orders {
        local.unspent_time = Duration::ZERO;
        return;
    }
    local.unspent_time += time.delta();
    let whole_milliseconds = local.unspent_time.as_millis();
    local.unspent_time -=
        Duration::from_millis(u64::try_from(whole_milliseconds).unwrap_or(u64::MAX));
    let events = local
        .host
        .advance_clock(u64::try_from(whole_milliseconds).unwrap_or(u64::MAX));
    view.fresh_events.extend(events);
}

fn refresh_view(local: Res<LocalMatch>, mut view: ResMut<MatchView>) {
    view.snapshot = local.host.snapshot();
}

fn save_at_final_whistle(
    local: Res<LocalMatch>,
    view: Res<MatchView>,
    recording: Option<ResMut<Recording>>,
) {
    if let Some(mut recording) = recording
        && view.snapshot.state.phase == Phase::Finished
    {
        recording.save_unless_saved(local.host.record(), "Final whistle");
    }
}

fn save_on_exit(
    mut exits: MessageReader<AppExit>,
    local: Res<LocalMatch>,
    recording: Option<ResMut<Recording>>,
) {
    if exits.read().next().is_some()
        && let Some(mut recording) = recording
    {
        recording.save_unless_saved(local.host.record(), "Stopped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::time::TimeUpdateStrategy;
    use goblins_rules::host::{MatchSetup, SeatKind, TurnClock};
    use goblins_rules::roster::Roster;

    fn app_with(seats: [SeatKind; 2]) -> App {
        let (host, opening_events) = MatchHost::start(MatchSetup {
            ruleset: Ruleset::default(),
            rosters: [Roster::DEFAULT, Roster::DEFAULT],
            seed: 42,
            seats,
            turn_seconds: [60, 60],
            agent_ready_grace_seconds: 15,
        })
        .unwrap();
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            LocalMatchPlugin {
                host,
                opening_events,
                record_to: None,
            },
        ))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            250,
        )));
        app.update();
        app
    }

    fn view(app: &App) -> &MatchView {
        app.world().resource::<MatchView>()
    }

    fn recording_app(folder_name: &str, turns_per_half: u8) -> (App, PathBuf) {
        let folder = std::env::temp_dir().join(format!(
            "goblinsbowl-local-record-{folder_name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&folder);
        let (host, opening_events) = MatchHost::start(MatchSetup {
            ruleset: Ruleset {
                turns_per_half,
                halves: 1,
                ..Ruleset::default()
            },
            rosters: [Roster::DEFAULT, Roster::DEFAULT],
            seed: 42,
            seats: [SeatKind::Human, SeatKind::Bot],
            turn_seconds: [60, 60],
            agent_ready_grace_seconds: 15,
        })
        .unwrap();
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            LocalMatchPlugin {
                host,
                opening_events,
                record_to: Some(folder.clone()),
            },
        ));
        app.update();
        (app, folder)
    }

    fn saved_records(folder: &std::path::Path) -> Vec<MatchRecord> {
        std::fs::read_dir(folder)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
                    .filter_map(|text| MatchRecord::from_json_lines(&text).ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn end_the_turn(app: &mut App) {
        let team = active_team(app);
        app.world_mut().write_message(MatchOrder::Command {
            team,
            command: Command::EndTurn,
        });
        app.update();
    }

    #[test]
    fn given_a_recorded_match_when_the_final_whistle_goes_then_its_record_is_saved() {
        let (mut app, folder) = recording_app("final-whistle", 1);

        end_the_turn(&mut app);
        end_the_turn(&mut app);

        let lengths: Vec<usize> = saved_records(&folder)
            .iter()
            .map(|record| record.entries.len())
            .collect();
        assert_eq!(lengths, vec![2]);
    }

    #[test]
    fn given_a_recorded_match_under_way_when_the_app_closes_then_what_was_played_is_saved() {
        let (mut app, folder) = recording_app("exit", 12);
        end_the_turn(&mut app);

        app.world_mut().write_message(AppExit::Success);
        app.update();

        let lengths: Vec<usize> = saved_records(&folder)
            .iter()
            .map(|record| record.entries.len())
            .collect();
        assert_eq!(lengths, vec![1]);
    }

    #[test]
    fn given_a_recorded_match_under_way_when_restarted_then_the_stopped_match_is_saved() {
        let (mut app, folder) = recording_app("restart", 12);
        end_the_turn(&mut app);

        app.world_mut().write_message(MatchOrder::Restart);
        app.update();

        let seeds: Vec<u64> = saved_records(&folder)
            .iter()
            .map(|record| record.setup.seed)
            .collect();
        assert_eq!(seeds, vec![42]);
    }

    fn active_team(app: &App) -> TeamId {
        view(app).snapshot.state.active_team().unwrap()
    }

    #[test]
    fn given_a_finished_match_when_restarted_then_a_new_match_kicks_off_with_the_next_seed() {
        let mut app = app_with([SeatKind::Human, SeatKind::Human]);
        app.world_mut().write_message(MatchOrder::Restart);

        app.update();

        let local = app.world().resource::<LocalMatch>();
        assert_eq!(
            (
                local.host.record().setup.seed,
                local.host.record().entries.len()
            ),
            (43, 0)
        );
    }

    #[test]
    fn given_a_restart_when_the_frame_runs_then_the_new_opening_events_are_fresh() {
        let mut app = app_with([SeatKind::Human, SeatKind::Human]);
        app.world_mut().write_message(MatchOrder::Restart);

        app.update();

        assert!(matches!(
            view(&app).fresh_events.first(),
            Some(GameEvent::CoinTossed { .. })
        ));
    }

    #[test]
    fn given_a_new_match_when_the_first_frame_runs_then_the_opening_events_are_fresh() {
        let app = app_with([SeatKind::Human, SeatKind::Human]);
        assert!(matches!(
            view(&app).fresh_events.first(),
            Some(GameEvent::CoinTossed { .. })
        ));
    }

    #[test]
    fn given_an_end_turn_order_when_the_frame_runs_then_the_other_team_plays() {
        let mut app = app_with([SeatKind::Human, SeatKind::Human]);
        let team = active_team(&app);
        app.world_mut().write_message(MatchOrder::Command {
            team,
            command: Command::EndTurn,
        });
        app.update();
        assert_eq!(
            view(&app).snapshot.state.active_team(),
            Some(team.duel_opponent())
        );
    }

    #[test]
    fn given_an_order_from_the_idle_team_when_the_frame_runs_then_the_refusal_is_shown() {
        let mut app = app_with([SeatKind::Human, SeatKind::Human]);
        let idle = active_team(&app).duel_opponent();
        app.world_mut().write_message(MatchOrder::Command {
            team: idle,
            command: Command::EndTurn,
        });
        app.update();
        assert!(view(&app).refusal.is_some());
    }

    #[test]
    fn given_humans_when_frames_pass_before_ready_then_the_clock_still_waits() {
        let mut app = app_with([SeatKind::Human, SeatKind::Human]);
        app.update();
        app.update();
        assert_eq!(
            view(&app).snapshot.clock,
            TurnClock::WaitingForReady {
                remaining_ms: 60_000
            }
        );
    }

    #[test]
    fn given_a_ready_human_when_four_quarter_seconds_pass_then_the_clock_lost_a_second() {
        let mut app = app_with([SeatKind::Human, SeatKind::Human]);
        let team = active_team(&app);
        app.world_mut().write_message(MatchOrder::Ready { team });
        app.update();
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(
            view(&app).snapshot.clock,
            TurnClock::Running {
                remaining_ms: 59_000
            }
        );
    }

    #[test]
    fn given_bots_when_frames_pass_then_their_clock_runs() {
        let mut app = app_with([SeatKind::Bot, SeatKind::Bot]);
        app.update();
        assert!(matches!(
            view(&app).snapshot.clock,
            TurnClock::Running { .. }
        ));
    }
}
