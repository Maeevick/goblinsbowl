//! The MCP face of the match: the tools an AI coach calls over Streamable HTTP.
//!
//! Each request carries a seat token (`Authorization: Bearer ...`); a middleware turns it into the
//! seat the tools act for, so the model never handles secrets and no MCP session is needed.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::{Request, State};
use axum::http::request::Parts;
use axum::http::{StatusCode, header::AUTHORIZATION};
use axum::middleware::{self, Next};
use axum::response::Response;
use goblins_rules::command::{Action, Activation, AssaultStyle, Command, ThrowStyle};
use goblins_rules::event::{GameEvent, TurnEndReason};
use goblins_rules::host::MatchSnapshot;
use goblins_rules::odds::Probability;
use goblins_rules::paths::safest_path;
use goblins_rules::preview::{activation_odds, board_after_success};
use goblins_rules::roster::GoblinId;
use goblins_rules::square::{Compass, Square};
use goblins_rules::state::GameState;
use goblins_rules::team::TeamId;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{Implementation, ServerCapabilities, ServerConfig};
use rmcp::service::RequestContext;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use rmcp::{Json, RoleServer, ServerHandler, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::call_log::{CallEntry, CallLog};
use crate::match_server::MatchClient;
use crate::view::drop_integer_formats;
use crate::view::{CoachView, OddsHints, coach_view, describe_chance, odds_lines};

const LONGEST_WAIT_SECONDS: u64 = 50;
const DEFAULT_WAIT_SECONDS: u64 = 45;
const RULES: &str = include_str!("../../../docs/rules.md");
const ENGLISH_RULES_HEADING: &str = "# Goblins Bowl, rules 0.9";

const INSTRUCTIONS: &str = "You coach one team of Goblins Bowl, a turn-based goblin football game played with dice. \
Loop: call wait_for_my_turn; when status is your_turn, play your goblins with play (several orders per call, \
resolved in order; add end_turn=true on your last call), then wait again, until status is game_over. \
Each goblin has its movement in steps, spent over as many orders as you like, even after other goblins \
played; it may assault once, throw once and brace in stance once per turn, an assault or a throw costing \
a step and a stance every step left (steps_left, actions_left). Assaults come in styles (simple, chirurgical, \
brutal, projection), a goblin that has not moved may throw into chaos and a carrier that has not moved may \
focus for +1 to its throws next turn: see the play schema and get_rules. \
Any failed dodge, assault, pick-up or throw, or your own throw not caught, ends your turn at once, \
so order safe moves first and keep risky ones for last. Otherwise your turn ends only with end_turn=true, \
a score, or its clock (time_left_ms) running out. To get exact odds without rolling, play with dry_run=true previews its orders as one plan, each \
from where the previous ones leave the board, and compare_plans weighs alternative plans side by side. \
Squares are [x, y]; goblins are H1..H9 (home, west den) and A1..A9 (away, east den). Call get_rules once \
for the full rules.";

/// The English half of `docs/rules.md`, from its heading on, as agents read it.
fn english_rules() -> String {
    RULES
        .split_once(ENGLISH_RULES_HEADING)
        .map_or(RULES.to_string(), |(_, english)| {
            format!("{ENGLISH_RULES_HEADING}{english}")
        })
}

/// A fresh seat secret, unpredictable enough for a match on the local machine.
pub fn random_token() -> String {
    let state = std::collections::hash_map::RandomState::new();
    let nanoseconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or_default();
    let half = |salt: u64| {
        use std::hash::{BuildHasher, Hasher};
        let mut hasher = state.build_hasher();
        hasher.write_u64(salt);
        hasher.write_u128(nanoseconds);
        hasher.finish()
    };
    format!("{:016x}{:016x}", half(1), half(2))
}

/// The secret each seat sends as a bearer token.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeatTokens {
    /// Token of the home seat.
    pub home: String,
    /// Token of the away seat.
    pub away: String,
}

#[derive(Clone, Copy, Debug)]
struct Seat(TeamId);

/// What the server remembers about a seat between calls: the events it already saw and the turn it last looked at.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SeatMemory {
    event_cursor: usize,
    turn_seen: Option<TurnMark>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TurnMark {
    half: u8,
    turns_started: u16,
    active_team: Option<TeamId>,
}

