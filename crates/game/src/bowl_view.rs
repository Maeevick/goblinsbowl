//! The bowl drawn as low-poly floor tiles inside a ring of stands, with hover and click picking.

use bevy::prelude::*;
use goblins_rules::arena::Arena;
use goblins_rules::roster::GoblinId;
use goblins_rules::square::Square;
use goblins_rules::team::TeamId;

use crate::local_match::MatchView;

const TILE_GAP: f32 = 0.04;
const TILE_THICKNESS: f32 = 0.1;
const STAND_HEIGHT: f32 = 0.7;
const HIGHLIGHT_LIFT: f32 = 0.06;

/// A pickable floor tile and the square it stands for.
#[derive(Component, Clone, Copy, Debug)]
pub struct Tile {
    /// Square of the arena covered by this tile.
    pub square: Square,
}

/// The square currently under the pointer, if any.
#[derive(Resource, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct HoveredSquare(pub Option<Square>);

/// What a primary click on the board pointed at.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoardClick {
    /// A floor square.
    Square(Square),
    /// A goblin figure.
    Goblin(GoblinId),
}

#[derive(Component)]
struct HoverHighlight;

/// Spawns the bowl of the match, its stands and the hover highlight, and tracks hovered and clicked squares.
pub struct BowlViewPlugin;

impl Plugin for BowlViewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HoveredSquare>()
            .add_message::<BoardClick>()
            .add_systems(Startup, (spawn_bowl, spawn_lights))
            .add_systems(Update, follow_hovered_square);
    }
}

/// World position of the centre of a grid cell, the grid being centred on the world origin.
///
/// Columns run along +X (west to east) and rows along +Z (north to south); cells are one unit wide.
/// Coordinates may lie outside the grid, which is how the stands around it are placed.
pub fn grid_cell_centre(arena_width: u8, arena_height: u8, x: i16, y: i16) -> Vec3 {
    Vec3::new(
        f32::from(x) - (f32::from(arena_width) - 1.0) / 2.0,
        0.0,
        f32::from(y) - (f32::from(arena_height) - 1.0) / 2.0,
    )
}

/// World position of the centre of `square` on the floor of `arena`.
pub fn square_centre(arena: &Arena, square: Square) -> Vec3 {
    grid_cell_centre(
        arena.width(),
        arena.height(),
        i16::from(square.x),
        i16::from(square.y),
    )
}

fn is_next_to_the_field(arena: &Arena, x: i16, y: i16) -> bool {
    (-1..=1).any(|dy| {
        (-1..=1).any(|dx| {
            let neighbour_x = u8::try_from(x + dx).ok();
            let neighbour_y = u8::try_from(y + dy).ok();
            neighbour_x
                .zip(neighbour_y)
                .is_some_and(|(nx, ny)| arena.is_in_bounds(Square::new(nx, ny)))
        })
    })
}

fn is_in_bounds_cell(arena: &Arena, x: i16, y: i16) -> bool {
    let column = u8::try_from(x).ok();
    let row = u8::try_from(y).ok();
    column
        .zip(row)
        .is_some_and(|(column, row)| arena.is_in_bounds(Square::new(column, row)))
}

fn floor_colour(arena: &Arena, square: Square) -> Color {
    match arena.den_owner(square) {
        Some(TeamId::HOME) => Color::srgb(0.25, 0.45, 0.85),
        Some(_) => Color::srgb(0.9, 0.7, 0.2),
        None if (square.x + square.y).is_multiple_of(2) => Color::srgb(0.78, 0.66, 0.45),
        None => Color::srgb(0.72, 0.6, 0.4),
    }
}

fn spawn_bowl(
    mut commands: Commands,
    view: Res<MatchView>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let arena = &view.arena;
    let tile_mesh = meshes.add(Cuboid::new(1.0 - TILE_GAP, TILE_THICKNESS, 1.0 - TILE_GAP));
    let stand_mesh = meshes.add(Cuboid::new(1.0, STAND_HEIGHT, 1.0));
    let stand_material = materials.add(Color::srgb(0.45, 0.42, 0.4));

    for square in arena.in_bounds_squares() {
        let centre = square_centre(arena, square) - Vec3::Y * TILE_THICKNESS / 2.0;
        commands
            .spawn((
                Mesh3d(tile_mesh.clone()),
                MeshMaterial3d(materials.add(floor_colour(arena, square))),
                Transform::from_translation(centre),
                Tile { square },
            ))
            .observe(remember_hovered_tile)
            .observe(forget_hovered_tile)
            .observe(remember_clicked_tile);
    }

    let width = i16::from(arena.width());
    let height = i16::from(arena.height());
    for y in -1..=height {
        for x in -1..=width {
            if is_in_bounds_cell(arena, x, y) || !is_next_to_the_field(arena, x, y) {
                continue;
            }
            let centre = grid_cell_centre(arena.width(), arena.height(), x, y)
                + Vec3::Y * (STAND_HEIGHT / 2.0 - TILE_THICKNESS);
            commands.spawn((
                Mesh3d(stand_mesh.clone()),
                MeshMaterial3d(stand_material.clone()),
                Transform::from_translation(centre),
                Pickable::IGNORE,
            ));
        }
    }

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.96, 0.02, 0.96))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 1.0, 1.0, 0.45),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })),
        Transform::default(),
        Visibility::Hidden,
        Pickable::IGNORE,
        HoverHighlight,
    ));
}

