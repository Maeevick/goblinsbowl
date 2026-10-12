//! Development aid: when `GOBLINSBOWL_SCREENSHOT` names an image path, the game renders its camera
//! (3D scene and HUD) into an offscreen image, saves it there after a few frames, then quits.
//! Rendering offscreen keeps captures working while the display is asleep or locked.
//!
//! `GOBLINSBOWL_SCRIPT` plays inputs at given frames beforehand, for example
//! `30:ready;60:click H5;90:click 7,9;100:done H6;120:end;600:restart`, so a capture can show an
//! order being given, a goblin done for its turn, the turn handed over or a new match.

use std::path::PathBuf;

use bevy::camera::RenderTarget;
use bevy::diagnostic::FrameCount;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::render::view::window::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk};
use bevy::ui::IsDefaultUiCamera;
use goblins_rules::command::{Activation, Command};
use goblins_rules::roster::GoblinId;
use goblins_rules::square::Square;

use crate::bowl_view::BoardClick;
use crate::camera_rig::CameraRig;
use crate::hud::{human_team_in_control, team_waiting_for_ready};
use crate::local_match::{MatchOrder, MatchSystems, MatchView};

const SCREENSHOT_PATH_VARIABLE: &str = "GOBLINSBOWL_SCREENSHOT";
const SCREENSHOT_FRAME_VARIABLE: &str = "GOBLINSBOWL_SCREENSHOT_FRAME";
const SCRIPT_VARIABLE: &str = "GOBLINSBOWL_SCRIPT";
const DEFAULT_CAPTURE_FRAME: u32 = 90;
const CAPTURE_WIDTH: u32 = 1600;
const CAPTURE_HEIGHT: u32 = 900;

/// Where and when to capture the window.
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct CaptureRequest {
    /// File the capture is written to; the extension picks the image format.
    pub path: PathBuf,
    /// Frame number at which the capture is taken, leaving time for assets to load.
    pub at_frame: u32,
}

/// An input played by a development script.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptedInput {
    /// Press the Ready button of the team waiting for it.
    Ready,
    /// Click on the board.
    Click(BoardClick),
    /// Order a goblin of the team in control to stay put and do nothing, ending its turn.
    Done(GoblinId),
    /// End the turn of the human team in control.
    EndTurn,
    /// Start a new match, as the Play again button does.
    Restart,
}

/// A scripted input and the frame it is played at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptStep {
    /// Frame number at which the input is played.
    pub at_frame: u32,
    /// The input.
    pub input: ScriptedInput,
}

#[derive(Resource)]
struct Script {
    steps: Vec<ScriptStep>,
    played: usize,
}

#[derive(Resource)]
struct OffscreenTarget(Handle<Image>);

#[derive(Resource, Default)]
struct CaptureProgress {
    requested: bool,
    saved: bool,
}

/// Captures the window and quits when the screenshot environment variable is set; does nothing otherwise.
pub struct DevCapturePlugin;

impl Plugin for DevCapturePlugin {
    fn build(&self, app: &mut App) {
        let request = capture_request_from(
            std::env::var(SCREENSHOT_PATH_VARIABLE).ok(),
            std::env::var(SCREENSHOT_FRAME_VARIABLE).ok(),
        );
        if let Some(request) = request {
            app.insert_resource(request)
                .init_resource::<CaptureProgress>()
                .add_systems(PostStartup, redirect_camera_offscreen)
                .add_systems(Update, (request_capture, quit_once_saved));
        }
        let script = std::env::var(SCRIPT_VARIABLE)
            .ok()
            .map(|text| parse_script(&text));
        match script {
            Some(Ok(steps)) => {
                app.insert_resource(Script { steps, played: 0 })
                    .add_systems(
                        Update,
                        play_script
                            .in_set(MatchSystems::Apply)
                            .before(MatchSystems::Present),
                    );
            }
            Some(Err(problem)) => eprintln!("{SCRIPT_VARIABLE} ignored: {problem}"),
            None => {}
        }
    }
}

/// Steps of a development script such as `30:ready;60:click H5;90:click 7,9;100:done H6;120:end`,
/// in the order given.
///
/// # Errors
/// A description of the first step that cannot be read.
pub fn parse_script(text: &str) -> Result<Vec<ScriptStep>, String> {
    text.split(';')
        .map(str::trim)
        .filter(|step| !step.is_empty())
        .map(parse_step)
        .collect()
}

