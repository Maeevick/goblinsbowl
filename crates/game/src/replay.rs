//! Replays a recorded match in the window, whoever coached it: its commands played one by one at the
//! pace picked, turn by turn back and forth, each order shown with the chance it had.

use std::collections::HashMap;
use std::time::Duration;

use bevy::prelude::*;
use goblins_rules::assessment::{activation_rows, safe_orders_played_late};
use goblins_rules::command::{Activation, Command};
use goblins_rules::event::GameEvent;
use goblins_rules::game::SetupError;
use goblins_rules::host::{MatchHost, MatchRecord, MatchSetup, SeatKind};
use goblins_rules::preview::{ActivationOdds, activation_odds};
use goblins_rules::roster::GoblinId;

use crate::goblin_view::{BowlAnimation, PlaybackSpeed};
use crate::local_match::{
    MatchJumped, MatchOrder, MatchRestarted, MatchSystems, MatchView, RestartableMatch,
};

/// Pause between two commands at normal speed, once the bowl has played out the last one.
const PAUSE_BETWEEN_COMMANDS: Duration = Duration::from_millis(800);

/// An order of the record as the replay shows it, read on the board it was played on.
#[derive(Clone, Debug, PartialEq)]
pub struct ReplayedOrder {
    /// The order.
    pub activation: Activation,
    /// Its chances on the board it was played on.
    pub odds: ActivationOdds,
    /// Other goblins of its team whose safe orders, played later in the turn, could have come first.
    pub safe_orders_played_late: Vec<GoblinId>,
}

/// Where a replay stands in its record.
#[derive(Clone, Debug)]
pub struct ReplayCursor {
    record: MatchRecord,
    host: MatchHost,
    next: usize,
    turn_starts: Vec<usize>,
    played_late: HashMap<usize, Vec<GoblinId>>,
}

impl ReplayCursor {
    /// The replay of `record` at its kick-off, and the events of the kick-off.
    ///
    /// # Errors
    /// [`SetupError`] when the record's setup cannot start a match.
    pub fn open(record: MatchRecord) -> Result<(Self, Vec<GameEvent>), SetupError> {
        let (host, opening) = kick_off(&record)?;
        let turn_starts = turn_starts(&record, host.clone());
        let played_late = activation_rows(&record)
            .map(|rows| safe_orders_played_late(&rows))
            .unwrap_or_default();
        Ok((
            Self {
                record,
                host,
                next: 0,
                turn_starts,
                played_late,
            },
            opening,
        ))
    }

    /// The match where the replay stands.
    pub fn host(&self) -> &MatchHost {
        &self.host
    }

    /// Whether every command of the record was played.
    pub fn is_over(&self) -> bool {
        self.next >= self.record.entries.len()
    }

    /// Plays the next command of the record: the order it was, if one, and what happened; none once
    /// the record is played out.
    pub fn play_next(&mut self) -> Option<(Option<ReplayedOrder>, Vec<GameEvent>)> {
        let entry = self.record.entries.get(self.next)?;
        let order = match &entry.command {
            Command::Act(activation) => {
                let game = self.host.game();
                activation_odds(
                    game.arena(),
                    game.ruleset(),
                    game.state(),
                    entry.team,
                    activation,
                )
                .ok()
                .map(|odds| ReplayedOrder {
                    activation: activation.clone(),
                    odds,
                    safe_orders_played_late: self
                        .played_late
                        .get(&self.next)
                        .cloned()
                        .unwrap_or_default(),
                })
            }
            _ => None,
        };
        let events = self.apply_next()?;
        Some((order, events))
    }

    /// Moves to the start of the next turn, with every event of the match until then; none once the
    /// last turn has started.
    pub fn jump_to_next_turn(&mut self) -> Option<Vec<GameEvent>> {
        let start = self
            .turn_starts
            .iter()
            .copied()
            .find(|start| *start > self.next)?;
        Some(self.jump_to(start))
    }

    /// Moves back to the start of the turn under way, or of the turn before when nothing of the turn
    /// under way was played yet, with every event of the match until then.
    pub fn jump_to_previous_turn(&mut self) -> Vec<GameEvent> {
        let current = self.turn_start_at_or_before(self.next);
        let target = if self.next > current {
            current
        } else {
            self.turn_start_at_or_before(current.saturating_sub(1))
        };
        self.jump_to(target)
    }

