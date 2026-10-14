//! Goblin cards in the bottom corners: the selected teammate on the left, above the order being
//! prepared, and the opponent under the pointer on the right.

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use goblins_rules::host::{MatchSnapshot, SeatKind};
use goblins_rules::roster::GoblinId;
use goblins_rules::state::GameState;
use goblins_rules::team::TeamId;

use crate::bowl_view::HoveredSquare;
use crate::goblin_view::GoblinFigure;
use crate::hud::{BOLD_FONT, READING_FONT, human_team_in_control, story_colour, text_bundle};
use crate::local_match::{MatchSystems, MatchView};
use crate::orders::PreparedOrder;
use crate::texts::Texts;

const CARD_TEXT: Color = Color::srgb(0.92, 0.92, 0.88);
const CARD_BACKGROUND: Color = Color::srgba(0.1, 0.08, 0.06, 0.8);

/// Which goblin a card shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardSide {
    /// The coach's selected goblin.
    Teammate,
    /// The opponent under the pointer.
    Opponent,
}

#[derive(Component)]
struct GoblinCard(CardSide);

#[derive(Component)]
struct CardLine {
    side: CardSide,
    index: usize,
}

/// A card for one side, its lines filled in as the selection and the pointer move.
pub fn goblin_card(side: CardSide, bold: &Handle<Font>, reading: &Handle<Font>) -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(2),
            padding: UiRect::axes(px(14), px(8)),
            border_radius: BorderRadius::all(px(10)),
            ..default()
        },
        BackgroundColor(CARD_BACKGROUND),
        Visibility::Hidden,
        Pickable::IGNORE,
        GoblinCard(side),
        children![
            (
                text_bundle("", bold, 22.0, Color::WHITE),
                CardLine { side, index: 0 }
            ),
            (
                text_bundle("", reading, 18.0, CARD_TEXT),
                CardLine { side, index: 1 }
            ),
            (
                text_bundle("", reading, 18.0, CARD_TEXT),
                CardLine { side, index: 2 }
            ),
        ],
    )
}

/// The team the window plays for: the human coach in control, else the only human seat.
pub fn viewing_team(snapshot: &MatchSnapshot) -> Option<TeamId> {
    human_team_in_control(snapshot).or_else(|| {
        let mut humans = TeamId::DUEL
            .into_iter()
            .filter(|team| snapshot.seats.get(team.index()) == Some(&SeatKind::Human));
        match (humans.next(), humans.next()) {
            (Some(only), None) => Some(only),
            _ => None,
        }
    })
}

/// The goblin the opponent card shows: the hovered one, unless it plays for the viewing team.
pub fn opponent_shown(
    state: &GameState,
    viewing_team: Option<TeamId>,
    hovered: Option<GoblinId>,
) -> Option<GoblinId> {
    let goblin = state.goblin(hovered?)?;
    (Some(goblin.team()) != viewing_team).then_some(goblin.id)
}

/// Shows the selected teammate and the hovered opponent in the bottom corners.
pub struct GoblinCardsPlugin;

impl Plugin for GoblinCardsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_opponent_card)
            .add_systems(Update, refresh_cards.after(MatchSystems::Present));
    }
}

fn spawn_opponent_card(mut commands: Commands, assets: Res<AssetServer>) {
    let bold: Handle<Font> = assets.load(BOLD_FONT);
    let reading: Handle<Font> = assets.load(READING_FONT);
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: px(16),
            bottom: px(150),
            ..default()
        },
        Pickable::IGNORE,
        children![goblin_card(CardSide::Opponent, &bold, &reading)],
    ));
}

