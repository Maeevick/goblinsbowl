//! Headless Goblins Bowl match server: AI agents coach through MCP at `http://127.0.0.1:<port>/mcp`.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::path::Path;
use std::time::Duration;

use anyhow::Context;
use clap::Parser;
use goblins_rules::bowls::with_bowl;
use goblins_rules::host::{MatchHost, MatchSetup};
use goblins_rules::ruleset::Ruleset;
use goblins_server::match_server::{ServerPacing, spawn_match};
use goblins_server::mcp::{SeatTokens, random_token, serve_mcp};
use goblins_server::options::{RemoteSeat, ServerOptions};
use goblins_server::runs::{
    RunFiles, save_record, save_record_at_final_whistle, seconds_since_epoch,
};

const SERVER_TICK: Duration = Duration::from_millis(100);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let options = ServerOptions::parse();
    let seed = options.seed.unwrap_or_else(seed_from_clock);
    let ruleset = with_bowl(
        load_ruleset(&options.rules)?,
        options.arena,
        options.arena_size,
    );
    let tokens = SeatTokens {
        home: options.home_token.clone().unwrap_or_else(random_token),
        away: options.away_token.clone().unwrap_or_else(random_token),
    };
    let setup = MatchSetup {
        rosters: [ruleset.default_roster; 2],
        ruleset,
        seed,
        seats: [options.home.into(), options.away.into()],
        turn_seconds: options.turn_seconds_per_seat(),
        agent_ready_grace_seconds: options.agent_grace_seconds,
    };
    let (host, _) = MatchHost::start(setup).context("the match cannot start")?;
    let (files, log) = RunFiles::open(&options.record_dir, seed)?;
    let client = spawn_match(
        host,
        ServerPacing {
            tick: SERVER_TICK,
            bot_delay: Duration::from_millis(options.bot_delay_ms),
            bot: options.bot,
            bot_seed: seed.wrapping_add(1),
        },
    );
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", options.port))
        .await
        .with_context(|| format!("cannot listen on 127.0.0.1:{}", options.port))?;
    print_connection_help(&options, seed, &tokens);
    println!(
        "  record: {}\n  tool calls: {}",
        files.record.display(),
        files.calls.display()
    );
    tokio::spawn(save_record_at_final_whistle(
        client.clone(),
        files.record.clone(),
    ));
    tokio::select! {
        served = serve_mcp(listener, client.clone(), tokens, options.odds_hints, log) => {
            served.context("the MCP server stopped")
        }
        () = shutdown_signal() => {
            save_record(&client, &files.record, "Stopped").await;
            Ok(())
        }
    }
}

async fn shutdown_signal() {
    let interrupted = tokio::signal::ctrl_c();
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        match signal(SignalKind::terminate()) {
            Ok(mut terminated) => {
                tokio::select! {
                    _ = interrupted => {}
                    _ = terminated.recv() => {}
                }
            }
            Err(_) => {
                let _ = interrupted.await;
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = interrupted.await;
    }
}

fn load_ruleset(path: &Path) -> anyhow::Result<Ruleset> {
    if !path.exists() {
        return Ok(Ruleset::default());
    }
    let text =
        std::fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
    Ruleset::from_toml_str(&text).with_context(|| format!("invalid rules in {}", path.display()))
}

fn print_connection_help(options: &ServerOptions, seed: u64, tokens: &SeatTokens) {
    let url = format!("http://127.0.0.1:{}/mcp", options.port);
    println!(
        "Goblins Bowl match server on {url} (seed {seed}, odds hints {:?})",
        options.odds_hints
    );
    for (name, seat, token) in [
        ("home", options.home, &tokens.home),
        ("away", options.away, &tokens.away),
    ] {
        match seat {
            RemoteSeat::Bot => println!("  {name} seat: built-in bot"),
            RemoteSeat::Agent => println!(
                "  {name} seat: agent, connect with\n    claude mcp add --transport http goblinsbowl-{name} {url} --header \"Authorization: Bearer {token}\""
            ),
        }
    }
}

fn seed_from_clock() -> u64 {
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u64(seconds_since_epoch());
    hasher.finish()
}