fn turn_mark(snapshot: &MatchSnapshot) -> TurnMark {
    TurnMark {
        half: snapshot.state.half,
        turns_started: snapshot
            .state
            .turns_played
            .iter()
            .map(|turns| u16::from(*turns))
            .sum(),
        active_team: snapshot.state.active_team(),
    }
}

/// Arguments of wait_for_my_turn.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[schemars(transform = drop_integer_formats)]
pub struct WaitArgs {
    /// Longest wait in seconds (at most 50, default 45). Call again if the status is still waiting.
    pub max_wait_seconds: Option<u64>,
}

/// A goblin's action: an assault, a throw, a stance or a focus, each once per turn.
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[schemars(transform = drop_integer_formats)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PlannedAction {
    /// Assault an adjacent enemy (standing or down) after moving.
    Assault {
        /// Enemy id such as A3.
        target: String,
        /// How to assault, where the rules allow styles: simple (default), chirurgical (+1 to
        /// win, the target loses no health), brutal (-1 to win, the target loses one more health)
        /// or projection (-1 to win, the target falls on the square `to`).
        style: Option<PlannedAssaultStyle>,
        /// For a projection: where the target falls around the attacker's square, 1 to 8
        /// clockwise from north (1 is [x, y-1], 3 is [x+1, y], 5 is [x, y+1], 7 is [x-1, y]).
        to: Option<u8>,
    },
    /// Throw the ball (held now or picked up on the path) to a standing teammate.
    Throw {
        /// Teammate id such as H5.
        target: String,
        /// How to throw, where the rules allow styles: simple (default) or chaos (no throw roll,
        /// the ball bounces three times from the receiver's square; the thrower must not have
        /// moved this turn and spends its whole move).
        style: Option<PlannedThrowStyle>,
    },
    /// Brace in stance after moving: +1 defense until your next turn, at the cost of every step left.
    Stance,
    /// Focus, where the rules allow it, for the goblin holding the ball: stand still this whole
    /// turn for +1 to its throws on your next turn.
    Focus,
}

/// How an assault is thrown.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PlannedAssaultStyle {
    /// Attack against defense.
    Simple,
    /// +1 to win, and the target loses no health.
    Chirurgical,
    /// -1 to win, and the target loses one more health.
    Brutal,
    /// -1 to win, and the target falls on the square `to` around the attacker.
    Projection,
}

/// How the ball is thrown.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PlannedThrowStyle {
    /// A throw roll, then the catch.
    Simple,
    /// No throw roll and no catch: the ball bounces three times from the receiver's square.
    Chaos,
}

/// One goblin's order.
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[schemars(transform = drop_integer_formats)]
pub struct PlannedActivation {
    /// Goblin id such as H3.
    pub goblin: String,
    /// Destination [x, y]; the server walks the safest path there. Leave out to stay in place.
    pub to: Option<[u8; 2]>,
    /// Exact squares to step into instead of `to`, each orthogonally next to the previous one.
    pub path: Option<Vec<[u8; 2]>>,
    /// Action after moving.
    pub action: Option<PlannedAction>,
    /// Your own estimate (0 to 1) of this order's chance to succeed, used to measure calibration.
    pub estimate: Option<f64>,
}

/// Arguments of play.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[schemars(transform = drop_integer_formats)]
pub struct PlayArgs {
    /// Orders resolved in this order, a goblin as often as its steps and action left allow;
    /// resolution stops at the first that ends your turn.
    pub activations: Vec<PlannedActivation>,
    /// End your turn after the orders; otherwise it lasts until a failed roll, a score or its clock.
    #[serde(default)]
    pub end_turn: bool,
    /// Only preview the orders as one plan, each from where the previous ones leave the board if
    /// their rolls succeed; nothing is rolled or changed.
    #[serde(default)]
    pub dry_run: bool,
    /// The version from your last result; the call is refused if the match moved on since.
    pub expected_version: Option<u64>,
}

