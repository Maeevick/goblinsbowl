//! The menu a local match opens on: pick the bowl, then play. Other choices will join it later.

use bevy::prelude::*;
use bevy::ui_widgets::{Activate, observe};
use goblins_rules::bowls::{BowlShape, BowlSize, bowl_mask};

use crate::hud::{BOLD_FONT, READING_FONT, button, overlay, panel, small_button, text_bundle};
use crate::local_match::{MatchOrder, MatchView};
use crate::texts::Texts;

const PICKED_BORDER: Color = Color::WHITE;
const UNPICKED_BORDER: Color = Color::srgb(0.95, 0.8, 0.4);

/// What the menu has picked so far, and whether it is still open.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct StartMenu {
    /// The bowl picked; none keeps the bowl of the rules file.
    pub bowl: Option<(BowlShape, BowlSize)>,
    /// Whether the menu still covers the bowl, the match waiting behind it.
    pub open: bool,
}

impl StartMenu {
    /// The menu opened on `arena_rows`: its bowl picked when it is one of the shapes and sizes,
    /// otherwise the rules file's own.
    pub fn opening_on(arena_rows: &[String]) -> Self {
        let bowl = BowlShape::ALL
            .into_iter()
            .flat_map(|shape| BowlSize::ALL.into_iter().map(move |size| (shape, size)))
            .find(|(shape, size)| bowl_mask(*shape, *size) == arena_rows);
        Self { bowl, open: true }
    }

    /// The menu with `shape` picked, keeping the size picked or a tiny one.
    pub fn with_shape(self, shape: BowlShape) -> Self {
        let size = self.bowl.map_or(BowlSize::Tiny, |(_, size)| size);
        Self {
            bowl: Some((shape, size)),
            ..self
        }
    }

    /// The menu with `size` picked, keeping the shape picked or a circle.
    pub fn with_size(self, size: BowlSize) -> Self {
        let shape = self.bowl.map_or(BowlShape::Circle, |(shape, _)| shape);
        Self {
            bowl: Some((shape, size)),
            ..self
        }
    }

    /// The order that starts the match picked: on the picked bowl when it differs from the bowl
    /// already set up, otherwise none.
    pub fn order_to_play(&self, arena_rows: &[String]) -> Option<MatchOrder> {
        let (shape, size) = self.bowl?;
        let arena = bowl_mask(shape, size);
        (arena != arena_rows).then_some(MatchOrder::RestartIn { arena })
    }
}

#[derive(Component)]
struct MenuScreen;

#[derive(Component, Clone, Copy)]
enum MenuChoice {
    Shape(BowlShape),
    Size(BowlSize),
}

#[derive(Component)]
struct ChoiceLabel;

#[derive(Component)]
struct PlayLabel;

#[derive(Component)]
struct RulesFileBowl;

/// Opens the start menu over a local match and starts the match on the bowl picked.
pub struct StartMenuPlugin;

impl Plugin for StartMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, open_the_menu)
            .add_systems(Update, (show_the_picks, close_once_played).chain());
    }
}

fn open_the_menu(
    mut commands: Commands,
    assets: Res<AssetServer>,
    texts: Res<Texts>,
    view: Res<MatchView>,
) {
    commands.insert_resource(StartMenu::opening_on(&view.ruleset.arena));
    let bold: Handle<Font> = assets.load(BOLD_FONT);
    let reading: Handle<Font> = assets.load(READING_FONT);
    let choice = |label: &'static str, choice: MenuChoice| menu_choice(label, &bold, choice);
    let row = || Node {
        column_gap: px(12),
        ..default()
    };
    commands.spawn((
        overlay(),
        GlobalZIndex(50),
        MenuScreen,
        children![(
            panel(),
            children![
                text_bundle("Goblins Bowl", &bold, 44.0, Color::WHITE),
                text_bundle(texts.arena_heading(), &reading, 22.0, Color::WHITE),
                (
                    row(),
                    children![
                        choice(
                            texts.shape_name(BowlShape::Circle),
                            MenuChoice::Shape(BowlShape::Circle)
                        ),
                        choice(
                            texts.shape_name(BowlShape::Cross),
                            MenuChoice::Shape(BowlShape::Cross)
                        ),
                        choice(
                            texts.shape_name(BowlShape::Rectangle),
                            MenuChoice::Shape(BowlShape::Rectangle)
                        ),
                    ],
                ),
                (
                    row(),
                    children![
                        choice(
                            texts.size_name(BowlSize::Tiny),
                            MenuChoice::Size(BowlSize::Tiny)
                        ),
                        choice(
                            texts.size_name(BowlSize::Medium),
                            MenuChoice::Size(BowlSize::Medium)
                        ),
                        choice(
                            texts.size_name(BowlSize::Huge),
                            MenuChoice::Size(BowlSize::Huge)
                        ),
                    ],
                ),
                (
                    text_bundle(
                        texts.rules_file_bowl(),
                        &reading,
                        18.0,
                        Color::srgb(1.0, 0.85, 0.45)
                    ),
                    RulesFileBowl
                ),
                (
                    button(texts.play_button(), &bold, PlayLabel),
                    observe(play_pressed)
                ),
            ],
        )],
    ));
}

