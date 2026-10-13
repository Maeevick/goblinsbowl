//! Goblins and the ball on the bowl: low-poly figures built from primitives, walking square by square.

use std::collections::VecDeque;
use std::f32::consts::FRAC_PI_2;

use bevy::light::NotShadowCaster;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::transform::TransformSystems;
use bevy::ui::UiSystems;
use goblins_rules::arena::Arena;
use goblins_rules::characteristics::Archetype;
use goblins_rules::roster::GoblinId;
use goblins_rules::square::Square;
use goblins_rules::state::{Ball, GameState, Goblin, Presence};
use goblins_rules::team::TeamId;

use crate::beats::{Beat, RollShown, beats_from_events};
use crate::bowl_view::{BoardClick, square_centre};
use crate::local_match::{MatchRestarted, MatchSystems, MatchView};

const WALK_SPEED: f32 = 7.0;
const FALL_SPEED: f32 = 6.0;
const BALL_SPEED: f32 = 12.0;
const BALL_RADIUS: f32 = 0.16;
const BALL_MARKER_LIFT: f32 = 0.025;
const BALL_BEAM_HEIGHT: f32 = 2.8;
const BALL_ABOVE_CARRIER: f32 = 1.55;
const HEAD_RADIUS: f32 = 0.24;
const LYING_LIFT: f32 = 0.24;
const LYING_SHIFT: f32 = 0.5;
const LABEL_ABOVE_FEET: f32 = 1.45;
const LABEL_WIDTH: f32 = 56.0;
const LABEL_HEIGHT: f32 = 36.0;
const LABEL_COLOUR: Color = Color::srgb(0.97, 0.96, 0.92);
const PIP_SIZE: f32 = 7.0;
const PIPS_PER_ROW: u8 = 4;
const PIP_FULL: Color = Color::srgb(0.95, 0.2, 0.25);
const PIP_LOST: Color = Color::srgba(0.15, 0.12, 0.12, 0.8);
const CATCH_UP_BEATS: usize = 12;
const CATCH_UP_PACE: f32 = 3.0;
const ROLL_HOLD_SECONDS: f32 = 0.9;
const FOOT_PLATE_LIFT: f32 = 0.016;

/// Where a figure should be and whether it lies on the ground.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FigurePose {
    /// Feet position on the floor.
    pub position: Vec3,
    /// Whether the goblin lies down.
    pub lying: bool,
}

/// How the floor under a goblin is lit, so the coach sees at a glance who can still play.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Footing {
    /// Not its team's turn, or not on the bowl: the floor stays as it is.
    Unlit,
    /// It can still walk or act this turn: a bright tile.
    Ready,
    /// It is done for this turn: a dark tile.
    Done,
}

/// The footing of `goblin` while `active_team` plays.
pub fn footing(goblin: &Goblin, active_team: Option<TeamId>) -> Footing {
    if Some(goblin.team()) != active_team || goblin.presence == Presence::KnockedOut {
        Footing::Unlit
    } else if goblin.can_act() {
        Footing::Ready
    } else {
        Footing::Done
    }
}

/// Whether the bowl is still playing out what happened: steps, falls, rolls and flights.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BowlAnimation {
    /// Whether beats are still waiting to be shown.
    pub busy: bool,
}

/// A goblin figure on the bowl.
#[derive(Component, Clone, Copy, Debug)]
pub struct GoblinFigure {
    /// The goblin it shows.
    pub id: GoblinId,
}