/// What happened to one planned activation.
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(transform = drop_integer_formats)]
pub struct ActivationReport {
    /// The goblin.
    pub goblin: String,
    /// Squares it was ordered through.
    pub path: Vec<[u8; 2]>,
    /// Chances before rolling (with full hints).
    pub odds: Vec<String>,
    /// done, dry run, turn over (knockdown, fumble, score), refused or not played.
    pub outcome: String,
    /// Events it caused.
    pub events: Vec<serde_json::Value>,
}

/// Result of play.
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(transform = drop_integer_formats)]
pub struct PlayReport {
    /// One report per planned activation, in order.
    pub activations: Vec<ActivationReport>,
    /// For a dry run, the chance that every order goes through (full hints only).
    pub plan_chance: Option<String>,
    /// The match after the call.
    pub view: CoachView,
}

/// One alternative to weigh: orders previewed one after the other.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[schemars(transform = drop_integer_formats)]
pub struct PlanToCompare {
    /// Orders previewed in sequence, each from where the previous ones leave the board if their
    /// rolls succeed.
    pub orders: Vec<PlannedActivation>,
}

/// Arguments of compare_plans.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[schemars(transform = drop_integer_formats)]
pub struct CompareArgs {
    /// Alternative plans, each previewed from the board as it stands now.
    pub plans: Vec<PlanToCompare>,
}

/// One plan as previewed, without rolling.
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(transform = drop_integer_formats)]
pub struct PlanReport {
    /// One report per order, in order; previewing stops at a refused order or at a score.
    pub orders: Vec<ActivationReport>,
    /// Chance that every order goes through (full hints only), 0 % when an order is refused.
    pub chance: Option<String>,
}

/// Result of compare_plans.
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(transform = drop_integer_formats)]
pub struct ComparisonReport {
    /// One preview per plan, in the order given.
    pub plans: Vec<PlanReport>,
}

