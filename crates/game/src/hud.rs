//! Heads-up display: teams and score, turn and clock, the ready screen, buttons, and the event log.

use std::collections::VecDeque;

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::Pressed;
use bevy::ui_widgets::{Activate, Button, observe};
use goblins_rules::command::Command;
use goblins_rules::event::GameEvent;
use goblins_rules::host::{MatchSnapshot, SeatKind, TurnClock};
use goblins_rules::state::{GameState, Goblin, Phase};
use goblins_rules::team::TeamId;

use crate::camera_rig::CameraRig;
use crate::goblin_view::BowlAnimation;
use crate::local_match::{MatchOrder, MatchRestarted, MatchSystems, MatchView};
use crate::texts::Texts;

pub(crate) const READING_FONT: &str = "fonts/Andika-Regular.ttf";
pub(crate) const BOLD_FONT: &str = "fonts/Andika-Bold.ttf";
const BUTTON_IDLE: Color = Color::srgb(0.2, 0.17, 0.12);
const BUTTON_HOVERED: Color = Color::srgb(0.32, 0.26, 0.16);
const BUTTON_PRESSED: Color = Color::srgb(0.55, 0.42, 0.15);
const HOME_COLOUR: Color = Color::srgb(0.45, 0.65, 1.0);
const AWAY_COLOUR: Color = Color::srgb(1.0, 0.82, 0.35);
const URGENT_SECONDS_MS: u64 = 10_000;
const LOG_LINES: usize = 7;
const END_TURN_CONFIRM_SECONDS: f32 = 3.0;
const BUTTON_BORDER: Color = Color::srgb(0.95, 0.8, 0.4);
const PULSE_PER_SECOND: f32 = 5.0;
const INTERLUDE_SECONDS: f32 = 2.5;
const LOG_COLOUR: Color = Color::srgb(0.92, 0.92, 0.88);

/// A line of the match story, and the team it is about, if any.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoryLine {
    /// What happened.
    pub text: String,
    /// The team that did it, which colours the line.
    pub team: Option<TeamId>,
}

/// The team an event is about: the one acting, scoring or starting its turn.
pub fn event_team(event: &GameEvent) -> Option<TeamId> {
    match event {
        GameEvent::CoinTossed { receiving_team } | GameEvent::KickedOff { receiving_team } => {
            Some(*receiving_team)
        }
        GameEvent::TurnStarted { team, .. }
        | GameEvent::TurnEnded { team, .. }
        | GameEvent::Scored { team, .. } => Some(*team),
        GameEvent::ReturnedFromKnockOut { goblin, .. }
        | GameEvent::StillKnockedOut { goblin }
        | GameEvent::CatchAttempted { goblin, .. }
        | GameEvent::StoodUp { goblin }
        | GameEvent::Moved { goblin, .. }
        | GameEvent::DodgeAttempted { goblin, .. }
        | GameEvent::PickAttempted { goblin, .. }
        | GameEvent::InterceptAttempted { goblin, .. }
        | GameEvent::StanceTaken { goblin }
        | GameEvent::FocusTaken { goblin }
        | GameEvent::KnockedDown { goblin, .. }
        | GameEvent::Projected { goblin, .. }
        | GameEvent::HealthLost { goblin, .. }
        | GameEvent::KnockedOut { goblin } => Some(goblin.team),
        GameEvent::ThrowAttempted { thrower, .. } => Some(thrower.team),
        GameEvent::HandedOff { giver, .. } => Some(giver.team),
        GameEvent::ChaosThrown { thrower, .. } => Some(thrower.team),
        GameEvent::AssaultResolved { attacker, .. } | GameEvent::StandOff { attacker, .. } => {
            Some(attacker.team)
        }
        GameEvent::BallDropped { .. }
        | GameEvent::BallBounced { .. }
        | GameEvent::HalfEnded { .. }
        | GameEvent::GameEnded { .. } => None,
    }
}

/// Colour of a story line: the team's colour, or a soft white for the match itself.
pub fn story_colour(team: Option<TeamId>) -> Color {
    match team {
        Some(TeamId::HOME) => HOME_COLOUR,
        Some(_) => AWAY_COLOUR,
        None => LOG_COLOUR,
    }
}

