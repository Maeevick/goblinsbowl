//! Goblins Bowl desktop game: hot-seat goblin football in a bowl, rendered with Bevy.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::path::Path;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use bevy::prelude::*;
use clap::Parser;
use goblins_rules::host::MatchHost;
use goblins_rules::ruleset::{Ruleset, RulesetError};

use goblinsbowl::bot_seat::BotSeatPlugin;
use goblinsbowl::local_match::LocalMatchPlugin;
use goblinsbowl::options::GameOptions;
use goblinsbowl::orders::OrdersPlugin;
use goblinsbowl::texts::Texts;
use goblinsbowl::{bowl_view, camera_rig, dev_capture, goblin_view, hud};

fn main() -> AppExit {
    let options = GameOptions::parse();
    let ruleset = match load_ruleset(&options.rules) {
        Ok(ruleset) => ruleset,
        Err(problem) => {
            eprintln!("{problem}");
            return AppExit::from_code(2);
        }
    };
    let seed = options.seed.unwrap_or_else(seed_from_clock);
    let (host, opening_events) = match MatchHost::start(options.match_setup(ruleset, seed)) {
        Ok(started) => started,
        Err(problem) => {
            eprintln!("the match cannot start: {problem}");
            return AppExit::from_code(2);
        }
    };
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.12, 0.14, 0.2)))
        .insert_resource(Texts {
            language: options.lang,
        })
        .insert_resource(options)
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Goblins Bowl".into(),
                    ..default()
                }),
                ..default()
            }),
            MeshPickingPlugin,
            LocalMatchPlugin {
                host,
                opening_events,
            },
            BotSeatPlugin {
                seed: seed.wrapping_add(1),
                pause: Duration::from_millis(800),
            },
            bowl_view::BowlViewPlugin,
            goblin_view::GoblinViewPlugin,
            OrdersPlugin,
            camera_rig::CameraRigPlugin,
            hud::HudPlugin,
            dev_capture::DevCapturePlugin,
        ))
        .run()
}

fn load_ruleset(path: &Path) -> Result<Ruleset, String> {
    if !path.exists() {
        return Ok(Ruleset::default());
    }
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    Ruleset::from_toml_str(&text)
        .map_err(|error: RulesetError| format!("invalid rules in {}: {error}", path.display()))
}

fn seed_from_clock() -> u64 {
    let mut hasher = RandomState::new().build_hasher();
    let nanoseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or_default();
    hasher.write_u128(nanoseconds);
    hasher.finish()
}