/// The tools of one coach, built for every request.
#[derive(Clone)]
pub struct CoachServer {
    client: MatchClient,
    memories: Arc<Mutex<[SeatMemory; 2]>>,
    hints: OddsHints,
    log: CallLog,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl CoachServer {
    /// A coach server answering for the seat found in each request, logging every call to `log`.
    pub fn new(
        client: MatchClient,
        memories: Arc<Mutex<[SeatMemory; 2]>>,
        hints: OddsHints,
        log: CallLog,
    ) -> Self {
        Self {
            client,
            memories,
            hints,
            log,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Wait until it is your turn (or the match is over), at most max_wait_seconds (≤ 50). Returns the match from your seat; if status is still waiting, call again."
    )]
    async fn wait_for_my_turn(
        &self,
        context: RequestContext<RoleServer>,
        Parameters(arguments): Parameters<WaitArgs>,
    ) -> Result<Json<CoachView>, String> {
        let logged = serde_json::to_value(&arguments).unwrap_or_default();
        self.logged(&context, "wait_for_my_turn", logged, async |seat| {
            let seconds = arguments
                .max_wait_seconds
                .unwrap_or(DEFAULT_WAIT_SECONDS)
                .min(LONGEST_WAIT_SECONDS);
            self.client
                .wait_until(Duration::from_secs(seconds), |snapshot| {
                    snapshot.state.active_team().is_none_or(|team| team == seat)
                })
                .await;
            Ok(Json(self.view_for(seat).await?))
        })
        .await
    }

    #[tool(
        description = "The match from your seat right now: status, clock, score, board, goblins, assault and ball options (pick-ups, scoring routes) and events since your last call."
    )]
    async fn get_state(
        &self,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<CoachView>, String> {
        self.logged(&context, "get_state", serde_json::json!({}), async |seat| {
            Ok(Json(self.view_for(seat).await?))
        })
        .await
    }

    #[tool(description = "The full rules of Goblins Bowl 0.9 (read once).")]
    async fn get_rules(&self, context: RequestContext<RoleServer>) -> Result<String, String> {
        self.logged(&context, "get_rules", serde_json::json!({}), async |_| {
            Ok(english_rules())
        })
        .await
    }

    #[tool(
        description = "Play orders in order during your turn: each moves one goblin with its steps left (to a destination via the safest path, or along an exact path) then optionally assaults, throws or braces in stance if that action is left. A goblin may take several orders a turn. Stops at the first order that ends your turn. end_turn=true hands over after them; dry_run=true only previews them as one plan, each from where the previous ones leave the board, with the chance that the whole plan goes through."
    )]
    async fn play(
        &self,
        context: RequestContext<RoleServer>,
        Parameters(arguments): Parameters<PlayArgs>,
    ) -> Result<Json<PlayReport>, String> {
        let logged = serde_json::to_value(&arguments).unwrap_or_default();
        self.logged(&context, "play", logged, async |seat| {
            let snapshot = self.client.latest();
            if !arguments.dry_run && snapshot.state.active_team() != Some(seat) {
                return Err("it is not your turn: call wait_for_my_turn".to_string());
            }
            if let Some(expected) = arguments.expected_version
                && expected != snapshot.version
            {
                return Err(format!(
                    "stale plan: the match is at version {}, you planned on {expected}; call get_state",
                    snapshot.version
                ));
            }
            if !arguments.dry_run && self.memory_of(seat).turn_seen != Some(turn_mark(&snapshot)) {
                return Err(
                    "the turn changed since you last looked: call get_state (or wait_for_my_turn) and plan again"
                        .to_string(),
                );
            }
            if arguments.dry_run {
                let plan = self.preview_plan(&snapshot.state, seat, &arguments.activations);
                return Ok(Json(PlayReport {
                    activations: plan.orders,
                    plan_chance: plan.chance,
                    view: self.view_for(seat).await?,
                }));
            }
            let mut reports = Vec::new();
            for (index, planned) in arguments.activations.iter().enumerate() {
                let snapshot = self.client.latest();
                let activation = match self.to_activation(planned, &snapshot.state) {
                    Ok(activation) => activation,
                    Err(reason) if index == 0 => return Err(reason),
                    Err(reason) => {
                        reports.push(refused(planned, reason));
                        break;
                    }
                };
                let odds = self.describe_odds(&snapshot, seat, &activation);
                match self
                    .client
                    .submit(seat, Command::Act(activation.clone()))
                    .await
                {
                    Ok(events) => {
                        let outcome = outcome_of(&events, seat);
                        let turn_over = outcome != "done";
                        reports.push(report(&activation, odds, outcome, events));
                        if turn_over {
                            break;
                        }
                    }
                    Err(refusal) if index == 0 => return Err(refusal.to_string()),
                    Err(refusal) => {
                        reports.push(refused(planned, refusal.to_string()));
                        break;
                    }
                }
            }
            let still_my_turn = self.client.latest().state.active_team() == Some(seat);
            if arguments.end_turn && still_my_turn {
                self.client
                    .submit(seat, Command::EndTurn)
                    .await
                    .map_err(|refusal| refusal.to_string())?;
            }
            Ok(Json(PlayReport {
                activations: reports,
                plan_chance: None,
                view: self.view_for(seat).await?,
            }))
        })
        .await
    }

    #[tool(
        description = "Compare plans without rolling any die: each plan is a sequence of orders previewed from the board as it stands now, each order from where the previous ones leave it if their rolls succeed. Returns each order's odds and the chance that the whole plan goes through, so that a dodge then a throw can be weighed against a riskier throw from where the goblin stands."
    )]
    async fn compare_plans(
        &self,
        context: RequestContext<RoleServer>,
        Parameters(arguments): Parameters<CompareArgs>,
    ) -> Result<Json<ComparisonReport>, String> {
        let logged = serde_json::to_value(&arguments).unwrap_or_default();
        self.logged(&context, "compare_plans", logged, async |seat| {
            let snapshot = self.client.latest();
            Ok(Json(ComparisonReport {
                plans: arguments
                    .plans
                    .iter()
                    .map(|plan| self.preview_plan(&snapshot.state, seat, &plan.orders))
                    .collect(),
            }))
        })
        .await
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for CoachServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                "goblinsbowl",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(INSTRUCTIONS)
    }
}

