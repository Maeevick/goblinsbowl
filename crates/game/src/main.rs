//! Goblins Bowl desktop game: hot-seat goblin football in a bowl, rendered with Bevy.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use bevy::prelude::*;
use clap::Parser;
use goblins_rules::event::GameEvent;
use goblins_rules::host::MatchHost;
use goblins_rules::ruleset::{Ruleset, RulesetError};
use goblins_server::match_server::ServerPacing;
use goblins_server::mcp::{SeatTokens, random_token};

use goblinsbowl::bot_seat::BotSeatPlugin;
use goblinsbowl::local_match::LocalMatchPlugin;
use goblinsbowl::options::{GameOptions, LocalSeat};
use goblinsbowl::orders::OrdersPlugin;
use goblinsbowl::pointer::PointerPlugin;
use goblinsbowl::server_link::{LinkSettings, ServerLink, ServerLinkPlugin};
use goblinsbowl::texts::Texts;
use goblinsbowl::{
    bowl_view, camera_rig, dev_capture, dice_view, final_whistle, goblin_cards, goblin_view, hud,
};

enum MatchSource {
    Local(Box<MatchHost>, Vec<GameEvent>),
    Linked(Arc<ServerLink>),
}

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
    let bot = options.bot;
    let record_to = options.local_record_dir();
    let bot_pause = Duration::from_millis(800);
    let (host, opening_events) = match MatchHost::start(options.match_setup(ruleset, seed)) {
        Ok(started) => started,
        Err(problem) => {
            eprintln!("the match cannot start: {problem}");
            return AppExit::from_code(2);
        }
    };
    let source = if options.has_agent() {
        let tokens = SeatTokens {
            home: options.home_token.clone().unwrap_or_else(random_token),
            away: options.away_token.clone().unwrap_or_else(random_token),
        };
        let settings = LinkSettings {
            port: options.port,
            tokens: tokens.clone(),
            hints: options.odds_hints,
            pacing: ServerPacing {
                bot_delay: bot_pause,
                bot,
                bot_seed: seed.wrapping_add(1),
                ..ServerPacing::default()
            },
            record_dir: options.record_dir.clone(),
            seed,
        };
        match ServerLink::start(host, settings) {
            Ok(link) => {
                print_connection_help(&options, &link, &tokens);
                MatchSource::Linked(Arc::new(link))
            }
            Err(problem) => {
                eprintln!("the agent server cannot start: {problem:#}");
                return AppExit::from_code(2);
            }
        }
    } else {
        MatchSource::Local(Box::new(host), opening_events)
    };
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgb(0.12, 0.14, 0.2)))
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
        ));
    match source {
        MatchSource::Linked(link) => app.add_plugins(ServerLinkPlugin { link }),
        MatchSource::Local(host, opening_events) => app.add_plugins((
            LocalMatchPlugin {
                host: *host,
                opening_events,
                record_to,
            },
            BotSeatPlugin {
                bot,
                seed: seed.wrapping_add(1),
                pause: bot_pause,
            },
        )),
    };
    app.add_plugins((
        bowl_view::BowlViewPlugin,
        goblin_view::GoblinViewPlugin,
        goblin_cards::GoblinCardsPlugin,
        dice_view::DiceViewPlugin,
        OrdersPlugin,
        PointerPlugin,
        camera_rig::CameraRigPlugin,
        hud::HudPlugin,
        final_whistle::FinalWhistlePlugin,
        dev_capture::DevCapturePlugin,
    ))
    .run()
}

fn print_connection_help(options: &GameOptions, link: &ServerLink, tokens: &SeatTokens) {
    let url = link.url();
    println!("Goblins Bowl: agents coach through {url}");
    for (name, seat, token) in [
        ("home", options.home, &tokens.home),
        ("away", options.away, &tokens.away),
    ] {
        if seat == LocalSeat::Agent {
            println!(
                "  {name} seat: agent, connect Claude Code with\n    claude mcp add --transport http goblinsbowl-{name} {url} --header \"Authorization: Bearer {token}\"\n  or coach it headless with\n    scripts/coach.sh {name} {url} {token} runs/window"
            );
        }
    }
    let files = link.files();
    println!(
        "  record: {}\n  tool calls: {}",
        files.record.display(),
        files.calls.display()
    );
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