/// Beats still to show, and what the ball last came to rest on.
#[derive(Resource, Default)]
struct Choreography {
    beats: VecDeque<Beat>,
    ball_rests_on: Option<BallAnchor>,
    front_held_for: f32,
    front_announced: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BallAnchor {
    Square(Square),
    Goblin(GoblinId),
}

type FigureParts<'w, 's> = Query<
    'w,
    's,
    (
        &'static GoblinFigure,
        &'static mut Transform,
        &'static mut Visibility,
        &'static Children,
    ),
    Without<BallFigure>,
>;
type BodyParts<'w, 's> = Query<
    'w,
    's,
    (&'static mut FigureBody, &'static mut Transform),
    (Without<GoblinFigure>, Without<BallFigure>),
>;
type BallPlacement<'w, 's> = Single<
    'w,
    's,
    &'static mut Transform,
    (With<BallFigure>, Without<GoblinFigure>, Without<FigureBody>),
>;

#[derive(Component, Clone, Copy, Debug)]
struct FigureBody {
    facing: Quat,
    lie: f32,
}

#[derive(Component)]
struct ReadyRing;

#[derive(Component)]
struct StanceRing;

#[derive(Component)]
struct FootPlate;

#[derive(Component)]
struct BallFigure;

#[derive(Component)]
struct BallMarker;

#[derive(Component)]
struct FigureLabel(Entity);

#[derive(Component)]
struct HealthPip {
    goblin: GoblinId,
    index: u8,
}

/// Coat colour of a team, a darker shade of its den: dokkaebi blue for home, leprechaun gold for away.
pub fn team_colour(team: TeamId) -> Color {
    if team == TeamId::HOME {
        Color::srgb(0.13, 0.25, 0.72)
    } else {
        Color::srgb(0.86, 0.56, 0.08)
    }
}

/// Rotation turning a figure of `team` towards the den it attacks.
pub fn facing(team: TeamId) -> Quat {
    if team == TeamId::HOME {
        Quat::from_rotation_y(FRAC_PI_2)
    } else {
        Quat::from_rotation_y(-FRAC_PI_2)
    }
}

/// Where `goblin` should stand on `arena`, or None when it is knocked out.
pub fn figure_pose(arena: &Arena, goblin: &Goblin) -> Option<FigurePose> {
    match goblin.presence {
        Presence::Standing(square) => Some(FigurePose {
            position: square_centre(arena, square),
            lying: false,
        }),
        Presence::Down(square) => Some(FigurePose {
            position: square_centre(arena, square),
            lying: true,
        }),
        Presence::KnockedOut => None,
    }
}

/// Placement of a figure's body: upright at `lie` 0, fallen on its back and still on its square at 1.
pub fn body_transform(facing: Quat, lie: f32) -> Transform {
    Transform::from_translation(facing * Vec3::new(0.0, LYING_LIFT * lie, LYING_SHIFT * lie))
        .with_rotation(facing * Quat::from_rotation_x(-FRAC_PI_2 * lie))
}

/// Where the ball should be drawn: resting on its square, or above its carrier.
pub fn ball_position(arena: &Arena, state: &GameState) -> Option<Vec3> {
    match state.ball {
        Ball::Loose { at } => Some(square_centre(arena, at) + Vec3::Y * BALL_RADIUS),
        Ball::Carried { by } => state
            .goblin(by)
            .and_then(Goblin::square)
            .map(|square| square_centre(arena, square) + Vec3::Y * BALL_ABOVE_CARRIER),
    }
}

/// How many health pips each row of a label holds: four at most, the rows as even as can be,
/// so the pips of goblins standing side by side never run into each other.
pub fn pip_rows(health: u8) -> Vec<u8> {
    let rows = health.div_ceil(PIPS_PER_ROW).max(1);
    (0..rows)
        .map(|row| health / rows + u8::from(row < health % rows))
        .collect()
}

/// Colour of the health pip at `index` for a goblin with `health` left.
pub fn pip_colour(index: u8, health: u8) -> Color {
    if index < health { PIP_FULL } else { PIP_LOST }
}

/// Height above the feet where a goblin's label sits: just over its head, or over the ball it carries.
pub fn label_lift(carrying: bool) -> f32 {
    if carrying {
        BALL_ABOVE_CARRIER + 2.0 * BALL_RADIUS
    } else {
        LABEL_ABOVE_FEET
    }
}

/// How much faster beats play while `queued` of them wait, so the bowl never lags far behind the match.
pub fn pace(queued: usize) -> f32 {
    if queued > CATCH_UP_BEATS {
        CATCH_UP_PACE
    } else {
        1.0
    }
}

fn approach(current: f32, target: f32, max_step: f32) -> f32 {
    let gap = target - current;
    if gap.abs() <= max_step {
        target
    } else {
        current + max_step.copysign(gap)
    }
}

/// Draws goblins and the ball and keeps them in step with the match.
pub struct GoblinViewPlugin;

impl Plugin for GoblinViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<BoardClick>()
            .add_message::<RollShown>()
            .add_message::<MatchRestarted>()
            .init_resource::<Choreography>()
            .init_resource::<BowlAnimation>()
            .add_systems(Startup, (spawn_figures, spawn_ball))
            .add_systems(
                Update,
                (
                    forget_the_last_match,
                    queue_beats,
                    play_beats,
                    report_animation,
                    carry_ball,
                    refresh_rings,
                    refresh_foot_plates,
                    refresh_health_pips,
                    mark_loose_ball,
                )
                    .chain()
                    .in_set(MatchSystems::Present),
            )
            .add_systems(
                PostUpdate,
                place_labels
                    .after(TransformSystems::Propagate)
                    .before(UiSystems::Layout),
            );
    }
}

struct FigureKit {
    bodies: [Handle<Mesh>; 3],
    coats: [Handle<StandardMaterial>; 2],
    skins: [Handle<StandardMaterial>; 2],
    head: Handle<Mesh>,
    ear: Handle<Mesh>,
    eye: Handle<Mesh>,
    horn: Handle<Mesh>,
    hat_crown: Handle<Mesh>,
    hat_brim: Handle<Mesh>,
    beard: Handle<Mesh>,
    club: Handle<Mesh>,
    satchel: Handle<Mesh>,
    ready_ring: Handle<Mesh>,
    stance_ring: Handle<Mesh>,
    ivory: Handle<StandardMaterial>,
    dark: Handle<StandardMaterial>,
    hat: Handle<StandardMaterial>,
    ginger: Handle<StandardMaterial>,
    leather: Handle<StandardMaterial>,
    ready_glow: Handle<StandardMaterial>,
    stance_glow: Handle<StandardMaterial>,
    foot_plate: Handle<Mesh>,
    ready_floor: Handle<StandardMaterial>,
    done_floor: Handle<StandardMaterial>,
    label_font: Handle<Font>,
}

#[derive(Resource)]
struct FloorLooks {
    ready: Handle<StandardMaterial>,
    done: Handle<StandardMaterial>,
}

struct BodyShape {
    radius: f32,
    length: f32,
    head_height: f32,
}

fn body_shape(archetype: Archetype) -> BodyShape {
    match archetype {
        Archetype::Bruiser => BodyShape {
            radius: 0.3,
            length: 0.2,
            head_height: 0.95,
        },
        Archetype::Sprinter => BodyShape {
            radius: 0.19,
            length: 0.32,
            head_height: 0.86,
        },
        Archetype::Thrower => BodyShape {
            radius: 0.23,
            length: 0.24,
            head_height: 0.86,
        },
    }
}