impl CoachServer {
    async fn logged<T>(
        &self,
        context: &RequestContext<RoleServer>,
        tool: &str,
        arguments: serde_json::Value,
        body: impl AsyncFnOnce(TeamId) -> Result<T, String>,
    ) -> Result<T, String> {
        let started_ms = self.log.elapsed_ms();
        let version_before = self.client.latest().version;
        let seat = seat_of(context);
        let seat_label = seat
            .as_ref()
            .map_or_else(|_| "unknown".to_string(), |seat| seat_name(*seat));
        let result = match seat {
            Ok(seat) => body(seat).await,
            Err(refusal) => Err(refusal),
        };
        self.log.record(&CallEntry {
            at_ms: started_ms,
            seat: seat_label,
            tool: tool.to_string(),
            arguments,
            latency_ms: self.log.elapsed_ms().saturating_sub(started_ms),
            error: result.as_ref().err().cloned(),
            version_before: Some(version_before),
            version: self.client.latest().version,
        });
        result
    }

    fn memory_of(&self, seat: TeamId) -> SeatMemory {
        self.memories
            .lock()
            .map(|memories| memories[seat.index()])
            .unwrap_or_default()
    }

    async fn view_for(&self, seat: TeamId) -> Result<CoachView, String> {
        let cursor = self.memory_of(seat).event_cursor;
        let (events, next_cursor) = self
            .client
            .events_since(cursor)
            .await
            .map_err(|error| error.to_string())?;
        let turn_seen = turn_mark(&self.client.latest());
        if let Ok(mut memories) = self.memories.lock() {
            memories[seat.index()] = SeatMemory {
                event_cursor: next_cursor,
                turn_seen: Some(turn_seen),
            };
        }
        if self.client.latest().state.active_team() == Some(seat) {
            self.client
                .ready(seat)
                .await
                .map_err(|error| error.to_string())?;
        }
        Ok(coach_view(
            self.client.arena(),
            self.client.ruleset(),
            &self.client.latest(),
            seat,
            &events,
            self.hints,
        ))
    }

    fn preview_plan(
        &self,
        state: &GameState,
        seat: TeamId,
        orders: &[PlannedActivation],
    ) -> PlanReport {
        let arena = self.client.arena();
        let ruleset = self.client.ruleset();
        let mut board = state.clone();
        let mut chance = Probability::CERTAIN;
        let mut reports = Vec::new();
        for planned in orders {
            let previewed = self.to_activation(planned, &board).and_then(|activation| {
                activation_odds(arena, ruleset, &board, seat, &activation)
                    .map(|odds| (activation, odds))
                    .map_err(|violation| violation.to_string())
            });
            let (activation, odds) = match previewed {
                Ok(previewed) => previewed,
                Err(reason) => {
                    reports.push(refused(planned, reason));
                    chance = Probability::IMPOSSIBLE;
                    break;
                }
            };
            chance = chance.and(odds.overall_success);
            reports.push(report(
                &activation,
                odds_lines(&odds, self.hints),
                "dry run".to_string(),
                Vec::new(),
            ));
            if odds.scores_on_the_way {
                break;
            }
            match board_after_success(arena, ruleset, &board, seat, &activation) {
                Ok(next) => board = next,
                Err(_) => break,
            }
        }
        PlanReport {
            orders: reports,
            chance: describe_chance(chance, self.hints),
        }
    }

    fn to_activation(
        &self,
        planned: &PlannedActivation,
        state: &GameState,
    ) -> Result<Activation, String> {
        let goblin: GoblinId = planned.goblin.parse().map_err(|error| format!("{error}"))?;
        let path = match (&planned.path, planned.to) {
            (Some(path), _) => path.iter().copied().map(Square::from).collect(),
            (None, Some(to)) => {
                let destination = Square::from(to);
                let current = state.goblin(goblin).and_then(|found| found.square());
                if current == Some(destination) {
                    Vec::new()
                } else {
                    safest_path(
                        self.client.arena(),
                        self.client.ruleset(),
                        state,
                        goblin,
                        destination,
                    )
                    .map(|planned_path| planned_path.squares)
                    .ok_or_else(|| {
                        format!(
                            "{goblin} cannot reach [{}, {}] with its steps left this turn (too far, blocked or out of the bowl)",
                            to[0], to[1]
                        )
                    })?
                }
            }
            (None, None) => Vec::new(),
        };
        let action = planned.action.as_ref().map(parse_action).transpose()?;
        Ok(Activation {
            goblin,
            path,
            action,
        })
    }