    /// Moves back to the kick-off, with its events.
    pub fn jump_to_kick_off(&mut self) -> Vec<GameEvent> {
        self.jump_to(0)
    }

    /// Moves to the end of the record, every command played, with every event of the match.
    pub fn jump_to_the_end(&mut self) -> Vec<GameEvent> {
        self.jump_to(self.record.entries.len())
    }

    fn turn_start_at_or_before(&self, entry: usize) -> usize {
        self.turn_starts
            .iter()
            .copied()
            .rev()
            .find(|start| *start <= entry)
            .unwrap_or(0)
    }

    fn jump_to(&mut self, entry: usize) -> Vec<GameEvent> {
        let Ok((host, mut story)) = kick_off(&self.record) else {
            return Vec::new();
        };
        self.host = host;
        self.next = 0;
        while self.next < entry {
            match self.apply_next() {
                Some(events) => story.extend(events),
                None => break,
            }
        }
        story
    }

    fn apply_next(&mut self) -> Option<Vec<GameEvent>> {
        let entry = self.record.entries.get(self.next)?;
        let events = self.host.submit(entry.team, entry.command.clone()).ok()?;
        self.next += 1;
        Some(events)
    }
}

/// The record's match at its kick-off, every seat left to the record.
fn kick_off(record: &MatchRecord) -> Result<(MatchHost, Vec<GameEvent>), SetupError> {
    MatchHost::start(MatchSetup {
        seats: [SeatKind::Bot, SeatKind::Bot],
        ..record.setup.clone()
    })
}

/// The entries of `record` that open a turn, played from `host` at its kick-off.
fn turn_starts(record: &MatchRecord, mut host: MatchHost) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut turn_before = None;
    for (index, entry) in record.entries.iter().enumerate() {
        let state = host.game().state();
        let turn = state.active_team().map(|team| {
            (
                state.half,
                team,
                state.turns_played.get(team.index()).copied(),
            )
        });
        if turn != turn_before {
            starts.push(index);
            turn_before = turn;
        }
        if host.submit(entry.team, entry.command.clone()).is_err() {
            break;
        }
    }
    starts
}

/// A jump the replay can make.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Jump {
    /// Back to the start of the turn under way, or of the one before.
    PreviousTurn,
    /// On to the start of the next turn, or to the end of the record during the last one.
    NextTurn,
    /// Back to the kick-off.
    KickOff,
}

/// The replay under way: where it stands, whether it plays on and how fast.
#[derive(Resource)]
pub struct Replay {
    cursor: ReplayCursor,
    playing: bool,
    speed: u8,
    pause: Timer,
    last_order: Option<ReplayedOrder>,
    opening: Vec<GameEvent>,
}

impl Replay {
    /// Whether it plays on by itself.
    pub fn is_playing(&self) -> bool {
        self.playing
    }

    /// How many times faster than a match it plays: 1, 2 or 3.
    pub fn speed(&self) -> u8 {
        self.speed
    }

    /// The order played last, if the last command was one.
    pub fn last_order(&self) -> Option<&ReplayedOrder> {
        self.last_order.as_ref()
    }

    /// Plays on, or pauses; a replay played out starts again from its kick-off.
    pub fn toggle_playing(&mut self) -> Option<Vec<GameEvent>> {
        self.playing = !self.playing;
        (self.playing && self.cursor.is_over()).then(|| self.jump(Jump::KickOff))
    }

    /// Plays `speed` times faster than a match, between 1 and 3.
    pub fn set_speed(&mut self, speed: u8) {
        self.speed = speed.clamp(1, 3);
        self.pause
            .set_duration(PAUSE_BETWEEN_COMMANDS / u32::from(self.speed));
    }

    /// Makes `jump`, with every event of the match until where it lands.
    pub fn jump(&mut self, jump: Jump) -> Vec<GameEvent> {
        self.last_order = None;
        self.pause.reset();
        match jump {
            Jump::PreviousTurn => self.cursor.jump_to_previous_turn(),
            Jump::NextTurn => match self.cursor.jump_to_next_turn() {
                Some(story) => story,
                None => self.cursor.jump_to_the_end(),
            },
            Jump::KickOff => self.cursor.jump_to_kick_off(),
        }
    }
}