fn archetype_index(archetype: Archetype) -> usize {
    match archetype {
        Archetype::Bruiser => 0,
        Archetype::Sprinter => 1,
        Archetype::Thrower => 2,
    }
}

fn faceted(sphere: Sphere) -> Mesh {
    sphere
        .mesh()
        .ico(1)
        .unwrap_or_else(|_| sphere.mesh().uv(10, 8))
        .with_duplicated_vertices()
        .with_computed_flat_normals()
}

fn glowing(colour: Color) -> StandardMaterial {
    StandardMaterial {
        base_color: colour,
        unlit: true,
        ..default()
    }
}

fn see_through(colour: Color) -> StandardMaterial {
    StandardMaterial {
        alpha_mode: AlphaMode::Blend,
        ..glowing(colour)
    }
}

fn figure_kit(
    assets: &AssetServer,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> FigureKit {
    let body = |archetype| {
        let shape = body_shape(archetype);
        Capsule3d::new(shape.radius, shape.length)
    };
    FigureKit {
        bodies: [
            meshes.add(body(Archetype::Bruiser)),
            meshes.add(body(Archetype::Sprinter)),
            meshes.add(body(Archetype::Thrower)),
        ],
        coats: [
            materials.add(team_colour(TeamId::HOME)),
            materials.add(team_colour(TeamId::AWAY)),
        ],
        skins: [
            materials.add(Color::srgb(0.86, 0.38, 0.3)),
            materials.add(Color::srgb(0.5, 0.68, 0.28)),
        ],
        head: meshes.add(faceted(Sphere::new(HEAD_RADIUS))),
        ear: meshes.add(Cone::new(0.07, 0.24)),
        eye: meshes.add(Sphere::new(0.04)),
        horn: meshes.add(Cone::new(0.06, 0.2)),
        hat_crown: meshes.add(Cylinder::new(0.15, 0.22)),
        hat_brim: meshes.add(Cylinder::new(0.28, 0.03)),
        beard: meshes.add(Cone::new(0.12, 0.2)),
        club: meshes.add(Cylinder::new(0.055, 0.5)),
        satchel: meshes.add(Cuboid::new(0.22, 0.18, 0.1)),
        ready_ring: meshes.add(Torus::new(0.36, 0.41)),
        stance_ring: meshes.add(Torus::new(0.42, 0.48)),
        ivory: materials.add(Color::srgb(0.95, 0.93, 0.84)),
        dark: materials.add(Color::srgb(0.06, 0.05, 0.05)),
        hat: materials.add(Color::srgb(0.07, 0.33, 0.15)),
        ginger: materials.add(Color::srgb(0.9, 0.42, 0.12)),
        leather: materials.add(Color::srgb(0.45, 0.28, 0.14)),
        ready_glow: materials.add(glowing(Color::srgb(0.35, 1.0, 0.45))),
        stance_glow: materials.add(glowing(Color::srgb(0.9, 0.95, 1.0))),
        foot_plate: meshes.add(Cuboid::new(0.94, 0.01, 0.94)),
        ready_floor: materials.add(see_through(Color::srgba(0.3, 0.95, 0.4, 0.85))),
        done_floor: materials.add(see_through(Color::srgba(0.1, 0.1, 0.12, 0.85))),
        label_font: assets.load("fonts/Andika-Bold.ttf"),
    }
}

fn spawn_figures(
    mut commands: Commands,
    view: Res<MatchView>,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let kit = figure_kit(&assets, &mut meshes, &mut materials);
    for goblin in &view.snapshot.state.goblins {
        let figure = spawn_figure(&mut commands, &kit, &view.arena, goblin);
        spawn_label(&mut commands, &kit, goblin, figure);
    }
    commands.insert_resource(FloorLooks {
        ready: kit.ready_floor.clone(),
        done: kit.done_floor.clone(),
    });
}

fn spawn_figure(
    commands: &mut Commands,
    kit: &FigureKit,
    arena: &Arena,
    goblin: &Goblin,
) -> Entity {
    let team = goblin.team();
    let side = team.index().min(1);
    let shape = body_shape(goblin.archetype);
    let pose = figure_pose(arena, goblin);
    let turned_to = facing(team);
    let lie = if pose.is_some_and(|pose| pose.lying) {
        1.0
    } else {
        0.0
    };
    commands
        .spawn((
            GoblinFigure { id: goblin.id },
            Hovered::default(),
            Transform::from_translation(pose.map_or(Vec3::ZERO, |pose| pose.position)),
            if pose.is_some() {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            },
        ))
        .observe(click_figure)
        .with_children(|figure| {
            figure
                .spawn((
                    FigureBody {
                        facing: turned_to,
                        lie,
                    },
                    body_transform(turned_to, lie),
                    Visibility::Inherited,
                ))
                .with_children(|body| {
                    spawn_body_parts(body, kit, goblin.archetype, team, side, &shape);
                });
            figure.spawn((
                Mesh3d(kit.ready_ring.clone()),
                MeshMaterial3d(kit.ready_glow.clone()),
                Transform::from_xyz(0.0, 0.02, 0.0),
                Visibility::Hidden,
                Pickable::IGNORE,
                NotShadowCaster,
                ReadyRing,
            ));
            figure.spawn((
                Mesh3d(kit.stance_ring.clone()),
                MeshMaterial3d(kit.stance_glow.clone()),
                Transform::from_xyz(0.0, 0.03, 0.0),
                Visibility::Hidden,
                Pickable::IGNORE,
                NotShadowCaster,
                StanceRing,
            ));
            figure.spawn((
                Mesh3d(kit.foot_plate.clone()),
                MeshMaterial3d(kit.ready_floor.clone()),
                Transform::from_xyz(0.0, FOOT_PLATE_LIFT, 0.0),
                Visibility::Hidden,
                Pickable::IGNORE,
                NotShadowCaster,
                FootPlate,
            ));
        })
        .id()
}

fn click_figure(
    click: On<PointerClick>,
    figures: Query<&GoblinFigure>,
    mut clicks: MessageWriter<BoardClick>,
) {
    if click.button != PointerButton::Primary {
        return;
    }
    if let Ok(figure) = figures.get(click.event_target()) {
        clicks.write(BoardClick::Goblin(figure.id));
    }
}

fn spawn_body_parts(
    body: &mut ChildSpawnerCommands,
    kit: &FigureKit,
    archetype: Archetype,
    team: TeamId,
    side: usize,
    shape: &BodyShape,
) {
    let skin = &kit.skins[side];
    body.spawn((
        Mesh3d(kit.bodies[archetype_index(archetype)].clone()),
        MeshMaterial3d(kit.coats[side].clone()),
        Transform::from_xyz(0.0, shape.radius + shape.length / 2.0, 0.0),
    ));
    body.spawn((
        Mesh3d(kit.head.clone()),
        MeshMaterial3d(skin.clone()),
        Transform::from_xyz(0.0, shape.head_height, 0.0),
    ));
    for ear_side in [-1.0_f32, 1.0] {
        body.spawn((
            Mesh3d(kit.ear.clone()),
            MeshMaterial3d(skin.clone()),
            Transform::from_xyz(
                ear_side * (HEAD_RADIUS + 0.03),
                shape.head_height + 0.04,
                0.0,
            )
            .with_rotation(Quat::from_rotation_z(-ear_side * 1.2)),
        ));
        body.spawn((
            Mesh3d(kit.eye.clone()),
            MeshMaterial3d(kit.dark.clone()),
            Transform::from_xyz(
                ear_side * 0.09,
                shape.head_height + 0.05,
                HEAD_RADIUS - 0.03,
            ),
        ));
    }
    if team == TeamId::HOME {
        body.spawn((
            Mesh3d(kit.horn.clone()),
            MeshMaterial3d(kit.ivory.clone()),
            Transform::from_xyz(0.0, shape.head_height + HEAD_RADIUS + 0.06, 0.0),
        ));
    } else {
        body.spawn((
            Mesh3d(kit.hat_brim.clone()),
            MeshMaterial3d(kit.hat.clone()),
            Transform::from_xyz(0.0, shape.head_height + HEAD_RADIUS - 0.06, 0.0),
        ));
        body.spawn((
            Mesh3d(kit.hat_crown.clone()),
            MeshMaterial3d(kit.hat.clone()),
            Transform::from_xyz(0.0, shape.head_height + HEAD_RADIUS + 0.05, 0.0),
        ));
        body.spawn((
            Mesh3d(kit.beard.clone()),
            MeshMaterial3d(kit.ginger.clone()),
            Transform::from_xyz(0.0, shape.head_height - 0.2, 0.1)
                .with_rotation(Quat::from_rotation_x(std::f32::consts::PI)),
        ));
    }
    match archetype {
        Archetype::Bruiser => {
            body.spawn((
                Mesh3d(kit.club.clone()),
                MeshMaterial3d(kit.leather.clone()),
                Transform::from_xyz(-(shape.radius + 0.08), 0.42, 0.06)
                    .with_rotation(Quat::from_rotation_z(0.5)),
            ));
        }
        Archetype::Thrower => {
            body.spawn((
                Mesh3d(kit.satchel.clone()),
                MeshMaterial3d(kit.leather.clone()),
                Transform::from_xyz(0.0, 0.4, -(shape.radius + 0.04)),
            ));
        }
        Archetype::Sprinter => {}
    }
}

fn spawn_label(commands: &mut Commands, kit: &FigureKit, goblin: &Goblin, figure: Entity) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: px(LABEL_WIDTH),
                height: px(LABEL_HEIGHT),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::FlexEnd,
                align_items: AlignItems::Center,
                row_gap: px(1),
                ..default()
            },
            GlobalZIndex(-1),
            Visibility::Hidden,
            Pickable::IGNORE,
            FigureLabel(figure),
        ))
        .with_children(|label| {
            label.spawn((
                Text::new(goblin.id.to_string()),
                TextFont {
                    font: kit.label_font.clone().into(),
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(LABEL_COLOUR),
                TextShadow {
                    offset: Vec2::splat(1.0),
                    color: Color::srgba(0.0, 0.0, 0.0, 0.9),
                },
                Pickable::IGNORE,
            ));
            label
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: px(2),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .with_children(|rows| {
                    let counts = pip_rows(goblin.characteristics.health);
                    let firsts = counts.iter().scan(0, |next, count| {
                        let first = *next;
                        *next += count;
                        Some(first)
                    });
                    for (first, count) in firsts.zip(counts.iter().copied()) {
                        rows.spawn((
                            Node {
                                column_gap: px(2),
                                ..default()
                            },
                            Pickable::IGNORE,
                        ))
                        .with_children(|pips| {
                            for index in first..first + count {
                                pips.spawn((
                                    Node {
                                        width: px(PIP_SIZE),
                                        height: px(PIP_SIZE),
                                        border_radius: BorderRadius::all(px(PIP_SIZE / 2.0)),
                                        ..default()
                                    },
                                    BackgroundColor(pip_colour(index, goblin.health)),
                                    Pickable::IGNORE,
                                    HealthPip {
                                        goblin: goblin.id,
                                        index,
                                    },
                                ));
                            }
                        });
                    }
                });
        });
}

