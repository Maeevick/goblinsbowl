//! Heads-up display: title, the square under the pointer, and the end-of-turn button.

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::Pressed;
use bevy::ui_widgets::{Activate, Button, observe};
use goblins_rules::square::Square;

use crate::bowl_view::{HoveredSquare, SelectedSquare};

const READING_FONT: &str = "fonts/Andika-Regular.ttf";
const BOLD_FONT: &str = "fonts/Andika-Bold.ttf";
const BUTTON_IDLE: Color = Color::srgb(0.2, 0.17, 0.12);
const BUTTON_HOVERED: Color = Color::srgb(0.32, 0.26, 0.16);
const BUTTON_PRESSED: Color = Color::srgb(0.55, 0.42, 0.15);

#[derive(Component)]
struct SquareReadout;

/// Shows the HUD and keeps its readouts in sync with the bowl.
pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_hud)
            .add_systems(Update, (refresh_square_readout, restyle_buttons));
    }
}

/// Text shown for the hovered and clicked squares, in French.
pub fn square_readout_text(hovered: Option<Square>, selected: Option<Square>) -> String {
    let describe = |square: Option<Square>| {
        square.map_or_else(|| "aucune".to_string(), |s| format!("[{}, {}]", s.x, s.y))
    };
    format!(
        "Case survolée : {} · Case choisie : {}",
        describe(hovered),
        describe(selected)
    )
}

fn spawn_hud(mut commands: Commands, assets: Res<AssetServer>) {
    let reading_font: Handle<Font> = assets.load(READING_FONT);
    let bold_font: Handle<Font> = assets.load(BOLD_FONT);
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            right: px(12),
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            ..default()
        },
        children![
            (
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4),
                    ..default()
                },
                children![
                    (
                        Text::new("Goblins Bowl · À toi de jouer !"),
                        TextFont {
                            font: bold_font.clone().into(),
                            font_size: FontSize::Px(34.0),
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.92, 0.7)),
                        TextShadow::default(),
                    ),
                    (
                        Text::new(square_readout_text(None, None)),
                        TextFont {
                            font: reading_font.into(),
                            font_size: FontSize::Px(22.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        TextShadow::default(),
                        SquareReadout,
                    ),
                ],
            ),
            (
                end_turn_button(bold_font),
                observe(|_activate: On<Activate>| {
                    info!("Fin du tour demandée");
                }),
            ),
        ],
    ));
}

fn end_turn_button(font: Handle<Font>) -> impl Bundle {
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
        BorderColor::all(Color::srgb(0.95, 0.8, 0.4)),
        BackgroundColor(BUTTON_IDLE),
        children![(
            Text::new("Fin du tour"),
            TextFont {
                font: font.into(),
                font_size: FontSize::Px(28.0),
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.95, 0.85)),
        )],
    )
}

fn refresh_square_readout(
    hovered: Res<HoveredSquare>,
    selected: Res<SelectedSquare>,
    mut readout: Single<&mut Text, With<SquareReadout>>,
) {
    if hovered.is_changed() || selected.is_changed() {
        ***readout = square_readout_text(hovered.0, selected.0);
    }
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

    #[test]
    fn given_nothing_hovered_nor_chosen_when_describing_then_both_say_none() {
        assert_eq!(
            square_readout_text(None, None),
            "Case survolée : aucune · Case choisie : aucune"
        );
    }

    #[test]
    fn given_hovered_and_chosen_squares_when_describing_then_both_coordinates_show() {
        assert_eq!(
            square_readout_text(Some(Square::new(7, 7)), Some(Square::new(0, 3))),
            "Case survolée : [7, 7] · Case choisie : [0, 3]"
        );
    }
}
