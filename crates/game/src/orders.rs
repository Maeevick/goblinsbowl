//! Orders given with the pointer: a first click aims, a second click on the same thing sends.

use std::cmp::Reverse;
use std::collections::HashMap;

use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy::ui_widgets::{Activate, observe};
use goblins_rules::arena::Arena;
use goblins_rules::command::{Action, Activation, Command};
use goblins_rules::host::{MatchSnapshot, TurnClock};
use goblins_rules::odds::Probability;
use goblins_rules::paths::{reachable_squares, safest_path};
use goblins_rules::preview::activation_odds;
use goblins_rules::roster::GoblinId;
use goblins_rules::ruleset::Ruleset;
use goblins_rules::square::Square;
use goblins_rules::state::{GameState, Goblin};
use goblins_rules::team::TeamId;
use goblins_rules::validation::validate_activation;

use crate::bowl_view::{BoardClick, square_centre};
use crate::goblin_view::GoblinFigure;
use crate::hud::{BOLD_FONT, READING_FONT, button, human_team_in_control, text_bundle};
use crate::local_match::{MatchOrder, MatchRestarted, MatchSystems, MatchView};
use crate::texts::Texts;

const MARK_LIFT: f32 = 0.012;
const RING_LIFT: f32 = 0.05;
const REACHABLE_ALPHA: f32 = 0.3;
const AIMED_ALPHA: f32 = 0.75;

/// An order being prepared for one goblin of the team in control.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Draft {
    /// The activation a second click on the aim would send.
    pub activation: Activation,
    /// What the coach aimed at, if anything yet.
    pub aim: Option<BoardClick>,
}

impl Draft {
    /// A freshly selected goblin with nothing planned yet.
    pub fn selecting(goblin: GoblinId) -> Self {
        Self {
            activation: Activation {
                goblin,
                path: Vec::new(),
                action: None,
            },
            aim: None,
        }
    }
}

/// What a click does to the order being prepared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reaction {
    /// Nothing changes.
    Ignore,
    /// The order being prepared becomes this draft.
    Prepare(Draft),
    /// The selected goblin is let go.
    Release,
    /// This activation goes to the match.
    Send(Activation),
}

/// How likely a planned move or action is to succeed, as shown by the colour of its squares.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Risk {
    /// No die can fail it.
    Safe,
    /// Two chances in three or better.
    Likely,
    /// One chance in three or better.
    Even,
    /// Less than one chance in three.
    Unlikely,
}

/// How a square of the bowl is marked while an order is prepared.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SquareMark {
    /// Risk of getting there, or of the planned action on it.
    pub risk: Risk,
    /// Whether the square belongs to the aimed order rather than merely being reachable.
    pub aimed: bool,
}

/// The human team whose clock is running, the only one whose clicks become orders.
pub fn coach_giving_orders(snapshot: &MatchSnapshot) -> Option<TeamId> {
    human_team_in_control(snapshot).filter(|_| matches!(snapshot.clock, TurnClock::Running { .. }))
}

/// The draft still worth showing after the board changed: dropped once its goblin cannot act for
/// `team` any more, back to a plain selection when its aimed order became illegal.
pub fn keep_draft(
    arena: &Arena,
    ruleset: &Ruleset,
    state: &GameState,
    team: Option<TeamId>,
    draft: &Draft,
) -> Option<Draft> {
    let team = team?;
    let goblin = draft.activation.goblin;
    if !can_be_activated(state, team, goblin) {
        return None;
    }
    if draft.aim.is_some()
        && validate_activation(arena, ruleset, state, team, &draft.activation).is_err()
    {
        return Some(Draft::selecting(goblin));
    }
    Some(draft.clone())
}

/// Colour of a risk band, from green (no die can fail) to red (less than one chance in three).
pub fn risk_colour(risk: Risk) -> Color {
    match risk {
        Risk::Safe => Color::srgb(0.3, 0.85, 0.4),
        Risk::Likely => Color::srgb(0.95, 0.85, 0.25),
        Risk::Even => Color::srgb(1.0, 0.55, 0.15),
        Risk::Unlikely => Color::srgb(0.95, 0.2, 0.2),
    }
}