fn anchor_of(ball: Ball) -> BallAnchor {
    match ball {
        Ball::Loose { at } => BallAnchor::Square(at),
        Ball::Carried { by } => BallAnchor::Goblin(by),
    }
}

fn spawn_ball(
    mut commands: Commands,
    view: Res<MatchView>,
    mut choreography: ResMut<Choreography>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    choreography.ball_rests_on = Some(anchor_of(view.snapshot.state.ball));
    commands.spawn((
        Mesh3d(meshes.add(faceted(Sphere::new(BALL_RADIUS)))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.98, 0.97, 0.94),
            emissive: LinearRgba::rgb(0.35, 0.33, 0.3),
            perceptual_roughness: 0.4,
            ..default()
        })),
        Transform::from_translation(
            ball_position(&view.arena, &view.snapshot.state).unwrap_or_default(),
        ),
        Pickable::IGNORE,
        BallFigure,
    ));
    let beam_colour = Color::srgba(1.0, 0.9, 0.5, 0.5);
    commands
        .spawn((
            Mesh3d(meshes.add(Torus::new(0.22, 0.3))),
            MeshMaterial3d(materials.add(glowing(Color::srgb(1.0, 0.95, 0.7)))),
            Transform::default(),
            Visibility::Hidden,
            Pickable::IGNORE,
            NotShadowCaster,
            BallMarker,
        ))
        .with_child((
            Mesh3d(meshes.add(Cylinder::new(0.1, BALL_BEAM_HEIGHT))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: beam_colour,
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            })),
            Transform::from_xyz(0.0, BALL_BEAM_HEIGHT / 2.0, 0.0),
            Pickable::IGNORE,
            NotShadowCaster,
        ));
}

