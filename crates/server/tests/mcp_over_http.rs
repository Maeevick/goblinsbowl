//! Integrated checks: an MCP client coaching over Streamable HTTP on a local port.

use std::collections::HashMap;

use goblins_rules::host::{MatchHost, MatchSetup, SeatKind};
use goblins_rules::roster::Roster;
use goblins_rules::ruleset::Ruleset;
use goblins_rules::team::TeamId;
use goblins_server::match_server::{MatchClient, ServerPacing, spawn_match};
use goblins_server::mcp::{SeatTokens, serve_mcp};
use goblins_server::view::OddsHints;
use rmcp::ServiceExt;
use rmcp::model::{CallToolRequestParams, CallToolResult, ClientConfig};
use rmcp::service::{RoleClient, RunningService};
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use serde_json::{Value, json};

type Outcome<T> = Result<T, Box<dyn std::error::Error>>;

const HOME_TOKEN: &str = "home-secret";
const AWAY_TOKEN: &str = "away-secret";

struct Running {
    url: String,
    client: MatchClient,
}

async fn start_server() -> Outcome<Running> {
    let (host, _) = MatchHost::start(MatchSetup {
        ruleset: Ruleset::default(),
        rosters: [Roster::DEFAULT, Roster::DEFAULT],
        seed: 42,
        seats: [SeatKind::Agent, SeatKind::Agent],
        turn_seconds: [600, 600],
        agent_ready_grace_seconds: 15,
    })?;
    let client = spawn_match(host, ServerPacing::default());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let url = format!("http://{}/mcp", listener.local_addr()?);
    let tokens = SeatTokens {
        home: HOME_TOKEN.to_string(),
        away: AWAY_TOKEN.to_string(),
    };
    tokio::spawn(serve_mcp(listener, client.clone(), tokens, OddsHints::Full));
    Ok(Running { url, client })
}

async fn coach(url: &str, token: &str) -> Outcome<RunningService<RoleClient, ClientConfig>> {
    let transport = StreamableHttpClientTransport::from_config(
        StreamableHttpClientTransportConfig::with_uri(url).auth_header(token),
    );
    Ok(ClientConfig::default().serve(transport).await?)
}

async fn call(
    coach: &RunningService<RoleClient, ClientConfig>,
    tool: &'static str,
    arguments: Value,
) -> Outcome<CallToolResult> {
    let arguments = arguments.as_object().cloned().unwrap_or_default();
    Ok(coach
        .call_tool(CallToolRequestParams::new(tool).with_arguments(arguments))
        .await?)
}

fn token_of(team: TeamId) -> &'static str {
    if team == TeamId::HOME {
        HOME_TOKEN
    } else {
        AWAY_TOKEN
    }
}

fn structured(result: &CallToolResult) -> Value {
    result.structured_content.clone().unwrap_or_default()
}

fn text_of(result: &CallToolResult) -> String {
    serde_json::to_string(&result.content).unwrap_or_default()
}

#[tokio::test]
async fn given_a_coach_when_listing_tools_then_wait_state_rules_and_play() -> Outcome<()> {
    let server = start_server().await?;
    let coach = coach(&server.url, HOME_TOKEN).await?;
    let mut names: Vec<String> = coach
        .list_all_tools()
        .await?
        .into_iter()
        .map(|tool| tool.name.to_string())
        .collect();
    names.sort();
    assert_eq!(
        names,
        ["get_rules", "get_state", "play", "wait_for_my_turn"]
    );
    Ok(())
}

#[tokio::test]
async fn given_the_home_token_when_reading_the_state_then_the_view_is_from_the_home_seat()
-> Outcome<()> {
    let server = start_server().await?;
    let coach = coach(&server.url, HOME_TOKEN).await?;
    let view = structured(&call(&coach, "get_state", json!({})).await?);
    assert!(
        view["you"]
            .as_str()
            .unwrap_or_default()
            .starts_with("home (H)")
    );
    Ok(())
}