    fn describe_odds(
        &self,
        snapshot: &MatchSnapshot,
        seat: TeamId,
        activation: &Activation,
    ) -> Vec<String> {
        activation_odds(
            self.client.arena(),
            self.client.ruleset(),
            &snapshot.state,
            seat,
            activation,
        )
        .map(|odds| odds_lines(&odds, self.hints))
        .unwrap_or_default()
    }
}

fn parse_action(planned: &PlannedAction) -> Result<Action, String> {
    let goblin = |text: &str| text.parse::<GoblinId>().map_err(|error| format!("{error}"));
    Ok(match planned {
        PlannedAction::Assault { target, style, to } => Action::Assault {
            target: goblin(target)?,
            style: match style.unwrap_or(PlannedAssaultStyle::Simple) {
                PlannedAssaultStyle::Simple => AssaultStyle::Simple,
                PlannedAssaultStyle::Chirurgical => AssaultStyle::Chirurgical,
                PlannedAssaultStyle::Brutal => AssaultStyle::Brutal,
                PlannedAssaultStyle::Projection => AssaultStyle::Projection(
                    to.and_then(Compass::from_d8_face)
                        .ok_or("a projection needs `to`, a direction from 1 to 8")?,
                ),
            },
        },
        PlannedAction::Throw { target, style } => Action::Throw {
            receiver: goblin(target)?,
            style: match style.unwrap_or(PlannedThrowStyle::Simple) {
                PlannedThrowStyle::Simple => ThrowStyle::Simple,
                PlannedThrowStyle::Chaos => ThrowStyle::Chaos,
            },
        },
        PlannedAction::Stance => Action::Stance,
        PlannedAction::Focus => Action::Focus,
    })
}

fn seat_name(seat: TeamId) -> String {
    if seat == TeamId::HOME { "home" } else { "away" }.to_string()
}

fn seat_of(context: &RequestContext<RoleServer>) -> Result<TeamId, String> {
    context
        .extensions
        .get::<Parts>()
        .and_then(|parts| parts.extensions.get::<Seat>())
        .map(|seat| seat.0)
        .ok_or_else(|| "no seat: connect with your seat token".to_string())
}

fn outcome_of(events: &[GameEvent], seat: TeamId) -> String {
    let scored = events.iter().find_map(|event| match event {
        GameEvent::Scored { team, .. } => Some(*team),
        _ => None,
    });
    if let Some(team) = scored {
        return if team == seat {
            "turn over: you scored".to_string()
        } else {
            "turn over: the opponent scored".to_string()
        };
    }
    events
        .iter()
        .find_map(|event| match event {
            GameEvent::TurnEnded { team, reason } if *team == seat => Some(match reason {
                TurnEndReason::Knockdown => "turn over: knockdown",
                TurnEndReason::Fumble => "turn over: fumble",
                TurnEndReason::NoGoblinCanAct => "turn over: every goblin has acted",
                TurnEndReason::TimeUp => "turn over: time is up",
                TurnEndReason::EndTurn | TurnEndReason::Score => "turn over",
            }),
            _ => None,
        })
        .unwrap_or("done")
        .to_string()
}

fn report(
    activation: &Activation,
    odds: Vec<String>,
    outcome: String,
    events: Vec<GameEvent>,
) -> ActivationReport {
    ActivationReport {
        goblin: activation.goblin.to_string(),
        path: activation.path.iter().copied().map(Into::into).collect(),
        odds,
        outcome,
        events: events
            .iter()
            .filter_map(|event| serde_json::to_value(event).ok())
            .collect(),
    }
}

fn refused(planned: &PlannedActivation, reason: String) -> ActivationReport {
    ActivationReport {
        goblin: planned.goblin.clone(),
        path: Vec::new(),
        odds: Vec::new(),
        outcome: format!("refused: {reason}"),
        events: Vec::new(),
    }
}

async fn authorize(
    State(seats): State<Arc<HashMap<String, TeamId>>>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let seat = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .and_then(|token| seats.get(token.trim()))
        .copied()
        .ok_or(StatusCode::UNAUTHORIZED)?;
    request.extensions_mut().insert(Seat(seat));
    Ok(next.run(request).await)
}