fn forget_the_last_match(
    mut restarts: MessageReader<MatchRestarted>,
    mut choreography: ResMut<Choreography>,
) {
    if restarts.read().count() > 0 {
        *choreography = Choreography::default();
    }
}

fn queue_beats(view: Res<MatchView>, mut choreography: ResMut<Choreography>) {
    if !view.fresh_events.is_empty() {
        let beats = beats_from_events(&view.fresh_events);
        choreography.beats.extend(beats);
    }
}

fn step(transform: &mut Mut<Transform>, target: Vec3, max_step: f32) -> bool {
    let stepped = transform.translation.move_towards(target, max_step);
    if stepped != transform.translation {
        transform.translation = stepped;
    }
    stepped == target
}

fn turn_bodies(children: &Children, bodies: &mut BodyParts, lying: bool, max_step: f32) -> bool {
    let target = if lying { 1.0 } else { 0.0 };
    let mut turned = true;
    for child in children.iter() {
        let Ok((mut body, mut placement)) = bodies.get_mut(child) else {
            continue;
        };
        let lie = approach(body.lie, target, max_step);
        if lie != body.lie {
            body.lie = lie;
            *placement = body_transform(body.facing, lie);
        }
        turned &= lie == target;
    }
    turned
}

fn lie_down(
    goblin: GoblinId,
    lying: bool,
    figures: &FigureParts,
    bodies: &mut BodyParts,
    max_step: f32,
) -> bool {
    figures
        .iter()
        .find(|(figure, ..)| figure.id == goblin)
        .is_none_or(|(_, _, _, children)| turn_bodies(children, bodies, lying, max_step))
}

fn settle_figures(
    view: &MatchView,
    figures: &mut FigureParts,
    bodies: &mut BodyParts,
    walk: f32,
    fall: f32,
) -> bool {
    let mut settled = true;
    for (figure, mut transform, mut visibility, children) in figures.iter_mut() {
        let pose = view
            .snapshot
            .state
            .goblin(figure.id)
            .and_then(|goblin| figure_pose(&view.arena, goblin));
        let Some(pose) = pose else {
            visibility.set_if_neq(Visibility::Hidden);
            continue;
        };
        if *visibility == Visibility::Hidden {
            *visibility = Visibility::Inherited;
            transform.translation = pose.position;
        }
        let arrived = step(&mut transform, pose.position, walk);
        let turned = turn_bodies(children, bodies, pose.lying, fall);
        settled &= arrived && turned;
    }
    settled
}