fn parse_step(step: &str) -> Result<ScriptStep, String> {
    let (frame, input) = step
        .split_once(':')
        .ok_or_else(|| format!("\"{step}\" has no frame"))?;
    let at_frame = frame
        .trim()
        .parse()
        .map_err(|_| format!("\"{frame}\" is not a frame number"))?;
    let input = input.trim();
    let input = if input == "ready" {
        ScriptedInput::Ready
    } else if input == "end" {
        ScriptedInput::EndTurn
    } else if input == "restart" {
        ScriptedInput::Restart
    } else if let Some(target) = input.strip_prefix("click ") {
        ScriptedInput::Click(parse_click_target(target.trim())?)
    } else if let Some(goblin) = input.strip_prefix("done ") {
        ScriptedInput::Done(
            goblin
                .trim()
                .parse()
                .map_err(|_| format!("\"{goblin}\" is not a goblin"))?,
        )
    } else {
        return Err(format!(
            "\"{input}\" is neither ready, end, restart, a click nor a goblin done"
        ));
    };
    Ok(ScriptStep { at_frame, input })
}

fn parse_click_target(target: &str) -> Result<BoardClick, String> {
    if let Some((x, y)) = target.split_once(',') {
        let coordinate = |value: &str| {
            value
                .trim()
                .parse::<u8>()
                .map_err(|_| format!("\"{target}\" is not a square"))
        };
        return Ok(BoardClick::Square(Square::new(
            coordinate(x)?,
            coordinate(y)?,
        )));
    }
    target
        .parse::<GoblinId>()
        .map(BoardClick::Goblin)
        .map_err(|_| format!("\"{target}\" is neither a square nor a goblin"))
}

fn play_script(
    frames: Res<FrameCount>,
    view: Res<MatchView>,
    mut script: ResMut<Script>,
    mut rig: ResMut<CameraRig>,
    mut clicks: MessageWriter<BoardClick>,
    mut orders: MessageWriter<MatchOrder>,
) {
    while let Some(step) = script.steps.get(script.played).copied() {
        if step.at_frame > frames.0 {
            return;
        }
        script.played += 1;
        match step.input {
            ScriptedInput::Ready => {
                if let Some(team) = team_waiting_for_ready(&view.snapshot) {
                    *rig = CameraRig::behind(team);
                    orders.write(MatchOrder::Ready { team });
                }
            }
            ScriptedInput::Click(click) => {
                clicks.write(click);
            }
            ScriptedInput::Done(goblin) => {
                if let Some(team) = human_team_in_control(&view.snapshot) {
                    orders.write(MatchOrder::Command {
                        team,
                        command: Command::Act(Activation {
                            goblin,
                            path: Vec::new(),
                            action: None,
                        }),
                    });
                }
            }
            ScriptedInput::EndTurn => {
                if let Some(team) = human_team_in_control(&view.snapshot) {
                    orders.write(MatchOrder::Command {
                        team,
                        command: Command::EndTurn,
                    });
                }
            }
            ScriptedInput::Restart => {
                orders.write(MatchOrder::Restart);
            }
        }
    }
}

/// Capture request built from the values of the path and frame variables, if a path is given.
///
/// A missing or unreadable frame falls back to frame 90.
pub fn capture_request_from(path: Option<String>, frame: Option<String>) -> Option<CaptureRequest> {
    let path = path.filter(|path| !path.trim().is_empty())?;
    let at_frame = frame
        .and_then(|frame| frame.trim().parse().ok())
        .unwrap_or(DEFAULT_CAPTURE_FRAME);
    Some(CaptureRequest {
        path: PathBuf::from(path),
        at_frame,
    })
}

fn redirect_camera_offscreen(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    cameras: Query<Entity, With<Camera3d>>,
) {
    let target = images.add(Image::new_target_texture(
        CAPTURE_WIDTH,
        CAPTURE_HEIGHT,
        TextureFormat::Rgba8Unorm,
        Some(TextureFormat::Rgba8UnormSrgb),
    ));
    for camera in &cameras {
        commands.entity(camera).insert((
            RenderTarget::Image(target.clone().into()),
            IsDefaultUiCamera,
        ));
    }
    commands.insert_resource(OffscreenTarget(target));
}

