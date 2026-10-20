//! Integrated checks: an MCP client coaching over Streamable HTTP on a local port.

use std::collections::HashMap;
use std::io::Write;
use std::sync::{Arc, Mutex};

use goblins_rules::host::{MatchHost, MatchSetup, SeatKind};
use goblins_rules::roster::Roster;
use goblins_rules::ruleset::Ruleset;
use goblins_rules::state::GameState;
use goblins_rules::team::TeamId;
use goblins_server::call_log::{CallEntry, CallLog};
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
const FROZEN_RULES: &str = "# Goblins Bowl, rules 0.10\n\nThe frozen rules of the benchmark.\n";
const AWAY_TOKEN: &str = "away-secret";

#[derive(Clone, Default)]
struct SharedBuffer(Arc<Mutex<Vec<u8>>>);

impl Write for SharedBuffer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .map_err(|_| std::io::Error::other("poisoned"))?
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl SharedBuffer {
    fn entries(&self) -> Outcome<Vec<CallEntry>> {
        let bytes = self
            .0
            .lock()
            .map_err(|_| std::io::Error::other("poisoned"))?
            .clone();
        let text = String::from_utf8(bytes)?;
        Ok(text
            .lines()
            .map(serde_json::from_str)
            .collect::<Result<_, _>>()?)
    }
}

struct Running {
    url: String,
    client: MatchClient,
    calls: SharedBuffer,
}

async fn start_server() -> Outcome<Running> {
    start_server_with(Ruleset::default()).await
}

