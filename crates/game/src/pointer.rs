//! The pointer shows what a click would do: select a goblin, walk, dodge, pick the ball up, score,
//! assault or throw, in the colour of the risk; or that the selected goblin can do nothing there.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::ecs::system::SystemParam;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::{CursorIcon, CustomCursor, CustomCursorImage, PrimaryWindow, SystemCursorIcon};
use goblins_rules::arena::Arena;
use goblins_rules::command::{Action, Activation};
use goblins_rules::preview::{RiskyStep, activation_odds};
use goblins_rules::ruleset::Ruleset;
use goblins_rules::state::GameState;
use goblins_rules::team::TeamId;

use crate::badges::{BADGE_SIZE, Glyph, badge_pixels};
use crate::bowl_view::{BoardClick, HoveredSquare};
use crate::goblin_view::GoblinFigure;
use crate::local_match::{MatchSystems, MatchView};
use crate::orders::{
    Draft, PreparedOrder, Reaction, Risk, coach_giving_orders, react_to_click, risk_colour, risk_of,
};

const RISKS: [Risk; 4] = [Risk::Safe, Risk::Likely, Risk::Even, Risk::Unlikely];

/// What a click under the pointer would do.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PointerIntent {
    /// Nothing to order here: the usual arrow.
    Plain,
    /// Select a goblin that can still play.
    Select,
    /// Aim or send an order of this kind, at this risk.
    Order(Glyph, Risk),
    /// Nothing the selected goblin can do here.
    Forbidden,
}

/// What clicking `target` would do for `team` while `draft` is being prepared.
pub fn pointer_intent(
    arena: &Arena,
    ruleset: &Ruleset,
    state: &GameState,
    team: TeamId,
    draft: Option<&Draft>,
    target: BoardClick,
) -> PointerIntent {
    match react_to_click(arena, ruleset, state, team, draft, target) {
        Reaction::Ignore if draft.is_some() => PointerIntent::Forbidden,
        Reaction::Ignore | Reaction::Release => PointerIntent::Plain,
        Reaction::Prepare(prepared) if prepared.aim.is_none() => PointerIntent::Select,
        Reaction::Prepare(Draft { activation, .. }) | Reaction::Send(activation) => {
            order_intent(arena, ruleset, state, team, &activation)
        }
    }
}

fn order_intent(
    arena: &Arena,
    ruleset: &Ruleset,
    state: &GameState,
    team: TeamId,
    activation: &Activation,
) -> PointerIntent {
    let Ok(odds) = activation_odds(arena, ruleset, state, team, activation) else {
        return PointerIntent::Forbidden;
    };
    let rolls_on_the_way = |wanted: fn(&RiskyStep) -> bool| odds.risky_steps.iter().any(wanted);
    let glyph = if odds.scores_on_the_way {
        Glyph::Score
    } else {
        match activation.action {
            Some(Action::Assault { .. }) => Glyph::Assault,
            Some(Action::Throw { .. }) => Glyph::Throw,
            Some(Action::Stance) | None => {
                if rolls_on_the_way(|step| matches!(step, RiskyStep::Pick { .. })) {
                    Glyph::Pick
                } else if rolls_on_the_way(|step| matches!(step, RiskyStep::Dodge { .. })) {
                    Glyph::Dodge
                } else {
                    Glyph::Walk
                }
            }
        }
    };
    PointerIntent::Order(glyph, risk_of(odds.overall_success))
}

#[derive(Resource)]
struct Badges(HashMap<(Glyph, Risk), Handle<Image>>);

#[derive(Clone, Debug, PartialEq, Eq)]
struct IntentInputs {
    version: u64,
    target: Option<BoardClick>,
    draft: Option<Draft>,
}

#[derive(Default)]
struct PointerMemory {
    computed_for: Option<IntentInputs>,
    shown: Option<PointerIntent>,
}

#[derive(SystemParam)]
struct PointedAt<'w, 's> {
    square: Res<'w, HoveredSquare>,
    figures: Query<'w, 's, (&'static GoblinFigure, &'static Hovered)>,
}

impl PointedAt<'_, '_> {
    fn target(&self) -> Option<BoardClick> {
        self.figures
            .iter()
            .find(|(_, hovered)| hovered.get())
            .map(|(figure, _)| BoardClick::Goblin(figure.id))
            .or(self.square.0.map(BoardClick::Square))
    }
}

/// Turns the window's pointer into a badge telling what a click would do on the board.
pub struct PointerPlugin;

impl Plugin for PointerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, draw_badges)
            .add_systems(Update, point_cursor.after(MatchSystems::Present));
    }
}

