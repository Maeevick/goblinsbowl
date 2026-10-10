//! Dice popups: what each roll showed, rising over the goblin who rolled, then fading away.

use bevy::prelude::*;
use bevy::transform::TransformSystems;
use bevy::ui::UiSystems;
use goblins_rules::roster::GoblinId;

use crate::beats::RollShown;
use crate::goblin_view::GoblinFigure;
use crate::hud::{BOLD_FONT, READING_FONT};
use crate::local_match::MatchSystems;
use crate::texts::Texts;

const POPUP_LIFE_SECONDS: f32 = 2.4;
const POPUP_FADE_SECONDS: f32 = 0.7;
const POPUP_RISE_PER_SECOND: f32 = 26.0;
const POPUP_WIDTH: f32 = 280.0;
const POPUP_HEIGHT: f32 = 62.0;
const POPUP_ABOVE_FEET: f32 = 2.0;
const SUCCESS_COLOUR: Color = Color::srgb(0.45, 1.0, 0.5);
const FAILURE_COLOUR: Color = Color::srgb(1.0, 0.42, 0.35);
const DETAIL_COLOUR: Color = Color::srgb(0.95, 0.95, 0.9);
const SHADOW_OPACITY: f32 = 0.85;

#[derive(Component)]
struct DicePopup {
    goblin: GoblinId,
    age: f32,
}

/// Shows every roll as a popup over the goblin who rolled.
pub struct DiceViewPlugin;

impl Plugin for DiceViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<RollShown>()
            .add_systems(Update, spawn_popups.in_set(MatchSystems::Present))
            .add_systems(
                PostUpdate,
                float_popups
                    .after(TransformSystems::Propagate)
                    .before(UiSystems::Layout),
            );
    }
}

/// Opacity of a popup `age` seconds old: solid at first, then fading out until it disappears.
pub fn popup_alpha(age: f32) -> f32 {
    ((POPUP_LIFE_SECONDS - age) / POPUP_FADE_SECONDS).clamp(0.0, 1.0)
}

fn popup_line(text: &str, font: &Handle<Font>, size: f32, colour: Color) -> impl Bundle {
    (
        Text::new(text),
        TextFont {
            font: font.clone().into(),
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(colour),
        TextShadow {
            offset: Vec2::splat(1.5),
            color: Color::BLACK.with_alpha(SHADOW_OPACITY),
        },
        Pickable::IGNORE,
    )
}

fn spawn_popups(
    mut commands: Commands,
    mut rolls: MessageReader<RollShown>,
    texts: Res<Texts>,
    assets: Res<AssetServer>,
) {
    for shown in rolls.read() {
        let reading: Handle<Font> = assets.load(READING_FONT);
        let bold: Handle<Font> = assets.load(BOLD_FONT);
        let headline_colour = if shown.roll.succeeded() {
            SUCCESS_COLOUR
        } else {
            FAILURE_COLOUR
        };
        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                width: px(POPUP_WIDTH),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            },
            Visibility::Hidden,
            Pickable::IGNORE,
            DicePopup {
                goblin: shown.goblin,
                age: 0.0,
            },
            children![
                popup_line(
                    texts.roll_headline(&shown.roll),
                    &bold,
                    30.0,
                    headline_colour
                ),
                popup_line(
                    &texts.roll_detail(&shown.roll),
                    &reading,
                    18.0,
                    DETAIL_COLOUR
                ),
            ],
        ));
    }
}

fn float_popups(
    mut commands: Commands,
    time: Res<Time>,
    camera: Single<(&Camera, &GlobalTransform)>,
    figures: Query<(&GoblinFigure, &GlobalTransform)>,
    mut popups: Query<(
        Entity,
        &mut DicePopup,
        &mut Node,
        &mut Visibility,
        &Children,
    )>,
    mut lines: Query<(&mut TextColor, &mut TextShadow)>,
) {
    let (camera, camera_transform) = *camera;
    for (entity, mut popup, mut node, mut visibility, children) in &mut popups {
        popup.age += time.delta_secs();
        if popup.age >= POPUP_LIFE_SECONDS {
            commands.entity(entity).despawn();
            continue;
        }
        let above_goblin = figures
            .iter()
            .find(|(figure, _)| figure.id == popup.goblin)
            .map(|(_, placement)| placement.translation() + Vec3::Y * POPUP_ABOVE_FEET);
        let on_screen =
            above_goblin.and_then(|point| camera.world_to_viewport(camera_transform, point).ok());
        let Some(position) = on_screen else {
            visibility.set_if_neq(Visibility::Hidden);
            continue;
        };
        node.left = px(position.x - POPUP_WIDTH / 2.0);
        node.top = px(position.y - POPUP_HEIGHT - popup.age * POPUP_RISE_PER_SECOND);
        visibility.set_if_neq(Visibility::Inherited);
        let alpha = popup_alpha(popup.age);
        for child in children.iter() {
            if let Ok((mut colour, mut shadow)) = lines.get_mut(child) {
                colour.0.set_alpha(alpha);
                shadow.color.set_alpha(alpha * SHADOW_OPACITY);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_a_fresh_popup_when_fading_it_then_solid() {
        assert_eq!(popup_alpha(0.2), 1.0);
    }

    #[test]
    fn given_a_popup_at_the_end_of_its_life_when_fading_it_then_invisible() {
        assert_eq!(popup_alpha(POPUP_LIFE_SECONDS), 0.0);
    }

    #[test]
    fn given_a_popup_halfway_through_its_fade_when_fading_it_then_half_transparent() {
        let halfway = POPUP_LIFE_SECONDS - POPUP_FADE_SECONDS / 2.0;
        assert!((popup_alpha(halfway) - 0.5).abs() < 1e-5);
    }
}