/// What `click` does for `team` while `draft` is being prepared.
pub fn react_to_click(
    arena: &Arena,
    ruleset: &Ruleset,
    state: &GameState,
    team: TeamId,
    draft: Option<&Draft>,
    click: BoardClick,
) -> Reaction {
    let click = match click {
        BoardClick::Square(square) => state
            .goblin_at(square)
            .map_or(click, |occupant| BoardClick::Goblin(occupant.id)),
        BoardClick::Goblin(_) => click,
    };
    let Some(draft) = draft else {
        return match click {
            BoardClick::Goblin(goblin) if can_be_activated(state, team, goblin) => {
                Reaction::Prepare(Draft::selecting(goblin))
            }
            _ => Reaction::Ignore,
        };
    };
    if draft.aim == Some(click) {
        return Reaction::Send(draft.activation.clone());
    }
    let selected = draft.activation.goblin;
    match click {
        BoardClick::Goblin(goblin) if goblin == selected => Reaction::Release,
        BoardClick::Goblin(goblin) => {
            let Some(clicked) = state.goblin(goblin) else {
                return Reaction::Ignore;
            };
            if clicked.team() != team {
                return aim_assault(arena, ruleset, state, team, draft, goblin);
            }
            let throw = Activation {
                goblin: selected,
                path: draft.activation.path.clone(),
                action: Some(Action::Throw { receiver: goblin }),
            };
            if validate_activation(arena, ruleset, state, team, &throw).is_ok() {
                Reaction::Prepare(Draft {
                    activation: throw,
                    aim: Some(click),
                })
            } else if can_be_activated(state, team, goblin) {
                Reaction::Prepare(Draft::selecting(goblin))
            } else {
                Reaction::Ignore
            }
        }
        BoardClick::Square(square) => {
            safest_path(arena, ruleset, state, selected, square).map_or(Reaction::Ignore, |path| {
                Reaction::Prepare(Draft {
                    activation: Activation {
                        goblin: selected,
                        path: path.squares,
                        action: None,
                    },
                    aim: Some(click),
                })
            })
        }
    }
}

fn can_be_activated(state: &GameState, team: TeamId, goblin: GoblinId) -> bool {
    state
        .goblin(goblin)
        .is_some_and(|goblin| goblin.team() == team && goblin.can_act())
}

fn aim_assault(
    arena: &Arena,
    ruleset: &Ruleset,
    state: &GameState,
    team: TeamId,
    draft: &Draft,
    target: GoblinId,
) -> Reaction {
    let selected = draft.activation.goblin;
    let start = state.goblin(selected).and_then(Goblin::square);
    let target_square = state.goblin(target).and_then(Goblin::square);
    let (Some(start), Some(target_square)) = (start, target_square) else {
        return Reaction::Ignore;
    };
    let planned_end = draft.activation.path.last().copied().unwrap_or(start);
    let path = if planned_end.is_orthogonally_adjacent(target_square) {
        Some(draft.activation.path.clone())
    } else if start.is_orthogonally_adjacent(target_square) {
        Some(Vec::new())
    } else {
        reachable_squares(arena, ruleset, state, selected)
            .into_iter()
            .filter(|path| path.destination.is_orthogonally_adjacent(target_square))
            .min_by_key(|path| {
                (
                    Reverse(path.success),
                    path.squares.len(),
                    path.squares.clone(),
                )
            })
            .map(|path| path.squares)
    };
    let Some(path) = path else {
        return Reaction::Ignore;
    };
    let activation = Activation {
        goblin: selected,
        path,
        action: Some(Action::Assault { target }),
    };
    if validate_activation(arena, ruleset, state, team, &activation).is_err() {
        return Reaction::Ignore;
    }
    Reaction::Prepare(Draft {
        activation,
        aim: Some(BoardClick::Goblin(target)),
    })
}

/// What is prepared once `reaction` happened: a goblin sent off stays selected, so it can keep
/// moving or act while its steps or its action are left.
pub fn prepared_after(reaction: Reaction, current: Option<Draft>) -> Option<Draft> {
    match reaction {
        Reaction::Ignore => current,
        Reaction::Prepare(draft) => Some(draft),
        Reaction::Release => None,
        Reaction::Send(activation) => Some(Draft::selecting(activation.goblin)),
    }
}

/// Whether `goblin` may still brace in Stance this turn, under rules that have one.
pub fn can_still_brace(ruleset: &Ruleset, state: &GameState, goblin: GoblinId) -> bool {
    ruleset.goblins_can_take_stance
        && state
            .goblin(goblin)
            .is_some_and(|goblin| goblin.is_standing() && goblin.has_action_left)
}

/// The drafted walk followed by a Stance.
pub fn stance_order(draft: &Draft) -> Activation {
    Activation {
        goblin: draft.activation.goblin,
        path: draft.activation.path.clone(),
        action: Some(Action::Stance),
    }
}

/// The risk band of a chance to succeed.
pub fn risk_of(success: Probability) -> Risk {
    if success == Probability::CERTAIN {
        Risk::Safe
    } else if success >= Probability::new(2, 3) {
        Risk::Likely
    } else if success >= Probability::new(1, 3) {
        Risk::Even
    } else {
        Risk::Unlikely
    }
}