/// Replays a record in the window instead of hosting a match.
pub struct ReplayPlugin {
    /// The replay at its kick-off.
    pub cursor: ReplayCursor,
    /// The events of its kick-off, shown on the first frame.
    pub opening_events: Vec<GameEvent>,
}

impl Plugin for ReplayPlugin {
    fn build(&self, app: &mut App) {
        let host = self.cursor.host();
        let view = MatchView {
            snapshot: host.snapshot(),
            arena: host.game().arena().clone(),
            ruleset: host.game().ruleset().clone(),
            fresh_events: Vec::new(),
            refusal: None,
        };
        app.add_message::<MatchOrder>()
            .add_message::<MatchRestarted>()
            .add_message::<MatchJumped>()
            .init_resource::<RestartableMatch>()
            .init_resource::<PlaybackSpeed>()
            .insert_resource(view)
            .insert_resource(Replay {
                cursor: self.cursor.clone(),
                playing: true,
                speed: 1,
                pause: Timer::new(PAUSE_BETWEEN_COMMANDS, TimerMode::Repeating),
                last_order: None,
                opening: self.opening_events.clone(),
            })
            .configure_sets(Update, (MatchSystems::Apply, MatchSystems::Present).chain())
            .add_systems(Update, play_the_record.in_set(MatchSystems::Apply));
    }
}

