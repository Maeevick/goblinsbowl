//! The replay bar: the order just played with its chance, the goblins whose safe orders could have
//! come first, and the controls: previous turn, play or pause, next turn, the speed and quit. Space
//! plays or pauses, and 1, 2 and 3 pick the speed.

use bevy::prelude::*;
use bevy::ui_widgets::{Activate, observe};

use crate::hud::{BOLD_FONT, READING_FONT, small_button, text_bundle};
use crate::local_match::{MatchJumped, MatchSystems};
use crate::orders::{risk_colour, risk_of};
use crate::replay::{Jump, Replay};
use crate::texts::Texts;

const PICKED_BORDER: Color = Color::WHITE;
const UNPICKED_BORDER: Color = Color::srgb(0.95, 0.8, 0.4);
const HINT_COLOUR: Color = Color::srgb(1.0, 0.65, 0.3);
const BACKDROP: Color = Color::srgba(0.07, 0.08, 0.11, 0.78);
const SPEEDS: [u8; 3] = [1, 2, 3];

#[derive(Component)]
struct OrderLine;

#[derive(Component)]
struct HintLine;

#[derive(Component)]
struct PlayPauseLabel;

#[derive(Component)]
struct ButtonLabel;

#[derive(Component, Clone, Copy)]
struct SpeedButton(u8);

/// Shows the replay bar and steers the replay from it and from the keyboard.
pub struct ReplayBarPlugin;

impl Plugin for ReplayBarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_replay_bar).add_systems(
            Update,
            (
                steer_from_keys.in_set(MatchSystems::Apply),
                refresh_replay_bar.after(MatchSystems::Present),
            ),
        );
    }
}

fn spawn_replay_bar(mut commands: Commands, assets: Res<AssetServer>, texts: Res<Texts>) {
    let bold: Handle<Font> = assets.load(BOLD_FONT);
    let reading: Handle<Font> = assets.load(READING_FONT);
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: px(16),
            bottom: px(16),
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
            padding: UiRect::all(px(12)),
            border_radius: BorderRadius::all(px(14)),
            ..default()
        },
        BackgroundColor(BACKDROP),
        children![
            (text_bundle("", &bold, 26.0, Color::WHITE), OrderLine),
            (text_bundle("", &reading, 20.0, HINT_COLOUR), HintLine),
            (
                Node {
                    column_gap: px(8),
                    ..default()
                },
                children![
                    (
                        small_button(texts.previous_turn_button(), &bold, ButtonLabel),
                        observe(jump_pressed(Jump::PreviousTurn))
                    ),
                    (
                        small_button(texts.pause_button(), &bold, PlayPauseLabel),
                        observe(play_pause_pressed)
                    ),
                    (
                        small_button(texts.next_turn_button(), &bold, ButtonLabel),
                        observe(jump_pressed(Jump::NextTurn))
                    ),
                    speed_button(1, &texts, &bold),
                    speed_button(2, &texts, &bold),
                    speed_button(3, &texts, &bold),
                    (
                        small_button(texts.quit_button(), &bold, ButtonLabel),
                        observe(quit_pressed)
                    ),
                ],
            ),
        ],
    ));
}

fn speed_button(speed: u8, texts: &Texts, font: &Handle<Font>) -> impl Bundle {
    (
        small_button(texts.speed_button(speed), font, ButtonLabel),
        SpeedButton(speed),
        observe(move |_activate: On<Activate>, mut replay: ResMut<Replay>| {
            replay.set_speed(speed);
        }),
    )
}

fn jump_pressed(
    jump: Jump,
) -> impl FnMut(On<Activate>, ResMut<Replay>, MessageWriter<MatchJumped>) {
    move |_activate, mut replay, mut jumps| {
        let story = replay.jump(jump);
        jumps.write(MatchJumped { story });
    }
}

fn play_pause_pressed(
    _activate: On<Activate>,
    mut replay: ResMut<Replay>,
    mut jumps: MessageWriter<MatchJumped>,
) {
    if let Some(story) = replay.toggle_playing() {
        jumps.write(MatchJumped { story });
    }
}

fn quit_pressed(_activate: On<Activate>, mut exit: MessageWriter<AppExit>) {
    exit.write(AppExit::Success);
}

fn steer_from_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut replay: ResMut<Replay>,
    mut jumps: MessageWriter<MatchJumped>,
) {
    if keys.just_pressed(KeyCode::Space)
        && let Some(story) = replay.toggle_playing()
    {
        jumps.write(MatchJumped { story });
    }
    for (key, speed) in [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3]
        .into_iter()
        .zip(SPEEDS)
    {
        if keys.just_pressed(key) {
            replay.set_speed(speed);
        }
    }
}

type OrderLineText<'w, 's> = Single<
    'w,
    's,
    (&'static mut Text, &'static mut TextColor),
    (With<OrderLine>, Without<HintLine>, Without<PlayPauseLabel>),
>;
type HintLineText<'w, 's> = Single<
    'w,
    's,
    &'static mut Text,
    (With<HintLine>, Without<OrderLine>, Without<PlayPauseLabel>),
>;
type PlayPauseText<'w, 's> = Single<
    'w,
    's,
    &'static mut Text,
    (With<PlayPauseLabel>, Without<OrderLine>, Without<HintLine>),
>;

fn refresh_replay_bar(
    replay: Res<Replay>,
    texts: Res<Texts>,
    mut order_line: OrderLineText,
    mut hint_line: HintLineText,
    mut play_pause: PlayPauseText,
    mut speeds: Query<(&SpeedButton, &mut BorderColor)>,
) {
    let (order_text, order_colour) = &mut *order_line;
    let (order, colour, hint) = match replay.last_order() {
        Some(order) => (
            texts.replayed_order(&order.activation, &order.odds),
            risk_colour(risk_of(order.odds.overall_success)),
            texts.safe_orders_first(&order.safe_orders_played_late),
        ),
        None => (String::new(), Color::WHITE, String::new()),
    };
    if order_text.0 != order {
        order_text.0 = order;
    }
    order_colour.0 = colour;
    if hint_line.0 != hint {
        hint_line.0 = hint;
    }
    let label = if replay.is_playing() {
        texts.pause_button()
    } else {
        texts.replay_play_button()
    };
    if play_pause.0 != label {
        play_pause.0 = label.to_string();
    }
    for (button, mut border) in &mut speeds {
        *border = BorderColor::all(if button.0 == replay.speed() {
            PICKED_BORDER
        } else {
            UNPICKED_BORDER
        });
    }
}