/// Serves the coach tools on `listener` at `/mcp` until the process stops, logging every call to `log`.
///
/// # Errors
/// The IO error that stopped the HTTP server.
pub async fn serve_mcp(
    listener: tokio::net::TcpListener,
    client: MatchClient,
    tokens: SeatTokens,
    hints: OddsHints,
    log: CallLog,
) -> std::io::Result<()> {
    let memories = Arc::new(Mutex::new([SeatMemory::default(); 2]));
    let service: StreamableHttpService<CoachServer, LocalSessionManager> =
        StreamableHttpService::new(
            move || {
                Ok(CoachServer::new(
                    client.clone(),
                    memories.clone(),
                    hints,
                    log.clone(),
                ))
            },
            Default::default(),
            StreamableHttpServerConfig::default()
                .with_allowed_origins(["http://localhost:*", "http://127.0.0.1:*"]),
        );
    let seats: Arc<HashMap<String, TeamId>> = Arc::new(HashMap::from([
        (tokens.home, TeamId::HOME),
        (tokens.away, TeamId::AWAY),
    ]));
    let app = axum::Router::new()
        .nest_service("/mcp", service)
        .layer(middleware::from_fn_with_state(seats, authorize));
    axum::serve(listener, app).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn has_integer_width_format<T: JsonSchema>() -> bool {
        let schema = serde_json::to_string(&schemars::schema_for!(T)).unwrap_or_default();
        schema.contains("\"format\":\"uint") || schema.contains("\"format\":\"int")
    }

    #[test]
    fn given_the_rules_document_when_sent_to_agents_then_only_its_english_half_from_the_0_2_heading()
     {
        let rules = english_rules();
        assert!(rules.starts_with("# Goblins Bowl, rules 0.9") && !rules.contains("règles"));
    }

    #[test]
    fn given_an_order_to_project_east_when_read_then_a_projection_to_the_east() {
        let planned: PlannedAction = serde_json::from_value(serde_json::json!(
            {"type": "assault", "target": "A3", "style": "projection", "to": 3}
        ))
        .unwrap();
        assert_eq!(
            parse_action(&planned),
            Ok(Action::Assault {
                target: "A3".parse().unwrap(),
                style: AssaultStyle::Projection(Compass::East)
            })
        );
    }

    #[test]
    fn given_an_order_to_project_without_a_direction_when_read_then_refused() {
        let planned: PlannedAction = serde_json::from_value(serde_json::json!(
            {"type": "assault", "target": "A3", "style": "projection"}
        ))
        .unwrap();
        assert!(parse_action(&planned).is_err());
    }

    #[test]
    fn given_an_order_to_throw_into_chaos_when_read_then_a_chaos_throw() {
        let planned: PlannedAction = serde_json::from_value(serde_json::json!(
            {"type": "throw", "target": "H5", "style": "chaos"}
        ))
        .unwrap();
        assert_eq!(
            parse_action(&planned),
            Ok(Action::Throw {
                receiver: "H5".parse().unwrap(),
                style: ThrowStyle::Chaos
            })
        );
    }

    #[test]
    fn given_an_order_to_brace_when_read_then_a_stance() {
        let planned: PlannedAction =
            serde_json::from_value(serde_json::json!({"type": "stance"})).unwrap();
        assert_eq!(parse_action(&planned), Ok(Action::Stance));
    }

    #[test]
    fn given_two_fresh_tokens_when_compared_then_they_differ() {
        assert_ne!(random_token(), random_token());
    }

    #[test]
    fn given_the_play_arguments_schema_when_generated_then_no_integer_width_format_remains() {
        assert!(!has_integer_width_format::<PlayArgs>());
    }

    #[test]
    fn given_the_play_report_schema_when_generated_then_no_integer_width_format_remains() {
        assert!(!has_integer_width_format::<PlayReport>());
    }

    #[test]
    fn given_the_wait_arguments_schema_when_generated_then_no_integer_width_format_remains() {
        assert!(!has_integer_width_format::<WaitArgs>());
    }

    #[test]
    fn given_the_comparison_arguments_schema_when_generated_then_no_integer_width_format_remains() {
        assert!(!has_integer_width_format::<CompareArgs>());
    }

    #[test]
    fn given_the_comparison_report_schema_when_generated_then_no_integer_width_format_remains() {
        assert!(!has_integer_width_format::<ComparisonReport>());
    }
}