/// Marks to draw while `draft` is prepared: every reachable square, and the aimed order on top.
pub fn square_marks(
    arena: &Arena,
    ruleset: &Ruleset,
    state: &GameState,
    team: TeamId,
    draft: &Draft,
) -> HashMap<Square, SquareMark> {
    let mut marks: HashMap<Square, SquareMark> =
        reachable_squares(arena, ruleset, state, draft.activation.goblin)
            .into_iter()
            .map(|path| {
                let mark = SquareMark {
                    risk: risk_of(path.success),
                    aimed: false,
                };
                (path.destination, mark)
            })
            .collect();
    if draft.aim.is_none() {
        return marks;
    }
    let Ok(odds) = activation_odds(arena, ruleset, state, team, &draft.activation) else {
        return marks;
    };
    let aimed = SquareMark {
        risk: risk_of(odds.overall_success),
        aimed: true,
    };
    let target_square = match draft.activation.action {
        Some(Action::Assault { target: other } | Action::Throw { receiver: other }) => {
            state.goblin(other).and_then(Goblin::square)
        }
        Some(Action::Stance) | None => None,
    };
    for square in draft.activation.path.iter().copied().chain(target_square) {
        marks.insert(square, aimed);
    }
    marks
}

/// The order the human coach in control is preparing, if any.
#[derive(Resource, Default, Clone, Debug, PartialEq, Eq)]
pub struct PreparedOrder(pub Option<Draft>);

#[derive(Component)]
struct MarkTile {
    square: Square,
}

#[derive(Component)]
struct SelectionRing;

#[derive(Component)]
struct OrderBar;

#[derive(Component)]
struct DraftLine;

#[derive(Component)]
struct DiceLine;

#[derive(Component)]
struct ConfirmLine;

#[derive(Component)]
struct StanceLabel;

#[derive(Component)]
struct StanceButton;

#[derive(Component)]
struct CancelLabel;

#[derive(Resource)]
struct MarkLooks(HashMap<SquareMark, Handle<StandardMaterial>>);

#[derive(Clone, Debug, PartialEq, Eq)]
struct MarksShownFor {
    version: u64,
    coach: Option<TeamId>,
    draft: Option<Draft>,
}

type DraftLineText<'w, 's> = Single<
    'w,
    's,
    (&'static mut Text, &'static mut TextColor),
    (With<DraftLine>, Without<DiceLine>, Without<ConfirmLine>),
>;
type DiceLineText<'w, 's> =
    Single<'w, 's, &'static mut Text, (With<DiceLine>, Without<DraftLine>, Without<ConfirmLine>)>;
type ConfirmLineText<'w, 's> =
    Single<'w, 's, &'static mut Text, (With<ConfirmLine>, Without<DraftLine>, Without<DiceLine>)>;

/// Turns clicks on the board into orders for the human team in control, showing reach and risk.
pub struct OrdersPlugin;

impl Plugin for OrdersPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PreparedOrder>()
            .add_message::<BoardClick>()
            .add_message::<MatchRestarted>()
            .add_systems(
                Startup,
                (spawn_marks, spawn_selection_ring, spawn_order_bar),
            )
            .add_systems(
                Update,
                (
                    forget_the_last_match,
                    react_to_board_clicks,
                    release_on_escape,
                    keep_prepared_order,
                    refresh_marks,
                    follow_selected_goblin,
                    refresh_order_bar,
                    refresh_stance_button,
                )
                    .chain()
                    .in_set(MatchSystems::Present),
            );
    }
}

fn spawn_marks(
    mut commands: Commands,
    view: Res<MatchView>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(Cuboid::new(0.92, 0.01, 0.92));
    let mut looks = HashMap::new();
    for risk in [Risk::Safe, Risk::Likely, Risk::Even, Risk::Unlikely] {
        for aimed in [false, true] {
            let alpha = if aimed { AIMED_ALPHA } else { REACHABLE_ALPHA };
            let look = materials.add(StandardMaterial {
                base_color: risk_colour(risk).with_alpha(alpha),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            });
            looks.insert(SquareMark { risk, aimed }, look);
        }
    }
    let unused_look = materials.add(StandardMaterial::default());
    for square in view.arena.in_bounds_squares() {
        commands.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(unused_look.clone()),
            Transform::from_translation(square_centre(&view.arena, square) + Vec3::Y * MARK_LIFT),
            Visibility::Hidden,
            Pickable::IGNORE,
            NotShadowCaster,
            MarkTile { square },
        ));
    }
    commands.insert_resource(MarkLooks(looks));
}