/// The last lines of the match story.
#[derive(Resource, Default, Clone, Debug, PartialEq, Eq)]
pub struct EventLog {
    lines: VecDeque<StoryLine>,
}

impl EventLog {
    /// Adds lines at the bottom, keeping only the most recent ones.
    pub fn push_lines(&mut self, lines: impl IntoIterator<Item = StoryLine>) {
        self.lines.extend(lines);
        while self.lines.len() > LOG_LINES {
            self.lines.pop_front();
        }
    }

    /// The lines, oldest first.
    pub fn lines(&self) -> impl Iterator<Item = &StoryLine> {
        self.lines.iter()
    }
}

/// Colour of the countdown: red when time is running out.
pub fn clock_colour(remaining_ms: u64) -> Color {
    if remaining_ms < URGENT_SECONDS_MS {
        Color::srgb(1.0, 0.35, 0.3)
    } else {
        Color::WHITE
    }
}

/// The human team whose turn waits for it to say it is ready, if any.
pub fn team_waiting_for_ready(snapshot: &MatchSnapshot) -> Option<TeamId> {
    let team = human_team_in_control(snapshot)?;
    matches!(snapshot.clock, TurnClock::WaitingForReady { .. }).then_some(team)
}

/// The team a human may give orders for right now, if any.
pub fn human_team_in_control(snapshot: &MatchSnapshot) -> Option<TeamId> {
    let team = snapshot.state.active_team()?;
    (snapshot.seats.get(team.index()) == Some(&SeatKind::Human)).then_some(team)
}

/// A human team's turn waiting to start.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WaitingTurn {
    /// The team.
    pub team: TeamId,
    /// The half.
    pub half: u8,
    /// Its turn number in the half.
    pub turn: u8,
}

/// The human team's turn waiting for its Ready, if any.
pub fn waiting_turn(snapshot: &MatchSnapshot) -> Option<WaitingTurn> {
    let team = team_waiting_for_ready(snapshot)?;
    Some(WaitingTurn {
        team,
        half: snapshot.state.half,
        turn: snapshot
            .state
            .turns_played
            .get(team.index())
            .copied()
            .unwrap_or_default(),
    })
}

/// What the hand-over between two turns shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandoverStage {
    /// No human team waits for its turn.
    NoHandover,
    /// The bowl still plays out the last moves, left in full view.
    WatchingTheBowl,
    /// The next team is announced with a goblin aside, before anyone can press Ready.
    Interlude,
    /// The next team is asked whether it is ready.
    AskingForReady,
}

/// The hand-over between two turns: which turn waits and how long the bowl has been still since.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct Handover {
    /// The turn waiting for its Ready.
    pub waiting: Option<WaitingTurn>,
    /// Seconds the bowl has been still while that turn waits.
    pub still_for: f32,
}

impl Handover {
    /// The hand-over `seconds` later, with `waiting` the turn now waiting and `animating` whether
    /// the bowl still plays out the last moves.
    pub fn after(self, waiting: Option<WaitingTurn>, animating: bool, seconds: f32) -> Self {
        let still_for = if waiting.is_none() || animating {
            0.0
        } else if self.waiting == waiting {
            self.still_for + seconds
        } else {
            seconds
        };
        Self { waiting, still_for }
    }

    /// What the hand-over shows now.
    pub fn stage(&self) -> HandoverStage {
        match self.waiting {
            None => HandoverStage::NoHandover,
            Some(_) if self.still_for <= 0.0 => HandoverStage::WatchingTheBowl,
            Some(_) if self.still_for < INTERLUDE_SECONDS => HandoverStage::Interlude,
            Some(_) => HandoverStage::AskingForReady,
        }
    }
}

/// Whether every goblin of `team` is done for the turn, so only ending it is left.
pub fn every_goblin_done(state: &GameState, team: TeamId) -> bool {
    !state.goblins_of(team).any(Goblin::can_act)
}

