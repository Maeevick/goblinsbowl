//! A 3/4 top-down camera orbiting a focus point on the floor: yaw, pan and zoom, never left-drag.

use std::f32::consts::{FRAC_PI_2, TAU};

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use goblins_rules::team::TeamId;

use crate::local_match::{MatchRestarted, MatchView};

const YAW_KEY_SPEED: f32 = 1.6;
const YAW_DRAG_SPEED: f32 = 0.006;
const PAN_KEY_SPEED: f32 = 8.0;
const PAN_DRAG_SPEED: f32 = 0.02;
const ZOOM_LINE_STEP: f32 = 1.2;
const ZOOM_PIXEL_STEP: f32 = 0.02;
const MIN_DISTANCE: f32 = 7.0;
const MAX_DISTANCE: f32 = 44.0;
const MAX_FOCUS_OFFSET: f32 = 12.0;
const DEFAULT_DISTANCE: f32 = 28.0;
const TINY_BOWL_LENGTH: f32 = 21.0;

/// Orbit parameters of the camera: a floor focus point seen from `distance`,
/// turned by `yaw` around the vertical axis and tilted `pitch` radians above the horizon.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct CameraRig {
    /// Point of the floor the camera looks at.
    pub focus: Vec3,
    /// Rotation around the vertical axis; 0 looks north from the south side of the bowl.
    pub yaw: f32,
    /// Angle above the horizon, kept below vertical.
    pub pitch: f32,
    /// Distance between the camera and the focus.
    pub distance: f32,
    /// Size of the bowl against a tiny one: distances and pans grow with it.
    pub scale: f32,
}

impl Default for CameraRig {
    fn default() -> Self {
        Self {
            focus: Vec3::new(0.0, 0.0, 1.0),
            yaw: 0.0,
            pitch: 1.0,
            distance: DEFAULT_DISTANCE,
            scale: 1.0,
        }
    }
}

impl CameraRig {
    /// Camera transform for this rig, looking at the focus point.
    pub fn camera_transform(&self) -> Transform {
        let pitch = self.pitch.clamp(0.1, FRAC_PI_2 - 0.05);
        let horizontal_reach = self.distance * pitch.cos();
        let offset = Vec3::new(
            horizontal_reach * self.yaw.sin(),
            self.distance * pitch.sin(),
            horizontal_reach * self.yaw.cos(),
        );
        Transform::from_translation(self.focus + offset).looking_at(self.focus, Vec3::Y)
    }

    /// The default rig for a bowl `width` by `height` squares, far enough to see it whole.
    pub fn for_bowl(width: u8, height: u8) -> Self {
        let scale = (f32::from(width.max(height)) / TINY_BOWL_LENGTH).max(1.0);
        Self {
            distance: DEFAULT_DISTANCE * scale,
            scale,
            ..Self::default()
        }
    }

    /// The default rig of this bowl moved behind `team`'s goblins, looking towards the den they
    /// attack.
    pub fn behind(self, team: TeamId) -> Self {
        let yaw = if team == TeamId::HOME {
            3.0 * FRAC_PI_2
        } else {
            FRAC_PI_2
        };
        Self {
            focus: Vec3::ZERO,
            yaw,
            ..Self::default().with_scale(self.scale)
        }
    }

    /// The default rig of a bowl `scale` times as big as a tiny one.
    fn with_scale(self, scale: f32) -> Self {
        Self {
            distance: DEFAULT_DISTANCE * scale,
            scale,
            ..self
        }
    }

    /// The rig turned by `angle` radians around the vertical axis, yaw kept within one turn.
    pub fn turned(self, angle: f32) -> Self {
        Self {
            yaw: (self.yaw + angle).rem_euclid(TAU),
            ..self
        }
    }

    /// The rig moved closer (positive `steps`) or further away, distance kept within bounds.
    pub fn zoomed(self, steps: f32) -> Self {
        Self {
            distance: (self.distance - steps).clamp(MIN_DISTANCE, MAX_DISTANCE * self.scale),
            ..self
        }
    }

    /// The rig whose focus slid on the floor by `screen_motion`
    /// (x towards the screen's right, y towards the screen's top), focus kept near the bowl.
    pub fn panned(self, screen_motion: Vec2) -> Self {
        let right = Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin());
        let forward = Vec3::new(-self.yaw.sin(), 0.0, -self.yaw.cos());
        let focus = self.focus + right * screen_motion.x + forward * screen_motion.y;
        let reach = MAX_FOCUS_OFFSET * self.scale;
        Self {
            focus: focus.clamp(Vec3::new(-reach, 0.0, -reach), Vec3::new(reach, 0.0, reach)),
            ..self
        }
    }
}

/// Spawns the 3D camera and steers it from the keyboard and mouse.
pub struct CameraRigPlugin;

impl Plugin for CameraRigPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<MatchRestarted>()
            .init_resource::<CameraRig>()
            .add_systems(Startup, (fit_the_bowl, spawn_camera).chain())
            .add_systems(Update, (look_at_a_new_match, steer_rig, apply_rig).chain());
    }
}

fn fit_the_bowl(view: Res<MatchView>, mut rig: ResMut<CameraRig>) {
    *rig = CameraRig::for_bowl(view.arena.width(), view.arena.height());
}