fn spawn_selection_ring(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Torus::new(0.44, 0.52))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.85, 0.3),
            unlit: true,
            ..default()
        })),
        Transform::default(),
        Visibility::Hidden,
        Pickable::IGNORE,
        NotShadowCaster,
        SelectionRing,
    ));
}

fn spawn_order_bar(mut commands: Commands, assets: Res<AssetServer>, texts: Res<Texts>) {
    let reading: Handle<Font> = assets.load(READING_FONT);
    let bold: Handle<Font> = assets.load(BOLD_FONT);
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: px(16),
            bottom: px(16),
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
            ..default()
        },
        Visibility::Hidden,
        Pickable::IGNORE,
        OrderBar,
        children![
            (text_bundle("", &bold, 24.0, Color::WHITE), DraftLine),
            (
                text_bundle("", &reading, 18.0, Color::srgb(0.92, 0.92, 0.88)),
                DiceLine
            ),
            (
                text_bundle("", &reading, 18.0, Color::srgb(1.0, 0.85, 0.45)),
                ConfirmLine
            ),
            (
                Node {
                    column_gap: px(12),
                    ..default()
                },
                Pickable::IGNORE,
                children![
                    (
                        button(texts.stance_button(), &bold, StanceLabel),
                        StanceButton,
                        observe(stance_pressed)
                    ),
                    (
                        button(texts.cancel_button(), &bold, CancelLabel),
                        observe(cancel_pressed)
                    ),
                ],
            ),
        ],
    ));
}

fn stance_pressed(
    _activate: On<Activate>,
    view: Res<MatchView>,
    mut prepared: ResMut<PreparedOrder>,
    mut orders: MessageWriter<MatchOrder>,
) {
    let Some(draft) = prepared.0.take() else {
        return;
    };
    if let Some(team) = coach_giving_orders(&view.snapshot) {
        let order = stance_order(&draft);
        prepared.0 = prepared_after(Reaction::Send(order.clone()), None);
        orders.write(MatchOrder::Command {
            team,
            command: Command::Act(order),
        });
    }
}

fn cancel_pressed(_activate: On<Activate>, mut prepared: ResMut<PreparedOrder>) {
    prepared.0 = None;
}

fn react_to_board_clicks(
    mut clicks: MessageReader<BoardClick>,
    view: Res<MatchView>,
    mut prepared: ResMut<PreparedOrder>,
    mut orders: MessageWriter<MatchOrder>,
) {
    let coach = coach_giving_orders(&view.snapshot);
    for click in clicks.read() {
        let Some(team) = coach else {
            continue;
        };
        let reaction = react_to_click(
            &view.arena,
            &view.ruleset,
            &view.snapshot.state,
            team,
            prepared.0.as_ref(),
            *click,
        );
        if let Reaction::Send(activation) = &reaction {
            orders.write(MatchOrder::Command {
                team,
                command: Command::Act(activation.clone()),
            });
        }
        prepared.0 = prepared_after(reaction, prepared.0.take());
    }
}

fn forget_the_last_match(
    mut restarts: MessageReader<MatchRestarted>,
    mut prepared: ResMut<PreparedOrder>,
) {
    if restarts.read().count() > 0 {
        prepared.0 = None;
    }
}

fn release_on_escape(keys: Res<ButtonInput<KeyCode>>, mut prepared: ResMut<PreparedOrder>) {
    if keys.just_pressed(KeyCode::Escape) && prepared.0.is_some() {
        prepared.0 = None;
    }
}

fn keep_prepared_order(view: Res<MatchView>, mut prepared: ResMut<PreparedOrder>) {
    let Some(draft) = &prepared.0 else {
        return;
    };
    let kept = keep_draft(
        &view.arena,
        &view.ruleset,
        &view.snapshot.state,
        coach_giving_orders(&view.snapshot),
        draft,
    );
    if kept.as_ref() != Some(draft) {
        prepared.0 = kept;
    }
}