/// The human team playing with its clock running that has nothing left to play, if any.
pub fn team_with_nothing_left_to_play(snapshot: &MatchSnapshot) -> Option<TeamId> {
    let team = human_team_in_control(snapshot)?;
    let running = matches!(snapshot.clock, TurnClock::Running { .. });
    (running && every_goblin_done(&snapshot.state, team)).then_some(team)
}

#[derive(Component)]
struct HomeScore;
#[derive(Component)]
struct AwayScore;
#[derive(Component)]
struct TurnLine;
#[derive(Component)]
struct ClockText;
#[derive(Component)]
struct LogText;
#[derive(Component)]
struct LogSpan(usize);
#[derive(Component)]
struct HeadlineBanner;
#[derive(Component)]
struct HeadlineText;
#[derive(Component)]
struct ReadyOverlay;
#[derive(Component)]
struct ReadyQuestion;
#[derive(Component)]
struct RefusalText;
#[derive(Component)]
struct EndTurnLabel;
#[derive(Component)]
struct EndTurnButton;
#[derive(Component)]
struct ReadyLabel;
#[derive(Component)]
struct ReadyButton;
#[derive(Component)]
struct ReadyQuip;
#[derive(Component)]
struct NothingLeftNotice;

type ClockLabel<'w, 's> = Single<
    'w,
    's,
    (&'static mut Text, &'static mut TextColor),
    (With<ClockText>, Without<TurnLine>),
>;
type ReadyVisibility<'w, 's> =
    Single<'w, 's, &'static mut Visibility, (With<ReadyOverlay>, Without<ReadyButton>)>;
type ReadyButtonVisibility<'w, 's> =
    Single<'w, 's, &'static mut Visibility, (With<ReadyButton>, Without<ReadyOverlay>)>;

#[derive(Resource, Default)]
struct EndTurnConfirmation {
    seconds_left: f32,
}

/// Shows the HUD and keeps it in sync with the match.
pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<MatchRestarted>()
            .init_resource::<EventLog>()
            .init_resource::<EndTurnConfirmation>()
            .init_resource::<Handover>()
            .add_systems(Startup, spawn_hud)
            .add_systems(
                Update,
                (
                    pause_on_key
                        .in_set(MatchSystems::Apply)
                        .before(MatchSystems::Present),
                    (
                        forget_the_last_match,
                        collect_log_lines,
                        refresh_scores,
                        refresh_turn_and_clock,
                        refresh_log,
                        refresh_refusal,
                        refresh_headline,
                        follow_handover,
                        refresh_ready_overlay,
                        refresh_end_turn_label,
                        show_end_turn_to_humans,
                        restyle_buttons,
                        show_nothing_left_to_play,
                    )
                        .in_set(MatchSystems::Present),
                ),
            );
    }
}