fn spawn_lights(mut commands: Commands) {
    commands.insert_resource(GlobalAmbientLight {
        brightness: 350.0,
        ..default()
    });
    commands.spawn((
        DirectionalLight {
            illuminance: 9_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(-6.0, 14.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn remember_hovered_tile(
    over: On<PointerOver>,
    tiles: Query<&Tile>,
    mut hovered: ResMut<HoveredSquare>,
) {
    if let Ok(tile) = tiles.get(over.event_target()) {
        hovered.0 = Some(tile.square);
    }
}

fn forget_hovered_tile(
    out: On<PointerOut>,
    tiles: Query<&Tile>,
    mut hovered: ResMut<HoveredSquare>,
) {
    if tiles
        .get(out.event_target())
        .is_ok_and(|tile| hovered.0 == Some(tile.square))
    {
        hovered.0 = None;
    }
}

fn remember_clicked_tile(
    click: On<PointerClick>,
    tiles: Query<&Tile>,
    mut clicks: MessageWriter<BoardClick>,
) {
    if click.button != PointerButton::Primary {
        return;
    }
    if let Ok(tile) = tiles.get(click.event_target()) {
        clicks.write(BoardClick::Square(tile.square));
    }
}

fn follow_hovered_square(
    hovered: Res<HoveredSquare>,
    view: Res<MatchView>,
    mut highlight: Single<(&mut Transform, &mut Visibility), With<HoverHighlight>>,
) {
    let (transform, visibility) = &mut *highlight;
    match hovered.0 {
        Some(square) => {
            transform.translation = square_centre(&view.arena, square) + Vec3::Y * HIGHLIGHT_LIFT;
            **visibility = Visibility::Visible;
        }
        None => **visibility = Visibility::Hidden,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::local_match::LocalMatchPlugin;
    use crate::test_support::{click_on, written};
    use goblins_rules::arena::BOWL_12_MASK;
    use goblins_rules::host::{MatchHost, MatchSetup, SeatKind};
    use goblins_rules::roster::Roster;
    use goblins_rules::ruleset::Ruleset;

    fn app_drawing(arena_mask: &[&str]) -> App {
        let ruleset = Ruleset {
            arena: arena_mask.iter().map(|row| row.to_string()).collect(),
            ..Ruleset::default()
        };
        let (host, opening_events) = MatchHost::start(MatchSetup {
            ruleset,
            rosters: [Roster::DEFAULT, Roster::DEFAULT],
            seed: 42,
            seats: [SeatKind::Human, SeatKind::Human],
            turn_seconds: [60, 60],
            agent_ready_grace_seconds: 15,
        })
        .unwrap();
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            LocalMatchPlugin {
                host,
                opening_events,
            },
            BowlViewPlugin,
        ))
        .init_asset::<Mesh>()
        .init_asset::<StandardMaterial>();
        app.update();
        app
    }

    fn app_with_a_tile(square: Square) -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_message::<BoardClick>();
        let tile = app
            .world_mut()
            .spawn(Tile { square })
            .observe(remember_clicked_tile)
            .id();
        (app, tile)
    }

    #[test]
    fn given_a_tile_when_clicked_then_its_square_is_clicked() {
        let (mut app, tile) = app_with_a_tile(Square::new(4, 7));
        app.world_mut()
            .trigger(click_on(tile, PointerButton::Primary));
        assert_eq!(
            written::<BoardClick>(&app),
            vec![BoardClick::Square(Square::new(4, 7))]
        );
    }

    #[test]
    fn given_a_tile_when_right_clicked_then_nothing_is_clicked() {
        let (mut app, tile) = app_with_a_tile(Square::new(4, 7));
        app.world_mut()
            .trigger(click_on(tile, PointerButton::Secondary));
        assert_eq!(written::<BoardClick>(&app), Vec::new());
    }

    #[test]
    fn given_a_match_on_the_12_bowl_when_drawing_it_then_one_tile_per_square() {
        let mut app = app_drawing(&BOWL_12_MASK);
        let tiles = app.world_mut().query::<&Tile>().iter(app.world()).count();
        assert_eq!(tiles, 108);
    }

    #[test]
    fn given_bowl_15_when_placing_the_centre_square_then_world_origin() {
        assert_eq!(
            square_centre(&Arena::bowl_15(), Square::new(7, 7)),
            Vec3::ZERO
        );
    }

    #[test]
    fn given_bowl_15_when_placing_the_home_den_then_west_of_the_origin() {
        assert!(square_centre(&Arena::bowl_15(), Square::new(0, 7)).x < 0.0);
    }
}
