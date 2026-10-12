//! The final whistle: who won, what each team did, and a way to play again or to quit.

use bevy::prelude::*;
use bevy::ui_widgets::{Activate, observe};
use goblins_rules::balance::Attempts;
use goblins_rules::scoresheet::{Scoresheet, TeamSheet};
use goblins_rules::state::Phase;
use goblins_rules::team::TeamId;

use crate::goblin_view::BowlAnimation;
use crate::hud::{
    BOLD_FONT, Headlines, READING_FONT, button, overlay, panel, story_colour, text_bundle,
};
use crate::local_match::{MatchOrder, MatchRestarted, MatchSystems, MatchView, RestartableMatch};
use crate::texts::Texts;

const LABEL_COLOUR: Color = Color::srgb(0.92, 0.9, 0.84);
const VALUE_COLOUR: Color = Color::WHITE;

/// What a line of the final scoresheet counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SheetEntry {
    /// Assaults made, won and lost.
    Assaults,
    /// Assaults suffered with a Stance bonus, held and fallen.
    BracedDefences,
    /// Assaults suffered without a Stance bonus, held and fallen.
    OpenDefences,
    /// Dodges, made and failed.
    Dodges,
    /// Passes, completed and failed.
    Passes,
    /// Catches, held and dropped.
    Catches,
    /// Pick-ups, made and failed.
    Pickups,
    /// Interceptions, made and missed.
    Interceptions,
    /// Squares walked holding the ball.
    StepsWithTheBall,
    /// Squares flown by the longest completed pass.
    LongestPass,
    /// Squares flown by every completed pass.
    PassDistance,
    /// Goblins knocked out.
    KnockOuts,
    /// Turns ended by choice, by the clock, by a knockdown and by a fumble.
    TurnEndings,
}

impl SheetEntry {
    /// Every line, in the order the scoresheet shows them.
    pub const ALL: [SheetEntry; 13] = [
        SheetEntry::Assaults,
        SheetEntry::BracedDefences,
        SheetEntry::OpenDefences,
        SheetEntry::Dodges,
        SheetEntry::Passes,
        SheetEntry::Catches,
        SheetEntry::Pickups,
        SheetEntry::Interceptions,
        SheetEntry::StepsWithTheBall,
        SheetEntry::LongestPass,
        SheetEntry::PassDistance,
        SheetEntry::KnockOuts,
        SheetEntry::TurnEndings,
    ];
}

/// What `entry` reads for a team: successes and failures, a count, or how its turns ended.
pub fn sheet_value(entry: SheetEntry, sheet: &TeamSheet) -> String {
    let successes_and_failures =
        |attempts: Attempts| format!("{} / {}", attempts.succeeded, attempts.failed());
    match entry {
        SheetEntry::Assaults => successes_and_failures(sheet.assaults),
        SheetEntry::BracedDefences => successes_and_failures(sheet.braced_defences),
        SheetEntry::OpenDefences => successes_and_failures(sheet.open_defences),
        SheetEntry::Dodges => successes_and_failures(sheet.dodges),
        SheetEntry::Passes => successes_and_failures(sheet.passes),
        SheetEntry::Catches => successes_and_failures(sheet.catches),
        SheetEntry::Pickups => successes_and_failures(sheet.pickups),
        SheetEntry::Interceptions => successes_and_failures(sheet.interceptions),
        SheetEntry::StepsWithTheBall => sheet.steps_with_the_ball.to_string(),
        SheetEntry::LongestPass => sheet.longest_pass.to_string(),
        SheetEntry::PassDistance => sheet.completed_pass_distance.to_string(),
        SheetEntry::KnockOuts => sheet.knock_outs.to_string(),
        SheetEntry::TurnEndings => {
            let endings = sheet.turn_endings;
            format!(
                "{} · {} · {} · {}",
                endings.by_choice, endings.by_clock, endings.by_knockdown, endings.by_fumble
            )
        }
    }
}

/// The scoresheet of the match being played, tallied as events come.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct MatchScoresheet(pub Scoresheet);

#[derive(Component)]
struct FinalOverlay;
#[derive(Component)]
struct FinalResult;
#[derive(Component)]
struct SheetCell {
    entry: SheetEntry,
    team: TeamId,
}
#[derive(Component)]
struct PlayAgainButton;
#[derive(Component)]
struct PlayAgainLabel;
#[derive(Component)]
struct QuitLabel;

/// Shows the end-of-match screen once the final whistle has been played out.
pub struct FinalWhistlePlugin;

impl Plugin for FinalWhistlePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<MatchRestarted>()
            .init_resource::<MatchScoresheet>()
            .add_systems(Startup, spawn_final_screen)
            .add_systems(
                Update,
                (
                    forget_the_last_match,
                    tally_the_match,
                    show_final_screen,
                    fill_final_screen,
                )
                    .chain()
                    .in_set(MatchSystems::Present),
            );
    }
}