pub(crate) fn text_bundle(
    text: impl Into<String>,
    font: &Handle<Font>,
    size: f32,
    colour: Color,
) -> impl Bundle {
    (
        Text::new(text),
        TextFont {
            font: font.clone().into(),
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(colour),
        TextShadow::default(),
    )
}

fn spawn_hud(mut commands: Commands, assets: Res<AssetServer>, texts: Res<Texts>) {
    let reading: Handle<Font> = assets.load(READING_FONT);
    let bold: Handle<Font> = assets.load(BOLD_FONT);
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: px(10),
            left: px(16),
            right: px(16),
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Start,
            ..default()
        },
        Pickable::IGNORE,
        children![
            (
                text_bundle(texts.team_name(TeamId::HOME), &bold, 30.0, HOME_COLOUR),
                HomeScore
            ),
            (
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    ..default()
                },
                children![
                    (text_bundle("", &reading, 22.0, Color::WHITE), TurnLine),
                    (text_bundle("", &bold, 48.0, Color::WHITE), ClockText),
                ],
            ),
            (
                text_bundle(texts.team_name(TeamId::AWAY), &bold, 30.0, AWAY_COLOUR),
                AwayScore
            ),
        ],
    ));
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(16),
                top: px(110),
                width: px(430),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|log| {
            log.spawn((
                text_bundle("", &reading, 18.0, LOG_COLOUR),
                Pickable::IGNORE,
                LogText,
            ))
            .with_children(|text| {
                for index in 0..LOG_LINES {
                    text.spawn((
                        TextSpan::new(""),
                        TextFont {
                            font: reading.clone().into(),
                            font_size: FontSize::Px(18.0),
                            ..default()
                        },
                        TextColor(LOG_COLOUR),
                        LogSpan(index),
                    ));
                }
            });
        });
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: px(112),
            left: px(0),
            right: px(0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        GlobalZIndex(10),
        Visibility::Hidden,
        Pickable::IGNORE,
        HeadlineBanner,
        children![(
            Node {
                max_width: px(620),
                padding: UiRect::axes(px(24), px(10)),
                border: UiRect::all(px(3)),
                border_radius: BorderRadius::all(px(16)),
                ..default()
            },
            BorderColor::all(BUTTON_BORDER),
            BackgroundColor(Color::srgba(0.12, 0.09, 0.07, 0.9)),
            Pickable::IGNORE,
            children![(
                text_bundle("", &bold, 30.0, Color::WHITE),
                Pickable::IGNORE,
                HeadlineText
            )],
        )],
    ));
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: px(96),
            right: px(16),
            padding: UiRect::axes(px(16), px(8)),
            border_radius: BorderRadius::all(px(12)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.1, 0.08, 0.06, 0.8)),
        Visibility::Hidden,
        Pickable::IGNORE,
        NothingLeftNotice,
        children![(
            text_bundle(
                texts.nothing_left_to_play(),
                &bold,
                24.0,
                Color::srgb(1.0, 0.9, 0.55)
            ),
            Pickable::IGNORE
        )],
    ));
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: px(16),
            right: px(16),
            column_gap: px(12),
            align_items: AlignItems::Center,
            ..default()
        },
        children![
            (
                text_bundle("", &reading, 20.0, Color::srgb(1.0, 0.6, 0.5)),
                RefusalText
            ),
            (
                button(texts.end_turn_button(), &bold, EndTurnLabel),
                EndTurnButton,
                observe(end_turn_pressed),
            ),
        ],
    ));
    commands.spawn((
        overlay(),
        Visibility::Hidden,
        ReadyOverlay,
        children![(
            panel(),
            children![
                (text_bundle("", &bold, 42.0, Color::WHITE), ReadyQuestion),
                (
                    text_bundle("", &reading, 24.0, Color::srgb(0.95, 0.85, 0.6)),
                    ReadyQuip
                ),
                (
                    button(texts.ready_button(), &bold, ReadyLabel),
                    ReadyButton,
                    observe(ready_pressed)
                ),
            ],
        )],
    ));
}

pub(crate) fn overlay() -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.05, 0.1, 0.55)),
    )
}

pub(crate) fn panel() -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(24),
            padding: UiRect::all(px(36)),
            border: UiRect::all(px(4)),
            border_radius: BorderRadius::all(px(24)),
            ..default()
        },
        BorderColor::all(Color::srgb(0.95, 0.8, 0.4)),
        BackgroundColor(Color::srgb(0.16, 0.13, 0.1)),
    )
}