fn play_beats(
    time: Res<Time>,
    view: Res<MatchView>,
    mut choreography: ResMut<Choreography>,
    mut figures: FigureParts,
    mut bodies: BodyParts,
    mut ball: BallPlacement,
    mut rolls: MessageWriter<RollShown>,
) {
    let speed_up = pace(choreography.beats.len());
    let walk = WALK_SPEED * speed_up * time.delta_secs();
    let fall = FALL_SPEED * speed_up * time.delta_secs();
    let fly = BALL_SPEED * speed_up * time.delta_secs();
    let Some(beat) = choreography.beats.front().copied() else {
        settle_figures(&view, &mut figures, &mut bodies, walk, fall);
        return;
    };
    let finished = match beat {
        Beat::Step { goblin, to } => {
            let target = square_centre(&view.arena, to);
            figures
                .iter_mut()
                .find(|(figure, ..)| figure.id == goblin)
                .is_none_or(|(_, mut transform, ..)| step(&mut transform, target, walk))
        }
        Beat::Fall { goblin } => lie_down(goblin, true, &figures, &mut bodies, fall),
        Beat::Rise { goblin } => lie_down(goblin, false, &figures, &mut bodies, fall),
        Beat::Vanish { goblin } => {
            if let Some((_, _, mut visibility, _)) =
                figures.iter_mut().find(|(figure, ..)| figure.id == goblin)
            {
                visibility.set_if_neq(Visibility::Hidden);
            }
            true
        }
        Beat::Roll { goblin, roll } => {
            if !choreography.front_announced {
                choreography.front_announced = true;
                rolls.write(RollShown { goblin, roll });
            }
            choreography.front_held_for += speed_up * time.delta_secs();
            choreography.front_held_for >= ROLL_HOLD_SECONDS
        }
        Beat::LineUp => settle_figures(&view, &mut figures, &mut bodies, walk, fall),
        Beat::BallTo { square } => {
            let resting = square_centre(&view.arena, square) + Vec3::Y * BALL_RADIUS;
            let arrived = step(&mut ball, resting, fly);
            if arrived {
                choreography.ball_rests_on = Some(BallAnchor::Square(square));
            }
            arrived
        }
        Beat::BallToGoblin { goblin } => {
            let hands = figures
                .iter()
                .find(|(figure, ..)| figure.id == goblin)
                .map(|(_, transform, ..)| transform.translation + Vec3::Y * BALL_ABOVE_CARRIER);
            let arrived = hands.is_none_or(|hands| step(&mut ball, hands, fly));
            if arrived {
                choreography.ball_rests_on = Some(BallAnchor::Goblin(goblin));
            }
            arrived
        }
    };
    if finished {
        choreography.beats.pop_front();
        choreography.front_held_for = 0.0;
        choreography.front_announced = false;
    }
}

fn report_animation(choreography: Res<Choreography>, mut animation: ResMut<BowlAnimation>) {
    animation.set_if_neq(BowlAnimation {
        busy: !choreography.beats.is_empty(),
    });
}

fn ball_in_flight(choreography: &Choreography) -> bool {
    matches!(
        choreography.beats.front(),
        Some(Beat::BallTo { .. } | Beat::BallToGoblin { .. })
    )
}

fn carry_ball(
    time: Res<Time>,
    view: Res<MatchView>,
    mut choreography: ResMut<Choreography>,
    figures: Query<(&GoblinFigure, &Transform), Without<BallFigure>>,
    mut ball: Single<&mut Transform, With<BallFigure>>,
) {
    if ball_in_flight(&choreography) {
        return;
    }
    if choreography.beats.is_empty() {
        choreography.ball_rests_on = Some(anchor_of(view.snapshot.state.ball));
    }
    let target = match choreography.ball_rests_on {
        Some(BallAnchor::Square(square)) => {
            Some(square_centre(&view.arena, square) + Vec3::Y * BALL_RADIUS)
        }
        Some(BallAnchor::Goblin(goblin)) => figures
            .iter()
            .find(|(figure, _)| figure.id == goblin)
            .map(|(_, carrier)| carrier.translation + Vec3::Y * BALL_ABOVE_CARRIER),
        None => None,
    };
    if let Some(target) = target {
        let speed_up = pace(choreography.beats.len());
        step(&mut ball, target, BALL_SPEED * speed_up * time.delta_secs());
    }
}

fn mark_loose_ball(
    view: Res<MatchView>,
    choreography: Res<Choreography>,
    mut marker: Single<(&mut Transform, &mut Visibility), With<BallMarker>>,
) {
    let (transform, visibility) = &mut *marker;
    match choreography.ball_rests_on {
        Some(BallAnchor::Square(square)) if !ball_in_flight(&choreography) => {
            let floor = square_centre(&view.arena, square) + Vec3::Y * BALL_MARKER_LIFT;
            if transform.translation != floor {
                transform.translation = floor;
            }
            visibility.set_if_neq(Visibility::Inherited);
        }
        _ => {
            visibility.set_if_neq(Visibility::Hidden);
        }
    }
}

fn refresh_foot_plates(
    view: Res<MatchView>,
    looks: Option<Res<FloorLooks>>,
    figures: Query<(&GoblinFigure, &Children)>,
    mut plates: Query<(&mut Visibility, &mut MeshMaterial3d<StandardMaterial>), With<FootPlate>>,
) {
    let Some(looks) = looks else {
        return;
    };
    let active_team = view.snapshot.state.active_team();
    for (figure, children) in &figures {
        let Some(goblin) = view.snapshot.state.goblin(figure.id) else {
            continue;
        };
        let look = match footing(goblin, active_team) {
            Footing::Unlit => None,
            Footing::Ready => Some(&looks.ready),
            Footing::Done => Some(&looks.done),
        };
        for child in children.iter() {
            let Ok((mut visibility, mut material)) = plates.get_mut(child) else {
                continue;
            };
            match look {
                Some(look) => {
                    visibility.set_if_neq(Visibility::Inherited);
                    if material.0 != *look {
                        material.0 = look.clone();
                    }
                }
                None => {
                    visibility.set_if_neq(Visibility::Hidden);
                }
            }
        }
    }
}

fn refresh_rings(
    view: Res<MatchView>,
    figures: Query<(&GoblinFigure, &Children)>,
    mut ready_rings: Query<&mut Visibility, (With<ReadyRing>, Without<StanceRing>)>,
    mut stance_rings: Query<&mut Visibility, (With<StanceRing>, Without<ReadyRing>)>,
) {
    let active_team = view.snapshot.state.active_team();
    let shown = |visible: bool| {
        if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        }
    };
    for (figure, children) in &figures {
        let Some(goblin) = view.snapshot.state.goblin(figure.id) else {
            continue;
        };
        let may_act = goblin.can_act() && Some(goblin.team()) == active_team;
        for child in children.iter() {
            if let Ok(mut visibility) = ready_rings.get_mut(child) {
                visibility.set_if_neq(shown(may_act));
            }
            if let Ok(mut visibility) = stance_rings.get_mut(child) {
                visibility.set_if_neq(shown(goblin.in_stance));
            }
        }
    }
}