fn draw_badges(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let badges = Glyph::ALL
        .into_iter()
        .flat_map(|glyph| RISKS.map(|risk| (glyph, risk)))
        .map(|(glyph, risk)| {
            let fill = risk_colour(risk).to_srgba().to_u8_array_no_alpha();
            let image = Image::new(
                Extent3d {
                    width: BADGE_SIZE,
                    height: BADGE_SIZE,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                badge_pixels(glyph, fill),
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
            );
            ((glyph, risk), images.add(image))
        })
        .collect();
    commands.insert_resource(Badges(badges));
}

fn point_cursor(
    mut commands: Commands,
    view: Res<MatchView>,
    prepared: Res<PreparedOrder>,
    pointed_at: PointedAt,
    badges: Option<Res<Badges>>,
    window: Option<Single<Entity, With<PrimaryWindow>>>,
    mut memory: Local<PointerMemory>,
) {
    let (Some(badges), Some(window)) = (badges, window) else {
        return;
    };
    let target = pointed_at.target();
    let inputs = IntentInputs {
        version: view.snapshot.version,
        target,
        draft: prepared.0.clone(),
    };
    if memory.computed_for.as_ref() == Some(&inputs) {
        return;
    }
    let intent = coach_giving_orders(&view.snapshot).zip(target).map_or(
        PointerIntent::Plain,
        |(team, target)| {
            pointer_intent(
                &view.arena,
                &view.ruleset,
                &view.snapshot.state,
                team,
                prepared.0.as_ref(),
                target,
            )
        },
    );
    memory.computed_for = Some(inputs);
    if memory.shown == Some(intent) {
        return;
    }
    memory.shown = Some(intent);
    let icon = match intent {
        PointerIntent::Plain => CursorIcon::System(SystemCursorIcon::Default),
        PointerIntent::Select => CursorIcon::System(SystemCursorIcon::Pointer),
        PointerIntent::Forbidden => CursorIcon::System(SystemCursorIcon::NotAllowed),
        PointerIntent::Order(glyph, risk) => match badges.0.get(&(glyph, risk)) {
            Some(badge) => CursorIcon::Custom(CustomCursor::Image(CustomCursorImage {
                handle: badge.clone(),
                hotspot: (
                    u16::try_from(BADGE_SIZE / 2).unwrap_or_default(),
                    u16::try_from(BADGE_SIZE / 2).unwrap_or_default(),
                ),
                ..default()
            })),
            None => CursorIcon::System(SystemCursorIcon::Pointer),
        },
    };
    commands.entity(*window).insert(icon);
}

#[cfg(test)]
mod tests {
    use super::*;
    use goblins_rules::host::{MatchHost, MatchSetup, SeatKind};
    use goblins_rules::roster::{GoblinId, Roster};
    use goblins_rules::square::Square;
    use goblins_rules::state::{Ball, Presence};

    fn id(text: &str) -> GoblinId {
        text.parse().unwrap()
    }

    fn rules_and_board(standing: &[(&str, u8, u8)], ball: Ball) -> (Arena, Ruleset, GameState) {
        let (host, _) = MatchHost::start(MatchSetup {
            ruleset: Ruleset::default(),
            rosters: [Roster::DEFAULT, Roster::DEFAULT],
            seed: 2,
            seats: [SeatKind::Human, SeatKind::Human],
            turn_seconds: [60, 60],
            agent_ready_grace_seconds: 15,
        })
        .unwrap();
        let mut state = host.snapshot().state;
        for goblin in &mut state.goblins {
            goblin.presence = Presence::KnockedOut;
            goblin.refresh_its_turn();
        }
        for (name, x, y) in standing {
            if let Some(goblin) = state
                .goblins
                .iter_mut()
                .find(|goblin| goblin.id == id(name))
            {
                goblin.presence = Presence::Standing(Square::new(*x, *y));
            }
        }
        state.ball = ball;
        (
            host.game().arena().clone(),
            host.game().ruleset().clone(),
            state,
        )
    }

    fn intent(
        standing: &[(&str, u8, u8)],
        ball: Ball,
        selected: Option<&str>,
        target: BoardClick,
    ) -> PointerIntent {
        let (arena, ruleset, state) = rules_and_board(standing, ball);
        let draft = selected.map(|goblin| Draft::selecting(id(goblin)));
        pointer_intent(
            &arena,
            &ruleset,
            &state,
            TeamId::HOME,
            draft.as_ref(),
            target,
        )
    }

    const BALL_AWAY: Ball = Ball::Loose {
        at: Square::new(10, 2),
    };

    fn app_pointing_at(square: Square, selected: Option<&str>) -> App {
        let (mut host, _) = MatchHost::start(MatchSetup {
            ruleset: Ruleset::default(),
            rosters: [Roster::DEFAULT, Roster::DEFAULT],
            seed: 2,
            seats: [SeatKind::Human, SeatKind::Human],
            turn_seconds: [60, 60],
            agent_ready_grace_seconds: 15,
        })
        .unwrap();
        host.ready(TeamId::AWAY).unwrap();
        let view = MatchView {
            snapshot: host.snapshot(),
            arena: host.game().arena().clone(),
            ruleset: host.game().ruleset().clone(),
            fresh_events: Vec::new(),
            refusal: None,
        };
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .insert_resource(view)
            .insert_resource(PreparedOrder(
                selected.map(|goblin| Draft::selecting(id(goblin))),
            ))
            .insert_resource(HoveredSquare(Some(square)))
            .add_plugins(PointerPlugin);
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        app.update();
        app
    }

    fn window_cursor(app: &mut App) -> Option<CursorIcon> {
        app.world_mut()
            .query_filtered::<&CursorIcon, With<PrimaryWindow>>()
            .iter(app.world())
            .next()
            .cloned()
    }

    #[test]
    fn given_a_selected_sprinter_pointing_at_a_square_in_reach_when_the_frame_runs_then_the_window_shows_a_badge()
     {
        let mut app = app_pointing_at(Square::new(12, 5), Some("A5"));
        assert!(matches!(
            window_cursor(&mut app),
            Some(CursorIcon::Custom(CustomCursor::Image(_)))
        ));
    }

    #[test]
    fn given_nothing_selected_pointing_at_an_empty_square_when_the_frame_runs_then_the_usual_arrow()
    {
        let mut app = app_pointing_at(Square::new(12, 12), None);
        assert_eq!(
            window_cursor(&mut app),
            Some(CursorIcon::System(SystemCursorIcon::Default))
        );
    }

    #[test]
    fn given_nothing_selected_when_pointing_at_a_goblin_that_can_play_then_select() {
        assert_eq!(
            intent(
                &[("H5", 10, 10)],
                BALL_AWAY,
                None,
                BoardClick::Goblin(id("H5"))
            ),
            PointerIntent::Select
        );
    }

    #[test]
    fn given_nothing_selected_when_pointing_at_an_empty_square_then_plain() {
        assert_eq!(
            intent(
                &[("H5", 10, 10)],
                BALL_AWAY,
                None,
                BoardClick::Square(Square::new(12, 12))
            ),
            PointerIntent::Plain
        );
    }

    #[test]
    fn given_a_selected_sprinter_alone_when_pointing_at_a_square_in_reach_then_a_safe_walk() {
        assert_eq!(
            intent(
                &[("H5", 10, 10)],
                BALL_AWAY,
                Some("H5"),
                BoardClick::Square(Square::new(12, 10))
            ),
            PointerIntent::Order(Glyph::Walk, Risk::Safe)
        );
    }

    #[test]
    fn given_a_selected_sprinter_next_to_a_bruiser_when_pointing_away_then_an_even_dodge() {
        assert_eq!(
            intent(
                &[("H5", 10, 10), ("A1", 11, 10)],
                BALL_AWAY,
                Some("H5"),
                BoardClick::Square(Square::new(8, 10))
            ),
            PointerIntent::Order(Glyph::Dodge, Risk::Even)
        );
    }

    #[test]
    fn given_a_selected_sprinter_when_pointing_at_the_loose_ball_then_a_likely_pick_up() {
        assert_eq!(
            intent(
                &[("H5", 10, 10)],
                Ball::Loose {
                    at: Square::new(10, 12)
                },
                Some("H5"),
                BoardClick::Square(Square::new(10, 12))
            ),
            PointerIntent::Order(Glyph::Pick, Risk::Likely)
        );
    }

    #[test]
    fn given_a_selected_bruiser_next_to_a_sprinter_when_pointing_at_it_then_a_likely_assault() {
        assert_eq!(
            intent(
                &[("H1", 10, 10), ("A5", 11, 10)],
                BALL_AWAY,
                Some("H1"),
                BoardClick::Goblin(id("A5"))
            ),
            PointerIntent::Order(Glyph::Assault, Risk::Likely)
        );
    }

    #[test]
    fn given_a_selected_carrier_when_pointing_at_an_open_teammate_then_an_even_throw() {
        assert_eq!(
            intent(
                &[("H9", 10, 10), ("H5", 10, 14)],
                Ball::Carried { by: id("H9") },
                Some("H9"),
                BoardClick::Goblin(id("H5"))
            ),
            PointerIntent::Order(Glyph::Throw, Risk::Even)
        );
    }

    #[test]
    fn given_a_selected_carrier_two_squares_from_the_den_when_pointing_into_it_then_a_safe_score() {
        assert_eq!(
            intent(
                &[("H5", 18, 10)],
                Ball::Carried { by: id("H5") },
                Some("H5"),
                BoardClick::Square(Square::new(20, 10))
            ),
            PointerIntent::Order(Glyph::Score, Risk::Safe)
        );
    }

    #[test]
    fn given_a_selected_bruiser_when_pointing_beyond_its_reach_then_forbidden() {
        assert_eq!(
            intent(
                &[("H1", 10, 10)],
                BALL_AWAY,
                Some("H1"),
                BoardClick::Square(Square::new(15, 10))
            ),
            PointerIntent::Forbidden
        );
    }
}