fn spawn_final_screen(mut commands: Commands, assets: Res<AssetServer>, texts: Res<Texts>) {
    let reading: Handle<Font> = assets.load(READING_FONT);
    let bold: Handle<Font> = assets.load(BOLD_FONT);
    commands
        .spawn((overlay(), Visibility::Hidden, FinalOverlay))
        .with_children(|screen| {
            screen.spawn(panel()).with_children(|panel| {
                panel.spawn(text_bundle(texts.final_title(), &bold, 40.0, Color::WHITE));
                panel.spawn((
                    text_bundle("", &bold, 28.0, Color::srgb(1.0, 0.9, 0.55)),
                    FinalResult,
                ));
                panel
                    .spawn(Node {
                        display: Display::Grid,
                        grid_template_columns: vec![
                            GridTrack::auto(),
                            GridTrack::px(230.0),
                            GridTrack::px(230.0),
                        ],
                        column_gap: px(20),
                        row_gap: px(4),
                        ..default()
                    })
                    .with_children(|grid| {
                        grid.spawn(text_bundle("", &reading, 19.0, LABEL_COLOUR));
                        for team in TeamId::DUEL {
                            grid.spawn(text_bundle(
                                texts.team_name(team),
                                &bold,
                                20.0,
                                story_colour(Some(team)),
                            ));
                        }
                        for entry in SheetEntry::ALL {
                            grid.spawn(text_bundle(
                                texts.sheet_label(entry),
                                &reading,
                                19.0,
                                LABEL_COLOUR,
                            ));
                            for team in TeamId::DUEL {
                                grid.spawn((
                                    text_bundle("", &bold, 19.0, VALUE_COLOUR),
                                    SheetCell { entry, team },
                                ));
                            }
                        }
                    });
                panel
                    .spawn(Node {
                        column_gap: px(16),
                        ..default()
                    })
                    .with_children(|buttons| {
                        buttons.spawn((
                            button(texts.play_again_button(), &bold, PlayAgainLabel),
                            PlayAgainButton,
                            observe(play_again_pressed),
                        ));
                        buttons.spawn((
                            button(texts.quit_button(), &bold, QuitLabel),
                            observe(quit_pressed),
                        ));
                    });
            });
        });
}

fn play_again_pressed(_activate: On<Activate>, mut orders: MessageWriter<MatchOrder>) {
    orders.write(MatchOrder::Restart);
}

fn quit_pressed(_activate: On<Activate>, mut exit: MessageWriter<AppExit>) {
    exit.write(AppExit::Success);
}

fn forget_the_last_match(
    mut restarts: MessageReader<MatchRestarted>,
    mut scoresheet: ResMut<MatchScoresheet>,
) {
    if restarts.read().count() > 0 {
        scoresheet.0 = Scoresheet::default();
    }
}

fn tally_the_match(view: Res<MatchView>, mut scoresheet: ResMut<MatchScoresheet>) {
    if !view.fresh_events.is_empty() {
        let tallied = std::mem::take(&mut scoresheet.0).tally(&view.fresh_events);
        scoresheet.0 = tallied;
    }
}

fn show_final_screen(
    view: Res<MatchView>,
    animation: Res<BowlAnimation>,
    headlines: Res<Headlines>,
    restartable: Option<Res<RestartableMatch>>,
    mut screen: Single<&mut Visibility, (With<FinalOverlay>, Without<PlayAgainButton>)>,
    mut play_again: Single<&mut Visibility, (With<PlayAgainButton>, Without<FinalOverlay>)>,
) {
    let played_out = !animation.busy && headlines.are_over();
    let over = view.snapshot.state.phase == Phase::Finished;
    screen.set_if_neq(if over && played_out {
        Visibility::Visible
    } else {
        Visibility::Hidden
    });
    play_again.set_if_neq(if restartable.is_some() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    });
}

fn fill_final_screen(
    view: Res<MatchView>,
    texts: Res<Texts>,
    scoresheet: Res<MatchScoresheet>,
    mut result: Single<&mut Text, (With<FinalResult>, Without<SheetCell>)>,
    mut cells: Query<(&SheetCell, &mut Text), Without<FinalResult>>,
) {
    let state = &view.snapshot.state;
    if state.phase != Phase::Finished {
        return;
    }
    let wanted = texts.final_result(
        state.score.first().copied().unwrap_or_default(),
        state.score.get(1).copied().unwrap_or_default(),
    );
    if result.0 != wanted {
        result.0 = wanted;
    }
    for (cell, mut text) in &mut cells {
        let value = sheet_value(cell.entry, &scoresheet.0.team(cell.team));
        if text.0 != value {
            text.0 = value;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use goblins_rules::scoresheet::TurnEndings;

    fn sheet() -> TeamSheet {
        TeamSheet {
            assaults: Attempts {
                tried: 5,
                succeeded: 3,
            },
            longest_pass: 7,
            turn_endings: TurnEndings {
                by_choice: 9,
                by_clock: 1,
                by_knockdown: 2,
                by_fumble: 0,
                by_score: 0,
                with_no_goblin_left: 0,
            },
            ..TeamSheet::default()
        }
    }

    #[test]
    fn given_five_assaults_and_three_won_when_reading_the_line_then_three_won_two_lost() {
        assert_eq!(sheet_value(SheetEntry::Assaults, &sheet()), "3 / 2");
    }

    #[test]
    fn given_a_seven_square_pass_when_reading_the_longest_pass_then_seven() {
        assert_eq!(sheet_value(SheetEntry::LongestPass, &sheet()), "7");
    }

    #[test]
    fn given_turns_ended_in_several_ways_when_reading_them_then_choice_clock_knockdown_and_fumble()
    {
        assert_eq!(
            sheet_value(SheetEntry::TurnEndings, &sheet()),
            "9 · 1 · 2 · 0"
        );
    }
}
