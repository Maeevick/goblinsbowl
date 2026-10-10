//! Goblins Bowl desktop game: hot-seat goblin football in a bowl, rendered with Bevy.

mod bowl_view;
mod camera_rig;
mod hud;

use bevy::prelude::*;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.12, 0.14, 0.2)))
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Goblins Bowl".into(),
                    ..default()
                }),
                ..default()
            }),
            MeshPickingPlugin,
            bowl_view::BowlViewPlugin,
            camera_rig::CameraRigPlugin,
            hud::HudPlugin,
        ))
        .run();
}