#[tokio::test]
async fn given_the_playing_seat_when_it_ends_its_turn_then_its_view_says_waiting() -> Outcome<()> {
    let server = start_server().await?;
    let team = server.client.latest().state.active_team().unwrap();
    let coach = coach(&server.url, token_of(team)).await?;
    let report =
        structured(&call(&coach, "play", json!({"activations": [], "end_turn": true})).await?);
    assert_eq!(report["view"]["status"], "waiting");
    Ok(())
}

#[tokio::test]
async fn given_the_idle_seat_when_it_plays_then_a_visible_tool_error() -> Outcome<()> {
    let server = start_server().await?;
    let idle = server
        .client
        .latest()
        .state
        .active_team()
        .unwrap()
        .duel_opponent();
    let coach = coach(&server.url, token_of(idle)).await?;
    let result = call(&coach, "play", json!({"activations": [], "end_turn": true})).await?;
    assert_eq!(
        (result.is_error, text_of(&result).contains("not your turn")),
        (Some(true), true)
    );
    Ok(())
}

#[tokio::test]
async fn given_a_stale_version_when_playing_then_refused() -> Outcome<()> {
    let server = start_server().await?;
    let team = server.client.latest().state.active_team().unwrap();
    let coach = coach(&server.url, token_of(team)).await?;
    let result = call(
        &coach,
        "play",
        json!({"activations": [], "end_turn": true, "expected_version": 99}),
    )
    .await?;
    assert!(text_of(&result).contains("stale plan"));
    Ok(())
}

#[tokio::test]
async fn given_a_dry_run_when_planning_a_stance_then_certain_odds_and_nothing_changes()
-> Outcome<()> {
    let server = start_server().await?;
    let team = server.client.latest().state.active_team().unwrap();
    let first_goblin = if team == TeamId::HOME { "H5" } else { "A5" };
    let coach = coach(&server.url, token_of(team)).await?;
    let report = structured(
        &call(
            &coach,
            "play",
            json!({"activations": [{"goblin": first_goblin, "action": {"type": "stance"}}], "dry_run": true}),
        )
        .await?,
    );
    assert_eq!(
        (
            report["activations"][0]["odds"].clone(),
            report["view"]["version"].clone()
        ),
        (json!(["no knockdown nor fumble: 100.0 % (1/1)"]), json!(0))
    );
    Ok(())
}

#[tokio::test]
async fn given_a_missing_token_when_connecting_then_refused() -> Outcome<()> {
    let server = start_server().await?;
    let transport = StreamableHttpClientTransport::from_config(
        StreamableHttpClientTransportConfig::with_uri(server.url),
    );
    assert!(ClientConfig::default().serve(transport).await.is_err());
    Ok(())
}

#[tokio::test]
async fn given_a_foreign_browser_origin_when_connecting_then_refused() -> Outcome<()> {
    let server = start_server().await?;
    let headers = HashMap::from([(
        axum::http::HeaderName::from_static("origin"),
        axum::http::HeaderValue::from_static("https://evil.example"),
    )]);
    let transport = StreamableHttpClientTransport::from_config(
        StreamableHttpClientTransportConfig::with_uri(server.url)
            .auth_header(HOME_TOKEN)
            .custom_headers(headers),
    );
    assert!(ClientConfig::default().serve(transport).await.is_err());
    Ok(())
}

#[tokio::test]
async fn given_an_agent_whose_turn_waits_when_it_reads_the_state_then_its_clock_starts()
-> Outcome<()> {
    let server = start_server().await?;
    let team = server.client.latest().state.active_team().unwrap();
    let coach = coach(&server.url, token_of(team)).await?;
    call(&coach, "get_state", json!({})).await?;
    assert!(matches!(
        server.client.latest().clock,
        goblins_rules::host::TurnClock::Running { .. }
    ));
    Ok(())
}