async fn start_server_with(ruleset: Ruleset) -> Outcome<Running> {
    let (host, _) = MatchHost::start(MatchSetup {
        ruleset,
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
    let calls = SharedBuffer::default();
    tokio::spawn(serve_mcp(
        listener,
        client.clone(),
        tokens,
        OddsHints::Full,
        FROZEN_RULES.into(),
        CallLog::to_writer(calls.clone()),
    ));
    Ok(Running { url, client, calls })
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
async fn given_a_coach_when_listing_tools_then_wait_state_rules_play_and_compare() -> Outcome<()> {
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
        [
            "compare_plans",
            "get_rules",
            "get_state",
            "play",
            "wait_for_my_turn"
        ]
    );
    Ok(())
}

async fn instructions_under(ruleset: Ruleset) -> Outcome<String> {
    let server = start_server_with(ruleset).await?;
    let coach = coach(&server.url, HOME_TOKEN).await?;
    Ok(coach
        .peer_info()
        .and_then(|info| info.instructions.clone())
        .unwrap_or_default())
}

#[tokio::test]
async fn given_rules_where_actions_cost_a_step_when_a_coach_connects_then_its_instructions_say_so()
-> Outcome<()> {
    let instructions = instructions_under(Ruleset {
        actions_cost_a_step: true,
        ..Ruleset::default()
    })
    .await?;

    assert!(instructions.contains("throw once per turn, each costing a step (steps_left"));
    Ok(())
}

#[tokio::test]
async fn given_rules_where_actions_cost_no_step_when_a_coach_connects_then_its_instructions_say_so()
-> Outcome<()> {
    let instructions = instructions_under(Ruleset {
        actions_cost_a_step: false,
        ..Ruleset::default()
    })
    .await?;

    assert!(
        instructions.contains("throw once per turn, neither costing a step (steps_left")
            && !instructions.contains("each costing a step")
    );
    Ok(())
}

const FROZEN_INSTRUCTIONS: &str = "You coach one team of Goblins Bowl, a turn-based goblin football game played with dice. \
Loop: call wait_for_my_turn; when status is your_turn, play your goblins with play (several orders per call, \
resolved in order; add end_turn=true on your last call), then wait again, until status is game_over. \
Each goblin has its movement in steps, spent over as many orders as you like, even after other goblins \
played; it may assault once and throw once per turn, each costing a step (steps_left, actions_left). Assaults come in styles (simple, chirurgical, \
brutal, projection), a goblin that has not moved may throw into chaos, and a Thrower may throw focused, \
every step it has left after the throw sacrificed for +1 to the roll: see the play schema and get_rules. \
Any failed dodge, assault, pick-up or throw, or your own throw not caught, ends your turn at once, \
so order safe moves first and keep risky ones for last. Otherwise your turn ends only with end_turn=true, \
a score, or its clock (time_left_ms) running out. To get exact odds without rolling, play with dry_run=true previews its orders as one plan, each \
from where the previous ones leave the board, and compare_plans weighs alternative plans side by side. \
Squares are [x, y]; goblins are H1..H9 (home, west den) and A1..A9 (away, east den). Call get_rules once \
for the full rules.";

fn frozen_benchmark_rules() -> Outcome<Ruleset> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assessment/rules-0.10.toml");
    Ok(Ruleset::from_toml_str(&std::fs::read_to_string(path)?)?)
}

#[tokio::test]
async fn given_the_frozen_benchmark_rules_when_a_coach_connects_then_it_reads_the_instructions_of_the_benchmark_word_for_word()
-> Outcome<()> {
    let instructions = instructions_under(frozen_benchmark_rules()?).await?;

    assert_eq!(instructions, FROZEN_INSTRUCTIONS);
    Ok(())
}

#[tokio::test]
async fn given_styles_kept_for_some_archetypes_when_a_coach_connects_then_its_instructions_say_who_may_use_which()
-> Outcome<()> {
    let instructions = instructions_under(Ruleset {
        assaults_reach_diagonals: true,
        brutal_and_projection_need_a_bruiser: true,
        aggressions_take_no_style: true,
        only_throwers_throw_into_chaos: true,
        focused_throw_bonus: Some(3),
        failed_focused_throws_down_the_thrower: true,
        ..Ruleset::default()
    })
    .await?;

    assert!(
        instructions.contains("Assaults reach the eight squares around and come in styles (simple, chirurgical; brutal and projection for Bruisers only), an assault on a downed goblin being an aggression that takes no style, a Thrower that has not moved may throw into chaos, and a Thrower that has not moved may throw focused for +3, spending its whole Move and falling if its throw roll misses: see the play schema and get_rules, whose rules prevail over the schema's descriptions."),
        "{instructions}"
    );
    Ok(())
}

#[tokio::test]
async fn given_a_server_serving_frozen_rules_when_a_coach_reads_the_rules_then_it_reads_those()
-> Outcome<()> {
    let server = start_server().await?;
    let coach = coach(&server.url, HOME_TOKEN).await?;
    let result = call(&coach, "get_rules", json!({})).await?;
    let read: Vec<String> = result
        .content
        .iter()
        .filter_map(|content| content.as_text().map(|text| text.text.clone()))
        .collect();
    assert_eq!(read, [FROZEN_RULES]);
    Ok(())
}

#[tokio::test]
async fn given_a_coach_reading_the_state_when_the_call_log_is_read_then_it_holds_that_call_for_its_seat()
-> Outcome<()> {
    let server = start_server().await?;
    let coach = coach(&server.url, HOME_TOKEN).await?;
    call(&coach, "get_state", json!({})).await?;
    let logged: Vec<(String, String)> = server
        .calls
        .entries()?
        .into_iter()
        .map(|entry| (entry.seat, entry.tool))
        .collect();
    assert_eq!(logged, [("home".to_string(), "get_state".to_string())]);
    Ok(())
}

#[tokio::test]
async fn given_a_coach_ending_its_turn_when_the_call_log_is_read_then_the_call_spans_one_more_version()
-> Outcome<()> {
    let server = start_server().await?;
    let team = server.client.latest().state.active_team().unwrap();
    let coach = coach(&server.url, token_of(team)).await?;
    call(&coach, "get_state", json!({})).await?;
    call(&coach, "play", json!({"activations": [], "end_turn": true})).await?;
    let play = server
        .calls
        .entries()?
        .into_iter()
        .find(|entry| entry.tool == "play")
        .ok_or("no play call logged")?;
    assert_eq!(
        Some(play.version),
        play.version_before.map(|before| before + 1)
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
    call(&coach, "get_state", json!({})).await?;
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
async fn given_a_dry_run_when_planning_a_free_walk_then_certain_odds_and_nothing_changes()
-> Outcome<()> {
    let server = start_server().await?;
    let state = server.client.latest().state.clone();
    let team = state.active_team().unwrap();
    let wing = wing_sprinter_of(&state, team)?;
    let coach = coach(&server.url, token_of(team)).await?;
    let report = structured(
        &call(
            &coach,
            "play",
            json!({"activations": [walk(&wing.sprinter, 1, &wing)], "dry_run": true}),
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

struct Wing {
    sprinter: String,
    column: i16,
    towards_own_den: i16,
}

fn wing_sprinter_of(state: &GameState, team: TeamId) -> Outcome<Wing> {
    let (sprinter, square) = state
        .goblins_of(team)
        .find_map(|goblin| {
            goblin
                .square()
                .filter(|square| square.y == 5)
                .map(|square| (goblin.id, square))
        })
        .ok_or("no goblin on the northern wing")?;
    Ok(Wing {
        sprinter: sprinter.to_string(),
        column: i16::from(square.x),
        towards_own_den: if team == TeamId::HOME { -1 } else { 1 },
    })
}

fn walk(goblin: &str, columns_away: i16, wing: &Wing) -> Value {
    json!({"goblin": goblin, "to": [wing.column + wing.towards_own_den * columns_away, 5]})
}

#[tokio::test]
async fn given_a_dry_run_of_two_walks_of_one_sprinter_when_previewed_then_the_second_walks_on_from_where_the_first_ends()
-> Outcome<()> {
    let server = start_server().await?;
    let state = server.client.latest().state.clone();
    let team = state.active_team().unwrap();
    let wing = wing_sprinter_of(&state, team)?;
    let coach = coach(&server.url, token_of(team)).await?;
    let report = structured(
        &call(
            &coach,
            "play",
            json!({"activations": [walk(&wing.sprinter, 2, &wing), walk(&wing.sprinter, 4, &wing)], "dry_run": true}),
        )
        .await?,
    );
    let second_path = report["activations"][1]["path"].clone();
    let expected = json!([
        [wing.column + wing.towards_own_den * 3, 5],
        [wing.column + wing.towards_own_den * 4, 5]
    ]);
    assert_eq!(
        (second_path, report["plan_chance"].clone()),
        (expected, json!("100.0 % (1/1)"))
    );
    Ok(())
}

#[tokio::test]
async fn given_a_dry_run_whose_second_walk_needs_more_steps_than_the_first_left_when_previewed_then_it_is_refused()
-> Outcome<()> {
    let server = start_server().await?;
    let state = server.client.latest().state.clone();
    let team = state.active_team().unwrap();
    let wing = wing_sprinter_of(&state, team)?;
    let coach = coach(&server.url, token_of(team)).await?;
    let report = structured(
        &call(
            &coach,
            "play",
            json!({"activations": [walk(&wing.sprinter, 3, &wing), walk(&wing.sprinter, -1, &wing)], "dry_run": true}),
        )
        .await?,
    );
    let second_outcome = report["activations"][1]["outcome"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    assert!(second_outcome.starts_with("refused") && report["plan_chance"] == json!("0.0 % (0/1)"));
    Ok(())
}

#[tokio::test]
async fn given_two_plans_when_compared_then_each_is_previewed_from_the_board_as_it_stands()
-> Outcome<()> {
    let server = start_server().await?;
    let state = server.client.latest().state.clone();
    let team = state.active_team().unwrap();
    let wing = wing_sprinter_of(&state, team)?;
    let coach = coach(&server.url, token_of(team)).await?;
    let back = walk(&wing.sprinter, 2, &wing);
    let forward = walk(&wing.sprinter, -2, &wing);
    let report = structured(
        &call(
            &coach,
            "compare_plans",
            json!({"plans": [{"orders": [back]}, {"orders": [forward]}]}),
        )
        .await?,
    );
    let forward_path = report["plans"][1]["orders"][0]["path"].clone();
    let expected = json!([
        [wing.column - wing.towards_own_den, 5],
        [wing.column - wing.towards_own_den * 2, 5]
    ]);
    assert_eq!(
        (forward_path, report["plans"][1]["chance"].clone()),
        (expected, json!("100.0 % (1/1)"))
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

#[tokio::test]
async fn given_a_plan_made_on_an_earlier_turn_when_played_then_refused() -> Outcome<()> {
    let server = start_server().await?;
    let team = server.client.latest().state.active_team().unwrap();
    let coach = coach(&server.url, token_of(team)).await?;
    call(&coach, "get_state", json!({})).await?;
    server
        .client
        .submit(team, goblins_rules::command::Command::EndTurn)
        .await?;
    server
        .client
        .submit(
            team.duel_opponent(),
            goblins_rules::command::Command::EndTurn,
        )
        .await?;
    let result = call(&coach, "play", json!({"activations": [], "end_turn": true})).await?;
    assert!(text_of(&result).contains("the turn changed since you last looked"));
    Ok(())
}

#[tokio::test]
async fn given_a_coach_that_never_looked_when_it_plays_then_refused() -> Outcome<()> {
    let server = start_server().await?;
    let team = server.client.latest().state.active_team().unwrap();
    let coach = coach(&server.url, token_of(team)).await?;
    let result = call(&coach, "play", json!({"activations": [], "end_turn": true})).await?;
    assert!(text_of(&result).contains("the turn changed since you last looked"));
    Ok(())
}