fn refresh_cards(
    view: Res<MatchView>,
    prepared: Res<PreparedOrder>,
    texts: Res<Texts>,
    square: Res<HoveredSquare>,
    figures: Query<(&GoblinFigure, &Hovered)>,
    mut cards: Query<(&GoblinCard, &mut Visibility)>,
    mut lines: Query<(&CardLine, &mut Text, &mut TextColor)>,
) {
    let state = &view.snapshot.state;
    let hovered = figures
        .iter()
        .find(|(_, hovered)| hovered.get())
        .map(|(figure, _)| figure.id)
        .or_else(|| {
            square
                .0
                .and_then(|square| state.goblin_at(square))
                .map(|goblin| goblin.id)
        });
    let shown = |side: CardSide| match side {
        CardSide::Teammate => prepared.0.as_ref().map(|draft| draft.activation.goblin),
        CardSide::Opponent => opponent_shown(state, viewing_team(&view.snapshot), hovered),
    };
    for (card, mut visibility) in &mut cards {
        visibility.set_if_neq(if shown(card.0).is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
    for (line, mut text, mut colour) in &mut lines {
        let Some(goblin) = shown(line.side).and_then(|id| state.goblin(id)) else {
            continue;
        };
        let coached = line.side == CardSide::Teammate;
        let wanted = texts.card_lines(goblin, coached)[line.index].clone();
        if text.0 != wanted {
            text.0 = wanted;
        }
        if line.index == 0 {
            colour.set_if_neq(TextColor(story_colour(Some(goblin.team()))));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use goblins_rules::host::{MatchHost, MatchSetup};
    use goblins_rules::roster::Roster;
    use goblins_rules::ruleset::Ruleset;
    use goblins_rules::square::Square;

    fn id(text: &str) -> GoblinId {
        text.parse().unwrap()
    }

    fn host_against_a_bot() -> MatchHost {
        MatchHost::start(MatchSetup {
            ruleset: Ruleset::default(),
            rosters: [Roster::DEFAULT, Roster::DEFAULT],
            seed: 2,
            seats: [SeatKind::Human, SeatKind::Bot],
            turn_seconds: [60, 60],
            agent_ready_grace_seconds: 15,
        })
        .unwrap()
        .0
    }

    #[test]
    fn given_a_human_against_a_bot_when_asking_who_the_window_plays_for_then_the_human_whatever_the_turn()
     {
        assert_eq!(
            viewing_team(&host_against_a_bot().snapshot()),
            Some(TeamId::HOME)
        );
    }

    #[test]
    fn given_the_home_coach_hovering_an_away_goblin_when_choosing_the_opponent_card_then_that_goblin()
     {
        let state = host_against_a_bot().snapshot().state;
        assert_eq!(
            opponent_shown(&state, Some(TeamId::HOME), Some(id("A4"))),
            Some(id("A4"))
        );
    }

    #[test]
    fn given_the_home_coach_hovering_one_of_its_goblins_when_choosing_the_opponent_card_then_none()
    {
        let state = host_against_a_bot().snapshot().state;
        assert_eq!(
            opponent_shown(&state, Some(TeamId::HOME), Some(id("H4"))),
            None
        );
    }

    fn app_hovering(square: Square) -> App {
        let host = host_against_a_bot();
        let view = MatchView {
            snapshot: host.snapshot(),
            arena: host.game().arena().clone(),
            ruleset: host.game().ruleset().clone(),
            fresh_events: Vec::new(),
            refusal: None,
        };
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Font>()
            .insert_resource(view)
            .insert_resource(Texts::default())
            .insert_resource(PreparedOrder(None))
            .insert_resource(HoveredSquare(Some(square)))
            .add_plugins(GoblinCardsPlugin);
        app.update();
        app
    }

    fn opponent_card(app: &mut App) -> (Visibility, Vec<String>) {
        let visibility = app
            .world_mut()
            .query::<(&GoblinCard, &Visibility)>()
            .iter(app.world())
            .find(|(card, _)| card.0 == CardSide::Opponent)
            .map(|(_, visibility)| *visibility)
            .unwrap_or(Visibility::Hidden);
        let mut lines: Vec<(usize, String)> = app
            .world_mut()
            .query::<(&CardLine, &Text)>()
            .iter(app.world())
            .filter(|(line, _)| line.side == CardSide::Opponent)
            .map(|(line, text)| (line.index, text.0.clone()))
            .collect();
        lines.sort();
        (
            visibility,
            lines.into_iter().map(|(_, text)| text).collect(),
        )
    }

    #[test]
    fn given_the_pointer_over_an_away_sprinter_when_the_frame_runs_then_its_card_shows_bottom_right()
     {
        let sprinter_square = host_against_a_bot()
            .snapshot()
            .state
            .goblin(id("A4"))
            .and_then(|goblin| goblin.square())
            .unwrap();
        let mut app = app_hovering(sprinter_square);

        let (visibility, lines) = opponent_card(&mut app);

        assert_eq!(
            (visibility, lines.first().cloned()),
            (Visibility::Inherited, Some("A4 · Sprinteur".to_string()))
        );
    }

    #[test]
    fn given_the_pointer_over_an_empty_square_when_the_frame_runs_then_no_opponent_card() {
        let mut app = app_hovering(Square::new(10, 2));

        let (visibility, _) = opponent_card(&mut app);

        assert_eq!(visibility, Visibility::Hidden);
    }
}
