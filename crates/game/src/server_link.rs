//! A match hosted by an embedded server, for games with an AI agent: agents coach through MCP and
//! bots play on the server, while the window shows the match and forwards the humans' orders.
//!
//! The server runs on its own runtime so agent calls never wait for the window to draw a frame.

use std::path::PathBuf;
use std::sync::Arc;

use bevy::prelude::*;
use goblins_rules::host::MatchHost;
use goblins_server::match_server::{MatchClient, ServerPacing, spawn_match};
use goblins_server::mcp::{SeatTokens, serve_mcp};
use goblins_server::runs::{RunFiles, save_record, save_record_at_final_whistle};
use goblins_server::view::OddsHints;
use tokio::runtime::Runtime;

use crate::local_match::{MatchOrder, MatchSystems, MatchView};

/// How the embedded server is set up.
#[derive(Clone, Debug)]
pub struct LinkSettings {
    /// Local port of the MCP endpoint; 0 picks a free one.
    pub port: u16,
    /// Secrets of the seats.
    pub tokens: SeatTokens,
    /// How much odds information agents receive.
    pub hints: OddsHints,
    /// Clock ticks and bot pacing.
    pub pacing: ServerPacing,
    /// Folder of the match record and the tool calls.
    pub record_dir: PathBuf,
    /// Dice seed of the match, used to name its files.
    pub seed: u64,
}

/// A running embedded server and the way to reach it.
pub struct ServerLink {
    runtime: Runtime,
    client: MatchClient,
    url: String,
    files: RunFiles,
}

impl ServerLink {
    /// Starts `host` on a background runtime and opens its MCP endpoint on 127.0.0.1.
    ///
    /// # Errors
    /// The runtime, the run files or the listening socket cannot be set up.
    pub fn start(host: MatchHost, settings: LinkSettings) -> anyhow::Result<Self> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("goblinsbowl-server")
            .enable_all()
            .build()?;
        let (files, log) = RunFiles::open(&settings.record_dir, settings.seed)?;
        let (client, url) = runtime.block_on(async {
            let client = spawn_match(host, settings.pacing);
            let listener = tokio::net::TcpListener::bind(("127.0.0.1", settings.port)).await?;
            let url = format!("http://{}/mcp", listener.local_addr()?);
            tokio::spawn(serve_mcp(
                listener,
                client.clone(),
                settings.tokens,
                settings.hints,
                log,
            ));
            tokio::spawn(save_record_at_final_whistle(
                client.clone(),
                files.record.clone(),
            ));
            tokio::spawn(save_record_when_interrupted(
                client.clone(),
                files.record.clone(),
            ));
            anyhow::Ok((client, url))
        })?;
        Ok(Self {
            runtime,
            client,
            url,
            files,
        })
    }

    /// Address of the MCP endpoint.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Where the match record and the tool calls are saved.
    pub fn files(&self) -> &RunFiles {
        &self.files
    }
}

async fn save_record_when_interrupted(client: MatchClient, record: PathBuf) {
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
    save_record(&client, &record, "Stopped").await;
    std::process::exit(0);
}

/// Shows a match hosted by an embedded server and forwards the humans' orders to it.
pub struct ServerLinkPlugin {
    /// The running server.
    pub link: Arc<ServerLink>,
}

#[derive(Resource)]
struct LinkedMatch {
    link: Arc<ServerLink>,
    cursor: usize,
    seen_version: Option<u64>,
}

impl Plugin for ServerLinkPlugin {
    fn build(&self, app: &mut App) {
        let client = &self.link.client;
        let view = MatchView {
            snapshot: (*client.latest()).clone(),
            arena: client.arena().clone(),
            ruleset: client.ruleset().clone(),
            fresh_events: Vec::new(),
            refusal: None,
        };
        app.add_message::<MatchOrder>()
            .insert_resource(view)
            .insert_resource(LinkedMatch {
                link: self.link.clone(),
                cursor: 0,
                seen_version: None,
            })
            .configure_sets(Update, (MatchSystems::Apply, MatchSystems::Present).chain())
            .add_systems(
                Update,
                (forward_orders, follow_the_server)
                    .chain()
                    .in_set(MatchSystems::Apply),
            )
            .add_systems(Last, save_on_exit);
    }
}