fn request_capture(
    mut commands: Commands,
    frames: Res<FrameCount>,
    request: Res<CaptureRequest>,
    target: Option<Res<OffscreenTarget>>,
    mut progress: ResMut<CaptureProgress>,
) {
    let Some(target) = target else {
        return;
    };
    if progress.requested || frames.0 < request.at_frame {
        return;
    }
    progress.requested = true;
    commands
        .spawn(Screenshot::image(target.0.clone()))
        .observe(save_to_disk(request.path.clone()))
        .observe(
            |_captured: On<ScreenshotCaptured>, mut progress: ResMut<CaptureProgress>| {
                progress.saved = true;
            },
        );
}

fn quit_once_saved(progress: Res<CaptureProgress>, mut exit: MessageWriter<AppExit>) {
    if progress.saved {
        exit.write(AppExit::Success);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_a_ready_step_when_parsing_the_script_then_ready_at_its_frame() {
        assert_eq!(
            parse_script("30:ready"),
            Ok(vec![ScriptStep {
                at_frame: 30,
                input: ScriptedInput::Ready
            }])
        );
    }

    #[test]
    fn given_a_click_on_a_goblin_when_parsing_the_script_then_a_goblin_click() {
        assert_eq!(
            parse_script("60:click H5"),
            Ok(vec![ScriptStep {
                at_frame: 60,
                input: ScriptedInput::Click(BoardClick::Goblin(GoblinId::new(
                    goblins_rules::team::TeamId::HOME,
                    5
                )))
            }])
        );
    }

    #[test]
    fn given_a_click_on_a_square_when_parsing_the_script_then_a_square_click() {
        assert_eq!(
            parse_script("90:click 7,9"),
            Ok(vec![ScriptStep {
                at_frame: 90,
                input: ScriptedInput::Click(BoardClick::Square(Square::new(7, 9)))
            }])
        );
    }

    #[test]
    fn given_a_goblin_done_when_parsing_the_script_then_that_goblin_is_done() {
        assert_eq!(
            parse_script("100:done A6"),
            Ok(vec![ScriptStep {
                at_frame: 100,
                input: ScriptedInput::Done(GoblinId::new(goblins_rules::team::TeamId::AWAY, 6))
            }])
        );
    }

    #[test]
    fn given_an_end_step_when_parsing_the_script_then_the_turn_ends_at_its_frame() {
        assert_eq!(
            parse_script("120:end"),
            Ok(vec![ScriptStep {
                at_frame: 120,
                input: ScriptedInput::EndTurn
            }])
        );
    }

    #[test]
    fn given_a_restart_step_when_parsing_the_script_then_a_restart_at_its_frame() {
        assert_eq!(
            parse_script("600:restart"),
            Ok(vec![ScriptStep {
                at_frame: 600,
                input: ScriptedInput::Restart
            }])
        );
    }

    #[test]
    fn given_several_spaced_steps_when_parsing_the_script_then_all_in_order() {
        assert_eq!(
            parse_script(" 30:ready ; 60:click H5 ;").map(|steps| steps.len()),
            Ok(2)
        );
    }

    #[test]
    fn given_a_step_without_frame_when_parsing_the_script_then_an_error() {
        assert!(parse_script("ready").is_err());
    }

    #[test]
    fn given_an_unknown_input_when_parsing_the_script_then_an_error() {
        assert!(parse_script("30:dance").is_err());
    }

    #[test]
    fn given_no_path_when_building_a_request_then_none() {
        assert_eq!(capture_request_from(None, Some("10".into())), None);
    }

    #[test]
    fn given_blank_path_when_building_a_request_then_none() {
        assert_eq!(capture_request_from(Some("  ".into()), None), None);
    }

    #[test]
    fn given_path_without_frame_when_building_a_request_then_default_frame() {
        assert_eq!(
            capture_request_from(Some("/tmp/shot.png".into()), None),
            Some(CaptureRequest {
                path: PathBuf::from("/tmp/shot.png"),
                at_frame: DEFAULT_CAPTURE_FRAME
            })
        );
    }

    #[test]
    fn given_path_and_frame_when_building_a_request_then_frame_is_used() {
        assert_eq!(
            capture_request_from(Some("/tmp/shot.png".into()), Some("12".into()))
                .map(|request| request.at_frame),
            Some(12)
        );
    }
}