fn look_at_a_new_match(
    mut restarts: MessageReader<MatchRestarted>,
    view: Res<MatchView>,
    mut rig: ResMut<CameraRig>,
) {
    if restarts.read().count() > 0 {
        *rig = CameraRig::for_bowl(view.arena.width(), view.arena.height());
    }
}

fn spawn_camera(mut commands: Commands, rig: Res<CameraRig>) {
    commands.spawn((Camera3d::default(), rig.camera_transform()));
}

fn steer_rig(
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
    time: Res<Time>,
    mut rig: ResMut<CameraRig>,
) {
    if keys.just_pressed(KeyCode::KeyR) {
        *rig = CameraRig::default().with_scale(rig.scale);
        return;
    }
    let seconds = time.delta_secs();
    let key_turn = axis(&keys, KeyCode::KeyE, KeyCode::KeyQ);
    let drag_turn = if mouse_buttons.pressed(MouseButton::Right) {
        -mouse_motion.delta.x * YAW_DRAG_SPEED
    } else {
        0.0
    };
    let key_pan = Vec2::new(
        axis(&keys, KeyCode::KeyD, KeyCode::KeyA)
            + axis(&keys, KeyCode::ArrowRight, KeyCode::ArrowLeft),
        axis(&keys, KeyCode::KeyW, KeyCode::KeyS)
            + axis(&keys, KeyCode::ArrowUp, KeyCode::ArrowDown),
    ) * PAN_KEY_SPEED
        * seconds;
    let drag_pan = if mouse_buttons.pressed(MouseButton::Middle) {
        Vec2::new(-mouse_motion.delta.x, mouse_motion.delta.y) * PAN_DRAG_SPEED
    } else {
        Vec2::ZERO
    };
    let zoom_steps = match mouse_scroll.unit {
        MouseScrollUnit::Line => mouse_scroll.delta.y * ZOOM_LINE_STEP,
        MouseScrollUnit::Pixel => mouse_scroll.delta.y * ZOOM_PIXEL_STEP,
    };
    *rig = rig
        .turned(key_turn * YAW_KEY_SPEED * seconds + drag_turn)
        .panned(key_pan + drag_pan)
        .zoomed(zoom_steps);
}

fn axis(keys: &ButtonInput<KeyCode>, positive: KeyCode, negative: KeyCode) -> f32 {
    f32::from(u8::from(keys.pressed(positive))) - f32::from(u8::from(keys.pressed(negative)))
}

fn apply_rig(rig: Res<CameraRig>, mut camera: Single<&mut Transform, With<Camera3d>>) {
    if rig.is_changed() {
        **camera = rig.camera_transform();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_default_rig_when_placing_the_camera_then_it_stands_south_of_the_focus() {
        let rig = CameraRig::default();
        assert!(rig.camera_transform().translation.z > rig.focus.z);
    }

    #[test]
    fn given_default_rig_when_placing_the_camera_then_it_stands_above_the_floor() {
        assert!(CameraRig::default().camera_transform().translation.y > 5.0);
    }

    #[test]
    fn given_rig_when_zooming_far_in_then_distance_stops_at_the_minimum() {
        assert_eq!(CameraRig::default().zoomed(100.0).distance, MIN_DISTANCE);
    }

    #[test]
    fn given_rig_when_zooming_far_out_then_distance_stops_at_the_maximum() {
        assert_eq!(CameraRig::default().zoomed(-100.0).distance, MAX_DISTANCE);
    }

    #[test]
    fn given_default_rig_when_panning_forward_then_focus_moves_north() {
        let rig = CameraRig::default();
        assert!(rig.panned(Vec2::new(0.0, 1.0)).focus.z < rig.focus.z);
    }

    #[test]
    fn given_default_rig_when_panning_right_then_focus_moves_east() {
        let rig = CameraRig::default();
        assert!(rig.panned(Vec2::new(1.0, 0.0)).focus.x > rig.focus.x);
    }

    #[test]
    fn given_rig_when_panning_far_away_then_focus_stays_near_the_bowl() {
        let focus = CameraRig::default().panned(Vec2::new(100.0, 0.0)).focus;
        assert_eq!(focus.x, MAX_FOCUS_OFFSET);
    }

    #[test]
    fn given_rig_turned_a_quarter_when_placing_the_camera_then_it_stands_east_of_the_focus() {
        let rig = CameraRig::default().turned(FRAC_PI_2);
        assert!(rig.camera_transform().translation.x > rig.focus.x);
    }

    #[test]
    fn given_a_huge_bowl_when_framing_it_then_the_camera_stands_further_and_pans_further() {
        let rig = CameraRig::for_bowl(51, 51);
        assert!(
            rig.distance > CameraRig::default().distance
                && rig.panned(Vec2::new(100.0, 0.0)).focus.x > MAX_FOCUS_OFFSET
        );
    }

    #[test]
    fn given_the_home_team_when_standing_behind_it_then_the_camera_is_west_of_the_focus() {
        let rig = CameraRig::default().behind(TeamId::HOME);
        assert!(rig.camera_transform().translation.x < rig.focus.x);
    }

    #[test]
    fn given_the_away_team_when_standing_behind_it_then_the_camera_is_east_of_the_focus() {
        let rig = CameraRig::default().behind(TeamId::AWAY);
        assert!(rig.camera_transform().translation.x > rig.focus.x);
    }
}