fn menu_choice(label: &'static str, font: &Handle<Font>, choice: MenuChoice) -> impl Bundle {
    (
        small_button(label, font, ChoiceLabel),
        choice,
        observe(
            move |_activate: On<Activate>, mut menu: ResMut<StartMenu>| {
                *menu = match choice {
                    MenuChoice::Shape(shape) => menu.with_shape(shape),
                    MenuChoice::Size(size) => menu.with_size(size),
                };
            },
        ),
    )
}

fn play_pressed(
    _activate: On<Activate>,
    view: Res<MatchView>,
    mut menu: ResMut<StartMenu>,
    mut orders: MessageWriter<MatchOrder>,
) {
    if let Some(order) = menu.order_to_play(&view.ruleset.arena) {
        orders.write(order);
    }
    menu.open = false;
}

fn show_the_picks(
    menu: Option<Res<StartMenu>>,
    mut choices: Query<(&MenuChoice, &mut BorderColor)>,
    mut rules_file_bowl: Query<&mut Node, With<RulesFileBowl>>,
) {
    let Some(menu) = menu.filter(|menu| menu.is_changed()) else {
        return;
    };
    for (choice, mut border) in &mut choices {
        let picked = match (choice, menu.bowl) {
            (MenuChoice::Shape(shape), Some((picked, _))) => *shape == picked,
            (MenuChoice::Size(size), Some((_, picked))) => *size == picked,
            (_, None) => false,
        };
        *border = BorderColor::all(if picked {
            PICKED_BORDER
        } else {
            UNPICKED_BORDER
        });
    }
    for mut node in &mut rules_file_bowl {
        node.display = if menu.bowl.is_none() {
            Display::Flex
        } else {
            Display::None
        };
    }
}

fn close_once_played(
    mut commands: Commands,
    menu: Option<Res<StartMenu>>,
    screens: Query<Entity, With<MenuScreen>>,
) {
    if menu.is_some_and(|menu| !menu.open) {
        for screen in &screens {
            commands.entity(screen).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use goblins_rules::ruleset::Ruleset;

    #[test]
    fn given_the_shipped_round_bowl_when_the_menu_opens_then_the_tiny_circle_is_picked() {
        assert_eq!(
            StartMenu::opening_on(&Ruleset::default().arena).bowl,
            Some((BowlShape::Circle, BowlSize::Tiny))
        );
    }

    #[test]
    fn given_a_bowl_of_the_rules_file_alone_when_the_menu_opens_then_nothing_is_picked() {
        let custom = vec!["HOOA".to_string(), "HOOA".to_string()];
        assert_eq!(StartMenu::opening_on(&custom).bowl, None);
    }

    #[test]
    fn given_the_tiny_circle_when_picking_the_rectangle_then_a_tiny_rectangle() {
        let menu =
            StartMenu::opening_on(&Ruleset::default().arena).with_shape(BowlShape::Rectangle);
        assert_eq!(menu.bowl, Some((BowlShape::Rectangle, BowlSize::Tiny)));
    }

    #[test]
    fn given_a_medium_cross_picked_when_playing_then_the_match_starts_again_on_it() {
        let shipped = Ruleset::default().arena;
        let menu = StartMenu::opening_on(&shipped)
            .with_shape(BowlShape::Cross)
            .with_size(BowlSize::Medium);
        assert_eq!(
            menu.order_to_play(&shipped),
            Some(MatchOrder::RestartIn {
                arena: bowl_mask(BowlShape::Cross, BowlSize::Medium)
            })
        );
    }

    #[test]
    fn given_the_bowl_already_set_up_when_playing_then_the_match_goes_on_as_it_is() {
        let shipped = Ruleset::default().arena;
        assert_eq!(
            StartMenu::opening_on(&shipped).order_to_play(&shipped),
            None
        );
    }
}