fn play_the_record(
    time: Res<Time>,
    animation: Option<Res<BowlAnimation>>,
    mut replay: ResMut<Replay>,
    mut view: ResMut<MatchView>,
    mut playback: ResMut<PlaybackSpeed>,
    mut orders: MessageReader<MatchOrder>,
    mut jumps: MessageWriter<MatchJumped>,
) {
    view.refusal = None;
    view.fresh_events = std::mem::take(&mut replay.opening);
    if orders.read().any(|order| *order == MatchOrder::Restart) {
        let story = replay.jump(Jump::KickOff);
        replay.playing = true;
        jumps.write(MatchJumped { story });
    }
    playback.0 = f32::from(replay.speed);
    let bowl_busy = animation.is_some_and(|animation| animation.busy);
    if !replay.playing || bowl_busy {
        replay.pause.reset();
    } else if replay.pause.tick(time.delta()).just_finished() {
        match replay.cursor.play_next() {
            Some((order, events)) => {
                replay.last_order = order;
                view.fresh_events.extend(events);
            }
            None => replay.playing = false,
        }
    }
    view.snapshot = replay.cursor.host().snapshot();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::written;
    use bevy::time::TimeUpdateStrategy;
    use goblins_rules::bot_match::play_bot_match;
    use goblins_rules::policies::Bot;
    use goblins_rules::ruleset::Ruleset;
    use goblins_rules::state::GameState;

    fn short_record() -> MatchRecord {
        let ruleset = Ruleset {
            turns_per_half: 3,
            halves: 1,
            ..Ruleset::default()
        };
        play_bot_match(
            &ruleset,
            [ruleset.default_roster; 2],
            4,
            [Bot::Cautious2, Bot::Bold2],
        )
        .unwrap()
        .0
    }

    fn opened() -> ReplayCursor {
        ReplayCursor::open(short_record()).unwrap().0
    }

    fn state(cursor: &ReplayCursor) -> GameState {
        cursor.host().snapshot().state
    }

    fn played_until_the_turn_changes(mut cursor: ReplayCursor) -> GameState {
        let turn_before = state(&cursor).turns_played.clone();
        while state(&cursor).turns_played == turn_before {
            cursor.play_next().unwrap();
        }
        state(&cursor)
    }

    #[test]
    fn given_a_replay_at_its_kick_off_when_every_command_is_played_then_it_ends_where_the_record_ends()
     {
        let record = short_record();
        let mut cursor = ReplayCursor::open(record.clone()).unwrap().0;
        while cursor.play_next().is_some() {}
        assert_eq!(state(&cursor), *record.replay().unwrap().state());
    }

    #[test]
    fn given_a_replay_at_its_kick_off_when_jumping_to_the_next_turn_then_it_stands_where_playing_on_would_get()
     {
        let mut cursor = opened();
        let played_on = played_until_the_turn_changes(cursor.clone());
        cursor.jump_to_next_turn().unwrap();
        assert_eq!(state(&cursor), played_on);
    }

    #[test]
    fn given_a_replay_inside_a_turn_when_jumping_back_then_it_stands_at_that_turn_s_start() {
        let mut cursor = opened();
        cursor.jump_to_next_turn().unwrap();
        let turn_start = state(&cursor);
        cursor.play_next().unwrap();
        cursor.jump_to_previous_turn();
        assert_eq!(state(&cursor), turn_start);
    }

    #[test]
    fn given_a_replay_at_a_turn_s_start_when_jumping_back_then_it_stands_at_the_turn_before() {
        let mut cursor = opened();
        let kick_off = state(&cursor);
        cursor.jump_to_next_turn().unwrap();
        cursor.jump_to_previous_turn();
        assert_eq!(state(&cursor), kick_off);
    }

    #[test]
    fn given_the_last_turn_under_way_when_jumping_to_the_next_turn_then_there_is_none() {
        let mut cursor = opened();
        while cursor.jump_to_next_turn().is_some() {}
        assert_eq!(cursor.jump_to_next_turn(), None);
    }

    #[test]
    fn given_the_last_turn_under_way_when_the_replay_jumps_on_then_it_stands_at_the_record_s_end() {
        let record = short_record();
        let mut app = replay_app();
        let mut replay = app.world_mut().resource_mut::<Replay>();
        while replay.cursor.jump_to_next_turn().is_some() {}
        replay.jump(Jump::NextTurn);
        assert_eq!(
            (replay.cursor.is_over(), state(&replay.cursor)),
            (true, record.replay().unwrap().state().clone())
        );
    }

    #[test]
    fn given_an_order_of_the_record_when_played_then_it_comes_with_the_chance_the_assessment_gives_it()
     {
        let record = short_record();
        let first_order = activation_rows(&record).unwrap().remove(0);
        let mut cursor = ReplayCursor::open(record).unwrap().0;
        let mut replayed = None;
        while replayed.is_none() {
            replayed = cursor.play_next().unwrap().0;
        }
        assert_eq!(
            replayed.map(|order| order.odds.overall_success),
            Some(first_order.success)
        );
    }

    fn replay_app() -> App {
        let (cursor, opening_events) = ReplayCursor::open(short_record()).unwrap();
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            ReplayPlugin {
                cursor,
                opening_events,
            },
        ))
        .init_resource::<BowlAnimation>()
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            300,
        )));
        app.update();
        app
    }

    fn version(app: &App) -> u64 {
        app.world().resource::<MatchView>().snapshot.version
    }

    #[test]
    fn given_a_replay_playing_when_its_pause_passes_then_the_next_command_is_played() {
        let mut app = replay_app();
        for _ in 0..4 {
            app.update();
        }
        assert!(version(&app) > 0);
    }

    #[test]
    fn given_a_paused_replay_when_time_passes_then_nothing_is_played() {
        let mut app = replay_app();
        app.world_mut().resource_mut::<Replay>().toggle_playing();
        for _ in 0..8 {
            app.update();
        }
        assert_eq!(version(&app), 0);
    }

    #[test]
    fn given_a_replay_at_three_times_the_speed_when_the_frame_runs_then_the_bowl_plays_three_times_faster()
     {
        let mut app = replay_app();
        app.world_mut().resource_mut::<Replay>().set_speed(3);
        app.update();
        assert_eq!(app.world().resource::<PlaybackSpeed>().0, 3.0);
    }

    #[test]
    fn given_a_replay_under_way_when_played_again_then_it_is_back_at_its_kick_off() {
        let mut app = replay_app();
        for _ in 0..8 {
            app.update();
        }
        app.world_mut().write_message(MatchOrder::Restart);
        app.update();
        assert_eq!((version(&app), written::<MatchJumped>(&app).len()), (0, 1));
    }
}