fn refresh_health_pips(view: Res<MatchView>, mut pips: Query<(&HealthPip, &mut BackgroundColor)>) {
    for (pip, mut background) in &mut pips {
        let health = view
            .snapshot
            .state
            .goblin(pip.goblin)
            .map_or(0, |goblin| goblin.health);
        background.set_if_neq(BackgroundColor(pip_colour(pip.index, health)));
    }
}

fn place_labels(
    view: Res<MatchView>,
    camera: Single<(&Camera, &GlobalTransform)>,
    figures: Query<(&GoblinFigure, &GlobalTransform, &Visibility)>,
    mut labels: Query<(&FigureLabel, &mut Node, &mut Visibility), Without<GoblinFigure>>,
) {
    let (camera, camera_transform) = *camera;
    for (label, mut node, mut visibility) in &mut labels {
        let Ok((figure, placement, figure_visibility)) = figures.get(label.0) else {
            continue;
        };
        let carrying = view.snapshot.state.carrier() == Some(figure.id);
        let above_head = placement.translation() + Vec3::Y * label_lift(carrying);
        match camera.world_to_viewport(camera_transform, above_head) {
            Ok(position) if *figure_visibility != Visibility::Hidden => {
                node.left = px(position.x - LABEL_WIDTH / 2.0);
                node.top = px(position.y - LABEL_HEIGHT);
                visibility.set_if_neq(Visibility::Inherited);
            }
            _ => {
                visibility.set_if_neq(Visibility::Hidden);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use goblins_rules::host::{MatchHost, MatchSetup, SeatKind};
    use goblins_rules::roster::Roster;
    use goblins_rules::ruleset::Ruleset;
    use goblins_rules::square::Square;

    use crate::test_support::{click_on, written};

    fn app_with_a_figure(goblin: GoblinId) -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_message::<BoardClick>();
        app.world_mut().register_component::<Window>();
        let body_part = app.world_mut().spawn_empty().id();
        app.world_mut()
            .spawn(GoblinFigure { id: goblin })
            .observe(click_figure)
            .add_child(body_part);
        (app, body_part)
    }

    #[test]
    fn given_a_figure_when_a_body_part_is_clicked_then_its_goblin_is_clicked() {
        let goblin = GoblinId::new(TeamId::HOME, 3);
        let (mut app, body_part) = app_with_a_figure(goblin);
        app.world_mut()
            .trigger(click_on(body_part, PointerButton::Primary));
        assert_eq!(
            written::<BoardClick>(&app),
            vec![BoardClick::Goblin(goblin)]
        );
    }

    #[test]
    fn given_a_figure_when_right_clicked_then_nothing_is_clicked() {
        let (mut app, body_part) = app_with_a_figure(GoblinId::new(TeamId::HOME, 3));
        app.world_mut()
            .trigger(click_on(body_part, PointerButton::Secondary));
        assert_eq!(written::<BoardClick>(&app), Vec::new());
    }

    fn goblin_of_home(state: &GameState) -> Goblin {
        state
            .goblin(GoblinId::new(TeamId::HOME, 5))
            .unwrap()
            .clone()
    }

    #[test]
    fn given_a_goblin_that_can_still_act_in_its_turn_when_lighting_its_floor_then_ready() {
        let goblin = goblin_of_home(&kicked_off());
        assert_eq!(footing(&goblin, Some(TeamId::HOME)), Footing::Ready);
    }

    #[test]
    fn given_a_goblin_done_for_its_turn_when_lighting_its_floor_then_done() {
        let mut goblin = goblin_of_home(&kicked_off());
        goblin.spend_its_turn();
        assert_eq!(footing(&goblin, Some(TeamId::HOME)), Footing::Done);
    }

    #[test]
    fn given_a_goblin_lying_down_in_its_turn_when_lighting_its_floor_then_done() {
        let mut goblin = goblin_of_home(&kicked_off());
        goblin.presence = Presence::Down(Square::new(7, 7));
        assert_eq!(footing(&goblin, Some(TeamId::HOME)), Footing::Done);
    }

    #[test]
    fn given_a_goblin_outside_its_turn_when_lighting_its_floor_then_unlit() {
        let goblin = goblin_of_home(&kicked_off());
        assert_eq!(footing(&goblin, Some(TeamId::AWAY)), Footing::Unlit);
    }

    #[test]
    fn given_a_knocked_out_goblin_in_its_turn_when_lighting_its_floor_then_unlit() {
        let mut goblin = goblin_of_home(&kicked_off());
        goblin.presence = Presence::KnockedOut;
        assert_eq!(footing(&goblin, Some(TeamId::HOME)), Footing::Unlit);
    }

    fn kicked_off() -> GameState {
        MatchHost::start(MatchSetup {
            ruleset: Ruleset::default(),
            rosters: [Roster::DEFAULT, Roster::DEFAULT],
            seed: 42,
            seats: [SeatKind::Human, SeatKind::Human],
            turn_seconds: [60, 60],
            agent_ready_grace_seconds: 15,
        })
        .unwrap()
        .0
        .snapshot()
        .state
    }

    fn goblin_with(presence: Presence) -> Goblin {
        let mut goblin = kicked_off().goblins[0].clone();
        goblin.presence = presence;
        goblin
    }

    #[test]
    fn given_the_two_teams_when_dressing_them_then_their_coats_differ() {
        assert_ne!(team_colour(TeamId::HOME), team_colour(TeamId::AWAY));
    }

    #[test]
    fn given_the_home_team_when_facing_then_it_looks_east_towards_the_away_den() {
        assert!((facing(TeamId::HOME) * Vec3::Z).abs_diff_eq(Vec3::X, 1e-6));
    }

    #[test]
    fn given_the_away_team_when_facing_then_it_looks_west_towards_the_home_den() {
        assert!((facing(TeamId::AWAY) * Vec3::Z).abs_diff_eq(Vec3::NEG_X, 1e-6));
    }

    #[test]
    fn given_a_standing_goblin_when_posing_it_then_upright_on_its_square() {
        let arena = Arena::bowl_15();
        let square = Square::new(3, 7);
        assert_eq!(
            figure_pose(&arena, &goblin_with(Presence::Standing(square))),
            Some(FigurePose {
                position: square_centre(&arena, square),
                lying: false
            })
        );
    }

    #[test]
    fn given_a_downed_goblin_when_posing_it_then_lying() {
        let pose = figure_pose(
            &Arena::bowl_15(),
            &goblin_with(Presence::Down(Square::new(3, 7))),
        );
        assert_eq!(pose.map(|pose| pose.lying), Some(true));
    }

    #[test]
    fn given_a_knocked_out_goblin_when_posing_it_then_nowhere() {
        assert_eq!(
            figure_pose(&Arena::bowl_15(), &goblin_with(Presence::KnockedOut)),
            None
        );
    }

    #[test]
    fn given_an_upright_body_when_placing_it_then_only_turned_towards_its_target() {
        let turned_to = facing(TeamId::HOME);
        assert_eq!(
            body_transform(turned_to, 0.0),
            Transform::from_rotation(turned_to)
        );
    }

    #[test]
    fn given_a_fallen_bruiser_when_placing_its_head_then_it_stays_on_its_square() {
        let head_height = body_shape(Archetype::Bruiser).head_height;
        let head = body_transform(facing(TeamId::AWAY), 1.0).transform_point(Vec3::Y * head_height);
        assert!(head.x.abs().max(head.z.abs()) < 0.5);
    }

    #[test]
    fn given_a_fallen_body_when_placing_it_then_its_back_rests_above_the_floor() {
        assert!(body_transform(facing(TeamId::HOME), 1.0).translation.y > 0.0);
    }

    #[test]
    fn given_a_loose_ball_when_placing_it_then_resting_on_its_square() {
        let arena = Arena::bowl_15();
        let mut state = kicked_off();
        let square = Square::new(9, 4);
        state.ball = Ball::Loose { at: square };
        assert_eq!(
            ball_position(&arena, &state),
            Some(square_centre(&arena, square) + Vec3::Y * BALL_RADIUS)
        );
    }

    #[test]
    fn given_a_carried_ball_when_placing_it_then_above_the_carrier() {
        let arena = Arena::bowl_15();
        let mut state = kicked_off();
        let carrier = state.goblins[2].clone();
        state.ball = Ball::Carried { by: carrier.id };
        let carrier_square = carrier.square().unwrap();
        assert_eq!(
            ball_position(&arena, &state),
            Some(square_centre(&arena, carrier_square) + Vec3::Y * BALL_ABOVE_CARRIER)
        );
    }

    #[test]
    fn given_seven_health_when_laying_out_its_pips_then_a_row_of_four_over_a_row_of_three() {
        assert_eq!(pip_rows(7), vec![4, 3]);
    }

    #[test]
    fn given_five_health_when_laying_out_its_pips_then_a_row_of_three_over_a_row_of_two() {
        assert_eq!(pip_rows(5), vec![3, 2]);
    }

    #[test]
    fn given_four_health_when_laying_out_its_pips_then_a_single_row() {
        assert_eq!(pip_rows(4), vec![4]);
    }

    #[test]
    fn given_two_health_left_when_colouring_the_second_pip_then_full() {
        assert_eq!(pip_colour(1, 2), PIP_FULL);
    }

    #[test]
    fn given_two_health_left_when_colouring_the_third_pip_then_lost() {
        assert_eq!(pip_colour(2, 2), PIP_LOST);
    }

    #[test]
    fn given_a_goblin_without_the_ball_when_lifting_its_label_then_lower_than_a_carrier_label() {
        assert!(label_lift(false) < label_lift(true));
    }

    #[test]
    fn given_the_carrier_when_lifting_its_label_then_above_the_ball() {
        assert!(label_lift(true) > BALL_ABOVE_CARRIER + BALL_RADIUS);
    }

    #[test]
    fn given_a_few_beats_waiting_when_pacing_then_normal_speed() {
        assert_eq!(pace(3), 1.0);
    }

    #[test]
    fn given_a_backlog_of_beats_when_pacing_then_faster() {
        assert!(pace(40) > 1.0);
    }

    #[test]
    fn given_a_far_target_when_approaching_then_only_the_allowed_step() {
        assert_eq!(approach(0.0, 1.0, 0.25), 0.25);
    }

    #[test]
    fn given_a_near_target_when_approaching_then_exactly_on_it() {
        assert_eq!(approach(0.9, 1.0, 0.25), 1.0);
    }
}