fn forward_orders(
    mut orders: MessageReader<MatchOrder>,
    linked: Res<LinkedMatch>,
    mut view: ResMut<MatchView>,
) {
    view.fresh_events.clear();
    view.refusal = None;
    let link = &linked.link;
    for order in orders.read() {
        let outcome = match order {
            MatchOrder::Command { team, command } => link
                .runtime
                .block_on(link.client.submit(*team, command.clone()))
                .map(|_| ())
                .map_err(|refusal| refusal.to_string()),
            MatchOrder::Ready { team } => link
                .runtime
                .block_on(link.client.ready(*team))
                .map_err(|refusal| refusal.to_string()),
            MatchOrder::TogglePause => {
                Err("the clock cannot be paused against an agent".to_string())
            }
        };
        if let Err(refusal) = outcome {
            view.refusal = Some(refusal);
        }
    }
}

fn follow_the_server(mut linked: ResMut<LinkedMatch>, mut view: ResMut<MatchView>) {
    let link = linked.link.clone();
    let snapshot = link.client.latest();
    if linked.seen_version != Some(snapshot.version) {
        match link
            .runtime
            .block_on(link.client.events_since(linked.cursor))
        {
            Ok((events, cursor)) => {
                view.fresh_events.extend(events);
                linked.cursor = cursor;
                linked.seen_version = Some(snapshot.version);
            }
            Err(problem) => view.refusal = Some(problem.to_string()),
        }
    }
    view.snapshot = (*snapshot).clone();
}

fn save_on_exit(mut exits: MessageReader<AppExit>, linked: Res<LinkedMatch>) {
    if exits.read().next().is_some() {
        let link = &linked.link;
        link.runtime
            .block_on(save_record(&link.client, &link.files.record, "Stopped"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use goblins_rules::command::Command;
    use goblins_rules::event::GameEvent;
    use goblins_rules::host::{MatchSetup, SeatKind};
    use goblins_rules::roster::Roster;
    use goblins_rules::ruleset::Ruleset;
    use goblins_rules::team::TeamId;

    fn linked_app() -> App {
        let (host, _) = MatchHost::start(MatchSetup {
            ruleset: Ruleset::default(),
            rosters: [Roster::DEFAULT, Roster::DEFAULT],
            seed: 42,
            seats: [SeatKind::Human, SeatKind::Human],
            turn_seconds: [600, 600],
            agent_ready_grace_seconds: 15,
        })
        .unwrap();
        let link = ServerLink::start(
            host,
            LinkSettings {
                port: 0,
                tokens: SeatTokens {
                    home: "home-secret".to_string(),
                    away: "away-secret".to_string(),
                },
                hints: OddsHints::Full,
                pacing: ServerPacing::default(),
                record_dir: std::env::temp_dir()
                    .join(format!("goblinsbowl-link-{}", std::process::id())),
                seed: 42,
            },
        )
        .unwrap();
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            ServerLinkPlugin {
                link: Arc::new(link),
            },
        ));
        app.update();
        app
    }

    fn view(app: &App) -> &MatchView {
        app.world().resource::<MatchView>()
    }

    #[test]
    fn given_a_linked_match_when_the_first_frame_runs_then_the_opening_events_are_fresh() {
        let app = linked_app();
        assert!(matches!(
            view(&app).fresh_events.first(),
            Some(GameEvent::CoinTossed { .. })
        ));
    }

    #[test]
    fn given_a_linked_match_when_the_human_ends_the_turn_then_the_other_team_plays() {
        let mut app = linked_app();
        let team = view(&app).snapshot.state.active_team().unwrap();
        app.world_mut().write_message(MatchOrder::Ready { team });
        app.world_mut().write_message(MatchOrder::Command {
            team,
            command: Command::EndTurn,
        });
        app.update();
        assert_eq!(
            view(&app).snapshot.state.active_team(),
            Some(team.duel_opponent())
        );
    }

    #[test]
    fn given_a_linked_match_when_the_idle_team_orders_then_the_refusal_is_shown() {
        let mut app = linked_app();
        let idle = view(&app)
            .snapshot
            .state
            .active_team()
            .map_or(TeamId::AWAY, TeamId::duel_opponent);
        app.world_mut().write_message(MatchOrder::Command {
            team: idle,
            command: Command::EndTurn,
        });
        app.update();
        assert!(view(&app).refusal.is_some());
    }
}