pub(crate) fn button(
    label: &str,
    font: &Handle<Font>,
    label_marker: impl Component,
) -> impl Bundle {
    (
        Button,
        Hovered::default(),
        Node {
            padding: UiRect::axes(px(28), px(14)),
            border: UiRect::all(px(4)),
            border_radius: BorderRadius::all(px(18)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BorderColor::all(BUTTON_BORDER),
        BackgroundColor(BUTTON_IDLE),
        children![(
            text_bundle(label, font, 28.0, Color::srgb(1.0, 0.95, 0.85)),
            label_marker
        )],
    )
}

/// A button smaller than [`button`], for choices that come in a row.
pub(crate) fn small_button(
    label: &str,
    font: &Handle<Font>,
    label_marker: impl Component,
) -> impl Bundle {
    (
        Button,
        Hovered::default(),
        Node {
            padding: UiRect::axes(px(16), px(8)),
            border: UiRect::all(px(3)),
            border_radius: BorderRadius::all(px(12)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BorderColor::all(BUTTON_BORDER),
        BackgroundColor(BUTTON_IDLE),
        children![(
            text_bundle(label, font, 20.0, Color::srgb(1.0, 0.95, 0.85)),
            label_marker
        )],
    )
}

fn end_turn_pressed(
    _activate: On<Activate>,
    view: Res<MatchView>,
    mut confirmation: ResMut<EndTurnConfirmation>,
    mut orders: MessageWriter<MatchOrder>,
) {
    let Some(team) = human_team_in_control(&view.snapshot) else {
        return;
    };
    let goblins_left = view
        .snapshot
        .state
        .goblins_of(team)
        .any(|goblin| goblin.can_act());
    if goblins_left && confirmation.seconds_left <= 0.0 {
        confirmation.seconds_left = END_TURN_CONFIRM_SECONDS;
        return;
    }
    confirmation.seconds_left = 0.0;
    orders.write(MatchOrder::Command {
        team,
        command: Command::EndTurn,
    });
}

fn ready_pressed(
    _activate: On<Activate>,
    view: Res<MatchView>,
    handover: Res<Handover>,
    mut rig: ResMut<CameraRig>,
    mut orders: MessageWriter<MatchOrder>,
) {
    if handover.stage() != HandoverStage::AskingForReady {
        return;
    }
    if let Some(team) = team_waiting_for_ready(&view.snapshot) {
        *rig = CameraRig::behind(team);
        orders.write(MatchOrder::Ready { team });
    }
}

fn forget_the_last_match(
    mut restarts: MessageReader<MatchRestarted>,
    mut log: ResMut<EventLog>,
    mut handover: ResMut<Handover>,
    mut confirmation: ResMut<EndTurnConfirmation>,
) {
    if restarts.read().count() == 0 {
        return;
    }
    *log = EventLog::default();
    *handover = Handover::default();
    confirmation.seconds_left = 0.0;
}

fn collect_log_lines(view: Res<MatchView>, texts: Res<Texts>, mut log: ResMut<EventLog>) {
    log.push_lines(view.fresh_events.iter().filter_map(|event| {
        texts.describe_event(event).map(|text| StoryLine {
            text,
            team: event_team(event),
        })
    }));
}

fn refresh_headline(
    animation: Res<BowlAnimation>,
    mut banner: Single<&mut Visibility, With<HeadlineBanner>>,
    mut text: Single<(&mut Text, &mut TextColor), With<HeadlineText>>,
) {
    let Some(headline) = &animation.headline else {
        banner.set_if_neq(Visibility::Hidden);
        return;
    };
    banner.set_if_neq(Visibility::Inherited);
    let (text, colour) = &mut *text;
    if text.0 != headline.text {
        text.0.clone_from(&headline.text);
    }
    colour.set_if_neq(TextColor(story_colour(headline.team)));
}

fn refresh_scores(
    view: Res<MatchView>,
    texts: Res<Texts>,
    mut home: Single<&mut Text, (With<HomeScore>, Without<AwayScore>)>,
    mut away: Single<&mut Text, (With<AwayScore>, Without<HomeScore>)>,
) {
    let score = &view.snapshot.state.score;
    home.0 = format!(
        "{}  {}",
        texts.team_name(TeamId::HOME),
        score.first().copied().unwrap_or_default()
    );
    away.0 = format!(
        "{}  {}",
        score.get(1).copied().unwrap_or_default(),
        texts.team_name(TeamId::AWAY)
    );
}

fn refresh_turn_and_clock(
    view: Res<MatchView>,
    texts: Res<Texts>,
    mut turn_line: Single<&mut Text, (With<TurnLine>, Without<ClockText>)>,
    mut clock: ClockLabel,
) {
    let state = &view.snapshot.state;
    let (text, colour) = &mut *clock;
    if state.phase == Phase::Finished {
        turn_line.0 = texts.full_time().to_string();
        text.0.clear();
        return;
    }
    let turn = state
        .active_team()
        .and_then(|team| state.turns_played.get(team.index()).copied())
        .unwrap_or_default();
    turn_line.0 = texts.turn_line(state.half, turn, view.ruleset.turns_per_half);
    let (remaining_ms, paused) = match view.snapshot.clock {
        TurnClock::Running { remaining_ms } | TurnClock::WaitingForReady { remaining_ms } => {
            (remaining_ms, false)
        }
        TurnClock::Paused { remaining_ms } => (remaining_ms, true),
        TurnClock::Stopped => (0, false),
    };
    text.0 = if paused {
        format!("{} ‖", texts.clock(remaining_ms))
    } else {
        texts.clock(remaining_ms)
    };
    colour.0 = clock_colour(remaining_ms);
}

fn refresh_log(log: Res<EventLog>, mut spans: Query<(&LogSpan, &mut TextSpan, &mut TextColor)>) {
    if !log.is_changed() {
        return;
    }
    let lines: Vec<&StoryLine> = log.lines().collect();
    for (span, mut text, mut colour) in &mut spans {
        let (line, team) = lines.get(span.0).map_or((String::new(), None), |line| {
            let break_after = if span.0 + 1 < lines.len() { "\n" } else { "" };
            (format!("{}{break_after}", line.text), line.team)
        });
        if text.0 != line {
            text.0 = line;
        }
        colour.set_if_neq(TextColor(story_colour(team)));
    }
}

fn refresh_refusal(view: Res<MatchView>, mut text: Single<&mut Text, With<RefusalText>>) {
    if let Some(refusal) = &view.refusal {
        text.0.clone_from(refusal);
    } else if !view.fresh_events.is_empty() {
        text.0.clear();
    }
}

fn pause_on_key(
    keys: Res<ButtonInput<KeyCode>>,
    view: Res<MatchView>,
    mut orders: MessageWriter<MatchOrder>,
) {
    let humans_only = view
        .snapshot
        .seats
        .iter()
        .all(|seat| *seat == SeatKind::Human);
    if humans_only && keys.just_pressed(KeyCode::KeyP) {
        orders.write(MatchOrder::TogglePause);
    }
}

fn follow_handover(
    time: Res<Time>,
    view: Res<MatchView>,
    animation: Res<BowlAnimation>,
    mut handover: ResMut<Handover>,
) {
    let next = handover.after(
        waiting_turn(&view.snapshot),
        animation.busy,
        time.delta_secs(),
    );
    handover.set_if_neq(next);
}

fn refresh_ready_overlay(
    handover: Res<Handover>,
    texts: Res<Texts>,
    mut overlay: ReadyVisibility,
    mut ready_button: ReadyButtonVisibility,
    mut question: Single<&mut Text, (With<ReadyQuestion>, Without<ReadyQuip>)>,
    mut quip: Single<&mut Text, (With<ReadyQuip>, Without<ReadyQuestion>)>,
) {
    let stage = handover.stage();
    let shown = |visible: bool| {
        if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        }
    };
    overlay.set_if_neq(shown(matches!(
        stage,
        HandoverStage::Interlude | HandoverStage::AskingForReady
    )));
    ready_button.set_if_neq(shown(stage == HandoverStage::AskingForReady));
    if let Some(waiting) = handover.waiting {
        let wanted_question = texts.ready_question(waiting.team);
        if question.0 != wanted_question {
            question.0 = wanted_question;
        }
        let wanted_quip = texts.handover_quip(waiting.team, waiting.half, waiting.turn);
        if quip.0 != wanted_quip {
            quip.0 = wanted_quip.to_string();
        }
    }
}

fn refresh_end_turn_label(
    time: Res<Time>,
    texts: Res<Texts>,
    mut confirmation: ResMut<EndTurnConfirmation>,
    mut labels: Query<&mut Text, With<EndTurnLabel>>,
) {
    confirmation.seconds_left = (confirmation.seconds_left - time.delta_secs()).max(0.0);
    let label = if confirmation.seconds_left > 0.0 {
        texts.end_turn_confirmation()
    } else {
        texts.end_turn_button()
    };
    for mut text in &mut labels {
        text.0 = label.to_string();
    }
}

fn show_end_turn_to_humans(
    view: Res<MatchView>,
    mut button: Single<&mut Visibility, With<EndTurnButton>>,
) {
    button.set_if_neq(if human_team_in_control(&view.snapshot).is_some() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    });
}

fn show_nothing_left_to_play(
    time: Res<Time>,
    view: Res<MatchView>,
    mut notice: Single<&mut Visibility, With<NothingLeftNotice>>,
    mut end_turn_border: Single<&mut BorderColor, With<EndTurnButton>>,
) {
    let nothing_left = team_with_nothing_left_to_play(&view.snapshot).is_some();
    notice.set_if_neq(if nothing_left {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    });
    let border = if nothing_left {
        let glow = 0.5 + 0.5 * (time.elapsed_secs() * PULSE_PER_SECOND).sin();
        BUTTON_BORDER.mix(&Color::WHITE, glow)
    } else {
        BUTTON_BORDER
    };
    end_turn_border.set_if_neq(BorderColor::all(border));
}

fn restyle_buttons(
    mut buttons: Query<(&Hovered, Has<Pressed>, &mut BackgroundColor), With<Button>>,
) {
    for (hovered, pressed, mut background) in &mut buttons {
        *background = match (hovered.get(), pressed) {
            (_, true) => BUTTON_PRESSED.into(),
            (true, false) => BUTTON_HOVERED.into(),
            (false, false) => BUTTON_IDLE.into(),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use goblins_rules::host::{MatchHost, MatchSetup};
    use goblins_rules::roster::Roster;
    use goblins_rules::ruleset::Ruleset;

    fn snapshot(seats: [SeatKind; 2]) -> MatchSnapshot {
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
        .snapshot()
    }

    fn story(text: &str, team: Option<TeamId>) -> StoryLine {
        StoryLine {
            text: text.to_string(),
            team,
        }
    }

    #[test]
    fn given_more_lines_than_kept_when_pushed_then_only_the_latest_remain() {
        let mut log = EventLog::default();
        log.push_lines((1..=10).map(|number| story(&format!("line {number}"), None)));
        assert_eq!(
            log.lines().next().map(|line| line.text.as_str()),
            Some("line 4")
        );
    }

    #[test]
    fn given_an_away_assault_when_finding_its_team_then_the_attacker_s_team() {
        let event = GameEvent::AssaultResolved {
            style: Default::default(),
            attacker: "A3".parse().unwrap(),
            defender: "H1".parse().unwrap(),
            contest: goblins_rules::event::ContestRoll {
                actor_modifier: 0,
                actor_faces: 4,
                actor_rolled: 3,
                opponent_faces: 4,
                opponent_rolled: 2,
                opponent_modifier: 0,
                margin: 1,
            },
            defender_braced: false,
        };
        assert_eq!(event_team(&event), Some(TeamId::AWAY));
    }

    #[test]
    fn given_the_half_ending_when_finding_its_team_then_none() {
        assert_eq!(event_team(&GameEvent::HalfEnded { half: 1 }), None);
    }

    #[test]
    fn given_lines_of_both_teams_and_of_the_match_when_coloured_then_three_colours() {
        assert_eq!(
            [Some(TeamId::HOME), Some(TeamId::AWAY), None].map(story_colour),
            [HOME_COLOUR, AWAY_COLOUR, LOG_COLOUR]
        );
    }

    #[test]
    fn given_nine_seconds_left_when_colouring_the_clock_then_red() {
        assert_eq!(clock_colour(9_000), Color::srgb(1.0, 0.35, 0.3));
    }

    #[test]
    fn given_a_minute_left_when_colouring_the_clock_then_white() {
        assert_eq!(clock_colour(60_000), Color::WHITE);
    }

    #[test]
    fn given_two_humans_at_kick_off_when_asking_who_must_get_ready_then_the_team_to_play() {
        let snapshot = snapshot([SeatKind::Human, SeatKind::Human]);
        assert_eq!(
            team_waiting_for_ready(&snapshot),
            snapshot.state.active_team()
        );
    }

    #[test]
    fn given_two_bots_when_asking_who_must_get_ready_then_nobody() {
        assert_eq!(
            team_waiting_for_ready(&snapshot([SeatKind::Bot, SeatKind::Bot])),
            None
        );
    }

    #[test]
    fn given_two_humans_when_asking_who_gives_orders_then_the_team_to_play() {
        let snapshot = snapshot([SeatKind::Human, SeatKind::Human]);
        assert_eq!(
            human_team_in_control(&snapshot),
            snapshot.state.active_team()
        );
    }

    fn ready_snapshot() -> MatchSnapshot {
        let (mut host, _) = MatchHost::start(MatchSetup {
            ruleset: Ruleset::default(),
            rosters: [Roster::DEFAULT, Roster::DEFAULT],
            seed: 42,
            seats: [SeatKind::Human, SeatKind::Human],
            turn_seconds: [60, 60],
            agent_ready_grace_seconds: 15,
        })
        .unwrap();
        let team = host.snapshot().state.active_team().unwrap();
        host.ready(team).unwrap();
        host.snapshot()
    }

    fn with_every_goblin_done(mut snapshot: MatchSnapshot) -> MatchSnapshot {
        let team = snapshot.state.active_team().unwrap();
        for goblin in &mut snapshot.state.goblins {
            if goblin.team() == team {
                goblin.spend_its_turn();
            }
        }
        snapshot
    }

    #[test]
    fn given_a_team_whose_goblins_are_all_done_when_its_clock_runs_then_it_has_nothing_left_to_play()
     {
        let snapshot = with_every_goblin_done(ready_snapshot());
        assert_eq!(
            team_with_nothing_left_to_play(&snapshot),
            snapshot.state.active_team()
        );
    }

    #[test]
    fn given_a_team_with_a_goblin_left_when_its_clock_runs_then_it_still_has_something_to_play() {
        assert_eq!(team_with_nothing_left_to_play(&ready_snapshot()), None);
    }

    #[test]
    fn given_a_team_whose_goblins_are_all_done_while_waiting_for_ready_then_no_notice_yet() {
        let snapshot = with_every_goblin_done(snapshot([SeatKind::Human, SeatKind::Human]));
        assert_eq!(team_with_nothing_left_to_play(&snapshot), None);
    }

    const AWAY_TURN_THREE: WaitingTurn = WaitingTurn {
        team: TeamId::AWAY,
        half: 1,
        turn: 3,
    };

    #[test]
    fn given_no_human_turn_waiting_when_the_frame_passes_then_no_handover() {
        let handover = Handover::default().after(None, false, 0.5);
        assert_eq!(handover.stage(), HandoverStage::NoHandover);
    }

    #[test]
    fn given_a_turn_waiting_while_the_bowl_still_moves_then_the_bowl_is_watched() {
        let handover = Handover::default().after(Some(AWAY_TURN_THREE), true, 0.5);
        assert_eq!(handover.stage(), HandoverStage::WatchingTheBowl);
    }

    #[test]
    fn given_a_turn_waiting_for_one_still_second_then_the_interlude_shows() {
        let handover = Handover::default()
            .after(Some(AWAY_TURN_THREE), true, 0.5)
            .after(Some(AWAY_TURN_THREE), false, 1.0);
        assert_eq!(handover.stage(), HandoverStage::Interlude);
    }

    #[test]
    fn given_a_turn_waiting_for_three_still_seconds_then_ready_is_asked() {
        let handover = Handover::default()
            .after(Some(AWAY_TURN_THREE), false, 1.5)
            .after(Some(AWAY_TURN_THREE), false, 1.5);
        assert_eq!(handover.stage(), HandoverStage::AskingForReady);
    }

    #[test]
    fn given_a_new_turn_waiting_when_the_frame_passes_then_the_stillness_starts_over() {
        let next_turn = WaitingTurn {
            team: TeamId::HOME,
            ..AWAY_TURN_THREE
        };
        let handover = Handover::default()
            .after(Some(AWAY_TURN_THREE), false, 3.0)
            .after(Some(next_turn), false, 0.1);
        assert_eq!(handover.stage(), HandoverStage::Interlude);
    }

    #[test]
    fn given_two_bots_when_asking_who_gives_orders_then_nobody() {
        assert_eq!(
            human_team_in_control(&snapshot([SeatKind::Bot, SeatKind::Bot])),
            None
        );
    }
}