fn refresh_marks(
    view: Res<MatchView>,
    prepared: Res<PreparedOrder>,
    looks: Option<Res<MarkLooks>>,
    mut shown_for: Local<Option<MarksShownFor>>,
    mut tiles: Query<(
        &MarkTile,
        &mut Visibility,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
) {
    let Some(looks) = looks else {
        return;
    };
    let coach = coach_giving_orders(&view.snapshot);
    let wanted = MarksShownFor {
        version: view.snapshot.version,
        coach,
        draft: prepared.0.clone(),
    };
    if shown_for.as_ref() == Some(&wanted) {
        return;
    }
    let marks = match (&prepared.0, coach) {
        (Some(draft), Some(team)) => square_marks(
            &view.arena,
            &view.ruleset,
            &view.snapshot.state,
            team,
            draft,
        ),
        _ => HashMap::new(),
    };
    for (tile, mut visibility, mut material) in &mut tiles {
        match marks.get(&tile.square).and_then(|mark| looks.0.get(mark)) {
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
    *shown_for = Some(wanted);
}

fn follow_selected_goblin(
    prepared: Res<PreparedOrder>,
    figures: Query<(&GoblinFigure, &Transform), Without<SelectionRing>>,
    mut ring: Single<(&mut Transform, &mut Visibility), With<SelectionRing>>,
) {
    let selected = prepared.0.as_ref().map(|draft| draft.activation.goblin);
    let feet = selected
        .and_then(|goblin| figures.iter().find(|(figure, _)| figure.id == goblin))
        .map(|(_, placement)| placement.translation);
    let (transform, visibility) = &mut *ring;
    match feet {
        Some(feet) => {
            transform.translation = feet + Vec3::Y * RING_LIFT;
            visibility.set_if_neq(Visibility::Inherited);
        }
        None => {
            visibility.set_if_neq(Visibility::Hidden);
        }
    }
}

fn refresh_stance_button(
    view: Res<MatchView>,
    prepared: Res<PreparedOrder>,
    mut stance_button: Single<&mut Visibility, With<StanceButton>>,
) {
    let braces = prepared.0.as_ref().is_some_and(|draft| {
        can_still_brace(&view.ruleset, &view.snapshot.state, draft.activation.goblin)
    });
    stance_button.set_if_neq(if braces {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    });
}

fn refresh_order_bar(
    view: Res<MatchView>,
    prepared: Res<PreparedOrder>,
    texts: Res<Texts>,
    mut bar: Single<&mut Visibility, With<OrderBar>>,
    mut draft_line: DraftLineText,
    mut dice_line: DiceLineText,
    mut confirm_line: ConfirmLineText,
) {
    let (Some(draft), Some(team)) = (&prepared.0, coach_giving_orders(&view.snapshot)) else {
        bar.set_if_neq(Visibility::Hidden);
        return;
    };
    bar.set_if_neq(Visibility::Inherited);
    let state = &view.snapshot.state;
    let odds = draft.aim.and_then(|_| {
        activation_odds(&view.arena, &view.ruleset, state, team, &draft.activation).ok()
    });
    let (line, colour, dice, hint) = match odds {
        Some(odds) => (
            texts.describe_draft(&draft.activation, &odds),
            risk_colour(risk_of(odds.overall_success)),
            texts.dice_detail(&odds),
            texts.confirm_hint().to_string(),
        ),
        None => (
            state
                .goblin(draft.activation.goblin)
                .map(|goblin| texts.selected_goblin(goblin.id, goblin.archetype))
                .unwrap_or_default(),
            Color::WHITE,
            String::new(),
            String::new(),
        ),
    };
    let (text, text_colour) = &mut *draft_line;
    if text.0 != line {
        text.0 = line;
    }
    text_colour.set_if_neq(TextColor(colour));
    if dice_line.0 != dice {
        dice_line.0 = dice;
    }
    if confirm_line.0 != hint {
        confirm_line.0 = hint;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use goblins_rules::host::{MatchHost, MatchSetup, SeatKind};
    use goblins_rules::roster::Roster;
    use goblins_rules::state::{Ball, Presence};

    fn id(text: &str) -> GoblinId {
        text.parse().unwrap()
    }

    fn square(x: u8, y: u8) -> Square {
        Square::new(x, y)
    }

    fn board(standing: &[(&str, u8, u8)], carrier: Option<&str>) -> GameState {
        let mut state = MatchHost::start(MatchSetup {
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
        .state;
        for goblin in &mut state.goblins {
            goblin.presence = Presence::KnockedOut;
            goblin.refresh_its_turn();
            goblin.in_stance = false;
        }
        for (name, x, y) in standing {
            let goblin = state
                .goblins
                .iter_mut()
                .find(|goblin| goblin.id == id(name))
                .unwrap();
            goblin.presence = Presence::Standing(square(*x, *y));
        }
        state.ball = match carrier {
            Some(name) => Ball::Carried { by: id(name) },
            None => Ball::Loose { at: square(7, 1) },
        };
        state
    }

    fn react(state: &GameState, draft: Option<&Draft>, click: BoardClick) -> Reaction {
        react_to_click(
            &Arena::bowl_15(),
            &Ruleset::default(),
            state,
            TeamId::HOME,
            draft,
            click,
        )
    }

    fn aimed(goblin: &str, path: &[(u8, u8)], action: Option<Action>, aim: BoardClick) -> Draft {
        Draft {
            activation: Activation {
                goblin: id(goblin),
                path: path.iter().map(|(x, y)| square(*x, *y)).collect(),
                action,
            },
            aim: Some(aim),
        }
    }

    fn started(seats: [SeatKind; 2]) -> MatchSnapshot {
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

    #[test]
    fn given_a_human_team_waiting_for_ready_when_asking_who_gives_orders_then_nobody() {
        assert_eq!(
            coach_giving_orders(&started([SeatKind::Human, SeatKind::Human])),
            None
        );
    }

    #[test]
    fn given_a_ready_human_team_when_asking_who_gives_orders_then_that_team() {
        let mut snapshot = started([SeatKind::Human, SeatKind::Human]);
        snapshot.clock = TurnClock::Running {
            remaining_ms: 30_000,
        };
        assert_eq!(coach_giving_orders(&snapshot), snapshot.state.active_team());
    }

    #[test]
    fn given_a_draft_for_a_goblin_that_acted_when_keeping_it_then_it_is_dropped() {
        let mut state = board(&[("H5", 7, 7)], None);
        state.goblins[4].spend_its_turn();
        assert_eq!(
            keep_draft(
                &Arena::bowl_15(),
                &Ruleset::default(),
                &state,
                Some(TeamId::HOME),
                &Draft::selecting(id("H5"))
            ),
            None
        );
    }

    #[test]
    fn given_an_aimed_walk_now_blocked_when_keeping_it_then_only_the_selection_remains() {
        let state = board(&[("H5", 7, 7), ("H6", 7, 8)], None);
        let walk = aimed(
            "H5",
            &[(7, 8), (7, 9)],
            None,
            BoardClick::Square(square(7, 9)),
        );
        assert_eq!(
            keep_draft(
                &Arena::bowl_15(),
                &Ruleset::default(),
                &state,
                Some(TeamId::HOME),
                &walk
            ),
            Some(Draft::selecting(id("H5")))
        );
    }

    #[test]
    fn given_an_aimed_walk_still_legal_when_keeping_it_then_it_stays() {
        let state = board(&[("H5", 7, 7)], None);
        let walk = aimed(
            "H5",
            &[(7, 8), (7, 9)],
            None,
            BoardClick::Square(square(7, 9)),
        );
        assert_eq!(
            keep_draft(
                &Arena::bowl_15(),
                &Ruleset::default(),
                &state,
                Some(TeamId::HOME),
                &walk
            ),
            Some(walk.clone())
        );
    }

    #[test]
    fn given_nothing_selected_when_clicking_an_own_goblin_then_it_is_selected() {
        let state = board(&[("H5", 7, 7)], None);
        assert_eq!(
            react(&state, None, BoardClick::Goblin(id("H5"))),
            Reaction::Prepare(Draft::selecting(id("H5")))
        );
    }

    #[test]
    fn given_nothing_selected_when_clicking_an_enemy_then_nothing_happens() {
        let state = board(&[("H5", 7, 7), ("A1", 10, 7)], None);
        assert_eq!(
            react(&state, None, BoardClick::Goblin(id("A1"))),
            Reaction::Ignore
        );
    }

    #[test]
    fn given_nothing_selected_when_clicking_a_goblin_that_already_acted_then_nothing_happens() {
        let mut state = board(&[("H5", 7, 7)], None);
        state.goblins[4].spend_its_turn();
        assert_eq!(
            react(&state, None, BoardClick::Goblin(id("H5"))),
            Reaction::Ignore
        );
    }

    #[test]
    fn given_a_selected_sprinter_when_clicking_a_reachable_square_then_its_walk_is_aimed() {
        let state = board(&[("H5", 7, 7)], None);
        let click = BoardClick::Square(square(7, 9));
        assert_eq!(
            react(&state, Some(&Draft::selecting(id("H5"))), click),
            Reaction::Prepare(aimed("H5", &[(7, 8), (7, 9)], None, click))
        );
    }

    #[test]
    fn given_an_aimed_walk_when_clicking_its_square_again_then_it_is_sent() {
        let state = board(&[("H5", 7, 7)], None);
        let click = BoardClick::Square(square(7, 9));
        let draft = aimed("H5", &[(7, 8), (7, 9)], None, click);
        assert_eq!(
            react(&state, Some(&draft), click),
            Reaction::Send(draft.activation.clone())
        );
    }

    #[test]
    fn given_a_selected_bruiser_when_clicking_a_square_out_of_reach_then_nothing_happens() {
        let state = board(&[("H1", 5, 7)], None);
        assert_eq!(
            react(
                &state,
                Some(&Draft::selecting(id("H1"))),
                BoardClick::Square(square(12, 7))
            ),
            Reaction::Ignore
        );
    }

    #[test]
    fn given_a_selected_goblin_when_clicking_it_again_then_it_is_released() {
        let state = board(&[("H5", 7, 7)], None);
        assert_eq!(
            react(
                &state,
                Some(&Draft::selecting(id("H5"))),
                BoardClick::Goblin(id("H5"))
            ),
            Reaction::Release
        );
    }

    #[test]
    fn given_a_selected_goblin_when_clicking_the_square_under_a_teammate_then_it_is_the_teammate() {
        let state = board(&[("H1", 5, 7), ("H5", 7, 7)], None);
        assert_eq!(
            react(
                &state,
                Some(&Draft::selecting(id("H1"))),
                BoardClick::Square(square(7, 7))
            ),
            Reaction::Prepare(Draft::selecting(id("H5")))
        );
    }

    #[test]
    fn given_a_selected_bruiser_next_to_an_enemy_when_clicking_it_then_an_assault_is_aimed() {
        let state = board(&[("H1", 7, 7), ("A1", 8, 7)], None);
        let click = BoardClick::Goblin(id("A1"));
        assert_eq!(
            react(&state, Some(&Draft::selecting(id("H1"))), click),
            Reaction::Prepare(aimed(
                "H1",
                &[],
                Some(Action::Assault { target: id("A1") }),
                click
            ))
        );
    }

    #[test]
    fn given_a_selected_bruiser_two_squares_from_an_enemy_when_clicking_it_then_it_walks_next_to_it_first()
     {
        let state = board(&[("H1", 5, 7), ("A1", 8, 7)], None);
        let click = BoardClick::Goblin(id("A1"));
        assert_eq!(
            react(&state, Some(&Draft::selecting(id("H1"))), click),
            Reaction::Prepare(aimed(
                "H1",
                &[(6, 7), (7, 7)],
                Some(Action::Assault { target: id("A1") }),
                click
            ))
        );
    }

    #[test]
    fn given_a_walk_ending_next_to_an_enemy_when_clicking_the_enemy_then_the_same_walk_ends_in_an_assault()
     {
        let state = board(&[("H5", 7, 7), ("A1", 9, 9)], None);
        let walk = aimed(
            "H5",
            &[(7, 8), (7, 9), (8, 9)],
            None,
            BoardClick::Square(square(8, 9)),
        );
        let click = BoardClick::Goblin(id("A1"));
        assert_eq!(
            react(&state, Some(&walk), click),
            Reaction::Prepare(aimed(
                "H5",
                &[(7, 8), (7, 9), (8, 9)],
                Some(Action::Assault { target: id("A1") }),
                click
            ))
        );
    }

    #[test]
    fn given_an_aimed_assault_when_clicking_the_enemy_again_then_it_is_sent() {
        let state = board(&[("H1", 7, 7), ("A1", 8, 7)], None);
        let click = BoardClick::Goblin(id("A1"));
        let draft = aimed("H1", &[], Some(Action::Assault { target: id("A1") }), click);
        assert_eq!(
            react(&state, Some(&draft), click),
            Reaction::Send(draft.activation.clone())
        );
    }

    #[test]
    fn given_a_selected_carrier_when_clicking_a_teammate_then_a_throw_is_aimed() {
        let state = board(&[("H9", 5, 7), ("H5", 9, 7)], Some("H9"));
        let click = BoardClick::Goblin(id("H5"));
        assert_eq!(
            react(&state, Some(&Draft::selecting(id("H9"))), click),
            Reaction::Prepare(aimed(
                "H9",
                &[],
                Some(Action::Throw { receiver: id("H5") }),
                click
            ))
        );
    }

    #[test]
    fn given_a_selected_goblin_without_the_ball_when_clicking_a_teammate_then_the_teammate_is_selected()
     {
        let state = board(&[("H1", 5, 7), ("H5", 9, 7)], None);
        assert_eq!(
            react(
                &state,
                Some(&Draft::selecting(id("H1"))),
                BoardClick::Goblin(id("H5"))
            ),
            Reaction::Prepare(Draft::selecting(id("H5")))
        );
    }

    #[test]
    fn given_an_aimed_walk_when_bracing_then_the_same_walk_ends_in_a_stance() {
        let draft = aimed(
            "H5",
            &[(7, 8), (7, 9)],
            None,
            BoardClick::Square(square(7, 9)),
        );
        assert_eq!(
            stance_order(&draft),
            Activation {
                goblin: id("H5"),
                path: vec![square(7, 8), square(7, 9)],
                action: Some(Action::Stance),
            }
        );
    }

    #[test]
    fn given_an_order_sent_when_preparing_what_comes_next_then_its_goblin_stays_selected() {
        let walk = aimed("H5", &[(8, 7)], None, BoardClick::Square(square(8, 7)));

        let next = prepared_after(Reaction::Send(walk.activation.clone()), Some(walk));

        assert_eq!(next, Some(Draft::selecting(id("H5"))));
    }

    #[test]
    fn given_a_released_goblin_when_preparing_what_comes_next_then_nothing_is_selected() {
        let next = prepared_after(Reaction::Release, Some(Draft::selecting(id("H5"))));

        assert_eq!(next, None);
    }

    #[test]
    fn given_an_ignored_click_when_preparing_what_comes_next_then_the_draft_is_kept() {
        let draft = Draft::selecting(id("H5"));

        let next = prepared_after(Reaction::Ignore, Some(draft.clone()));

        assert_eq!(next, Some(draft));
    }

    #[test]
    fn given_a_goblin_that_already_used_its_action_when_asking_if_it_may_brace_then_no() {
        let mut state = board(&[("H5", 7, 7)], None);
        state.goblins[4].has_action_left = false;

        assert!(!can_still_brace(&Ruleset::default(), &state, id("H5")));
    }

    #[test]
    fn given_a_goblin_with_its_action_left_when_asking_if_it_may_brace_then_yes() {
        let state = board(&[("H5", 7, 7)], None);

        assert!(can_still_brace(&Ruleset::default(), &state, id("H5")));
    }

    #[test]
    fn given_rules_without_stance_when_asking_if_a_goblin_may_brace_then_no() {
        let state = board(&[("H5", 7, 7)], None);
        let without_stance = Ruleset {
            goblins_can_take_stance: false,
            ..Ruleset::default()
        };

        assert!(!can_still_brace(&without_stance, &state, id("H5")));
    }

    #[test]
    fn given_the_safest_and_the_riskiest_bands_when_colouring_them_then_they_differ() {
        assert_ne!(risk_colour(Risk::Safe), risk_colour(Risk::Unlikely));
    }

    #[test]
    fn given_a_certain_success_when_banding_its_risk_then_safe() {
        assert_eq!(risk_of(Probability::CERTAIN), Risk::Safe);
    }

    #[test]
    fn given_three_chances_in_four_when_banding_its_risk_then_likely() {
        assert_eq!(risk_of(Probability::new(3, 4)), Risk::Likely);
    }

    #[test]
    fn given_one_chance_in_two_when_banding_its_risk_then_even() {
        assert_eq!(risk_of(Probability::new(1, 2)), Risk::Even);
    }

    #[test]
    fn given_one_chance_in_four_when_banding_its_risk_then_unlikely() {
        assert_eq!(risk_of(Probability::new(1, 4)), Risk::Unlikely);
    }

    #[test]
    fn given_a_selected_sprinter_alone_when_marking_squares_then_every_reachable_square_is_safe() {
        let state = board(&[("H5", 7, 8)], None);
        let marks = square_marks(
            &Arena::bowl_15(),
            &Ruleset::default(),
            &state,
            TeamId::HOME,
            &Draft::selecting(id("H5")),
        );
        assert!(
            marks.len() > 40
                && marks.values().all(|mark| *mark
                    == SquareMark {
                        risk: Risk::Safe,
                        aimed: false
                    })
        );
    }

    #[test]
    fn given_an_aimed_walk_when_marking_squares_then_its_destination_is_aimed() {
        let state = board(&[("H5", 7, 7)], None);
        let walk = aimed(
            "H5",
            &[(7, 8), (7, 9)],
            None,
            BoardClick::Square(square(7, 9)),
        );
        let marks = square_marks(
            &Arena::bowl_15(),
            &Ruleset::default(),
            &state,
            TeamId::HOME,
            &walk,
        );
        assert_eq!(
            marks.get(&square(7, 9)),
            Some(&SquareMark {
                risk: Risk::Safe,
                aimed: true
            })
        );
    }

    #[test]
    fn given_an_aimed_assault_when_marking_squares_then_the_target_square_carries_its_risk() {
        let state = board(&[("H1", 7, 7), ("A1", 8, 7)], None);
        let assault = aimed(
            "H1",
            &[],
            Some(Action::Assault { target: id("A1") }),
            BoardClick::Goblin(id("A1")),
        );
        let marks = square_marks(
            &Arena::bowl_15(),
            &Ruleset::default(),
            &state,
            TeamId::HOME,
            &assault,
        );
        assert_eq!(
            marks.get(&square(8, 7)),
            Some(&SquareMark {
                risk: Risk::Even,
                aimed: true
            })
        );
    }
}
