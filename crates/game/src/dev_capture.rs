//! Development aid: when `GOBLINSBOWL_SCREENSHOT` names an image path, the game renders its camera
//! (3D scene and HUD) into an offscreen image, saves it there after a few frames, then quits.
//! Rendering offscreen keeps captures working while the display is asleep or locked.

use std::path::PathBuf;

use bevy::camera::RenderTarget;
use bevy::diagnostic::FrameCount;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::render::view::window::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk};
use bevy::ui::IsDefaultUiCamera;

const SCREENSHOT_PATH_VARIABLE: &str = "GOBLINSBOWL_SCREENSHOT";
const SCREENSHOT_FRAME_VARIABLE: &str = "GOBLINSBOWL_SCREENSHOT_FRAME";
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
