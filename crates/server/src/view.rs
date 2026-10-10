//! What a coaching agent sees: a compact, self-explaining picture of the match from its seat.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use goblins_rules::arena::Arena;
use goblins_rules::board::render_board;
use goblins_rules::command::{Action, Activation};
use goblins_rules::event::GameEvent;
use goblins_rules::host::{MatchSnapshot, TurnClock};
use goblins_rules::odds::{ContestDice, Probability};
use goblins_rules::preview::{
    ActionOdds, ActivationOdds, RiskyStep, activation_odds, assault_dice,
};
use goblins_rules::ruleset::Ruleset;
use goblins_rules::state::{Ball, Goblin, Phase, Presence};
use goblins_rules::team::TeamId;

/// How much odds information agents get.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum OddsHints {
    /// Exact chances, as fractions and percentages.
    #[default]
    Full,
    /// Only which dice face off, the agent computes the chances.
    Dice,
    /// Nothing: the agent reads the rules and estimates.
    None,
}

/// Whether the agent should act.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(transform = drop_integer_formats)]
#[serde(rename_all = "snake_case")]
pub enum CoachStatus {
    /// Your team is playing: act now, the clock runs.
    YourTurn,
    /// The other team is playing: call wait_for_my_turn again.
    Waiting,
    /// The final whistle went.
    GameOver,
}

/// A value for each team.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(transform = drop_integer_formats)]
pub struct PerTeam {
    /// Home team (H, defends the west den).
    pub home: u8,
    /// Away team (A, defends the east den).
    pub away: u8,
}

/// One goblin as the agent sees it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(transform = drop_integer_formats)]
pub struct GoblinView {
    /// Id such as H3 or A7.
    pub id: String,
    /// bruiser, sprinter or thrower.
    pub archetype: String,
    /// Square [x, y], absent when knocked out.
    pub square: Option<[u8; 2]>,
    /// standing, down or knocked_out.
    pub condition: String,
    /// Health left.
    pub health: u8,
    /// Full health.
    pub max_health: u8,
    /// Braced in Stance (+1 defense to itself and adjacent teammates).
    pub in_stance: bool,
    /// May still be activated this turn (only meaningful for the team playing).
    pub can_act: bool,
    /// Faces of the attack die.
    pub attack: u8,
    /// Faces of the defense die.
    pub defense: u8,
    /// Faces of the agility die.
    pub agility: u8,
    /// Squares per activation.
    pub movement: u8,
}

/// An assault one of your goblins could make without moving.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(transform = drop_integer_formats)]
pub struct AssaultOption {
    /// Your goblin.
    pub attacker: String,
    /// The adjacent enemy.
    pub target: String,
    /// Dice facing off, for example "d4 vs d4+1" (absent without hints).
    pub dice: Option<String>,
    /// Chance to win, in percent (full hints only).
    pub success_percent: Option<f64>,
}

/// The match as seen from one seat.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(transform = drop_integer_formats)]
pub struct CoachView {
    /// your_turn, waiting or game_over.
    pub status: CoachStatus,
    /// Your team and where you score.
    pub you: String,
    /// Increases with every applied command; send it back as expected_version.
    pub version: u64,
    /// Milliseconds left in the current turn when its clock runs.
    pub time_left_ms: Option<u64>,
    /// Current half (1 or 2).
    pub half: u8,
    /// Turns each team started in this half (10 per half).
    pub turns_played: PerTeam,
    /// Points scored.
    pub score: PerTeam,
    /// Where the ball is.
    pub ball: String,
    /// The bowl as text: H1 standing, h1 down, H1* carrier, H1# in Stance, o loose ball, : den.
    pub board: Vec<String>,
    /// Every goblin.
    pub goblins: Vec<GoblinView>,
    /// Assaults your goblins able to act could make right now, without moving.
    pub assault_options: Vec<AssaultOption>,
    /// What happened since your previous call.
    pub events: Vec<serde_json::Value>,
}

/// Removes the integer width formats (`uint8`, `uint64`, ...) that schemars writes and strict JSON Schema
/// validators reject, keeping plain `integer` types.
pub fn drop_integer_formats(schema: &mut schemars::Schema) {
    fn walk(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(map) => {
                let names_an_integer_width = map
                    .get("format")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|format| format.starts_with("int") || format.starts_with("uint"));
                if names_an_integer_width {
                    map.remove("format");
                }
                map.values_mut().for_each(walk);
            }
            serde_json::Value::Array(items) => items.iter_mut().for_each(walk),
            _ => {}
        }
    }
    if let Some(map) = schema.as_object_mut() {
        map.values_mut().for_each(walk);
        map.remove("format");
    }
}

/// Builds the view of `snapshot` for the coach of `seat`, with the events it has not seen yet.
pub fn coach_view(
    arena: &Arena,
    ruleset: &Ruleset,
    snapshot: &MatchSnapshot,
    seat: TeamId,
    unseen_events: &[GameEvent],
    hints: OddsHints,
) -> CoachView {
    let state = &snapshot.state;
    let status = match state.phase {
        Phase::Finished => CoachStatus::GameOver,
        Phase::Playing { active_team } if active_team == seat => CoachStatus::YourTurn,
        Phase::Playing { .. } => CoachStatus::Waiting,
    };
    let time_left_ms = match snapshot.clock {
        TurnClock::Running { remaining_ms }
        | TurnClock::WaitingForReady { remaining_ms }
        | TurnClock::Paused { remaining_ms } => Some(remaining_ms),
        TurnClock::Stopped => None,
    };
    let assault_options = if status == CoachStatus::YourTurn {
        assault_options(arena, ruleset, snapshot, seat, hints)
    } else {
        Vec::new()
    };
    CoachView {
        status,
        you: describe_seat(arena, seat),
        version: snapshot.version,
        time_left_ms,
        half: state.half,
        turns_played: per_team(&state.turns_played),
        score: per_team(&state.score),
        ball: describe_ball(snapshot),
        board: render_board(arena, state),
        goblins: state.goblins.iter().map(goblin_view).collect(),
        assault_options,
        events: unseen_events
            .iter()
            .filter_map(|event| serde_json::to_value(event).ok())
            .collect(),
    }
}

/// Human-readable chance, such as "62.5 % (5/8)", or None when hints hide it.
pub fn describe_chance(chance: Probability, hints: OddsHints) -> Option<String> {
    (hints == OddsHints::Full).then(|| {
        format!(
            "{:.1} % ({}/{})",
            chance.as_percentage(),
            chance.numerator(),
            chance.denominator()
        )
    })
}

/// Which dice face off, for example "d4 vs d4+1".
pub fn describe_dice(dice: &ContestDice) -> String {
    let modifier = if dice.opponent_modifier == 0 {
        String::new()
    } else {
        format!("{:+}", dice.opponent_modifier)
    };
    format!(
        "d{} vs d{}{modifier}",
        dice.actor_faces, dice.opponent_faces
    )
}

/// What an agent is told about a planned activation: every roll with its dice, and its chance
/// when odds hints are full; nothing when hints are off.
pub fn odds_lines(odds: &ActivationOdds, hints: OddsHints) -> Vec<String> {
    if hints == OddsHints::None {
        return Vec::new();
    }
    let annotate = |text: String, dice: Option<String>, chance: Probability| match (
        dice,
        describe_chance(chance, hints),
    ) {
        (Some(dice), Some(chance)) => format!("{text}: {dice}, {chance}"),
        (Some(dice), None) => format!("{text}: {dice}"),
        (None, Some(chance)) => format!("{text}: {chance}"),
        (None, None) => text,
    };
    let mut lines: Vec<String> = odds
        .risky_steps
        .iter()
        .map(|step| match step {
            RiskyStep::Dodge {
                from,
                tackler,
                dice,
                success,
            } => annotate(
                format!("dodge out of [{}, {}] past {tackler}", from.x, from.y),
                Some(describe_dice(dice)),
                *success,
            ),
            RiskyStep::Pick { at, success } => {
                annotate(format!("pick at [{}, {}]", at.x, at.y), None, *success)
            }
        })
        .collect();
    match &odds.action {
        Some(ActionOdds::Assault {
            target,
            dice,
            success,
        }) => lines.push(annotate(
            format!("assault {target}"),
            dice.as_ref().map(describe_dice),
            *success,
        )),
        Some(ActionOdds::Throw {
            receiver,
            throw,
            interceptions,
            catch,
            success,
        }) => {
            lines.push(annotate(format!("throw to {receiver}"), None, *throw));
            lines.extend(interceptions.iter().map(|(interceptor, chance)| {
                annotate(format!("interception by {interceptor}"), None, *chance)
            }));
            lines.push(annotate(format!("catch by {receiver}"), None, *catch));
            lines.push(annotate(format!("ball reaches {receiver}"), None, *success));
        }
        Some(ActionOdds::Stance) | None => {}
    }
    if odds.scores_on_the_way {
        lines.push("scores on the way".to_string());
    }
    if let Some(chance) = describe_chance(odds.overall_success, hints) {
        lines.push(format!("no knockdown nor fumble: {chance}"));
    }
    lines
}

fn describe_seat(arena: &Arena, seat: TeamId) -> String {
    let column = |team: TeamId| {
        arena
            .den_squares(team)
            .next()
            .map_or_else(|| "?".to_string(), |square| square.x.to_string())
    };
    let (name, own_side, enemy_side) = if seat == TeamId::HOME {
        ("home (H)", "west", "east")
    } else {
        ("away (A)", "east", "west")
    };
    format!(
        "{name}: you defend the {own_side} den (column {}) and score in the {enemy_side} den (column {})",
        column(seat),
        column(seat.duel_opponent())
    )
}

fn per_team(values: &[u8]) -> PerTeam {
    PerTeam {
        home: values
            .get(TeamId::HOME.index())
            .copied()
            .unwrap_or_default(),
        away: values
            .get(TeamId::AWAY.index())
            .copied()
            .unwrap_or_default(),
    }
}

fn describe_ball(snapshot: &MatchSnapshot) -> String {
    match snapshot.state.ball {
        Ball::Carried { by } => match snapshot.state.ball_square() {
            Some(square) => format!("carried by {by} at [{}, {}]", square.x, square.y),
            None => format!("carried by {by}"),
        },
        Ball::Loose { at } => format!("loose at [{}, {}]", at.x, at.y),
    }
}

fn goblin_view(goblin: &Goblin) -> GoblinView {
    let condition = match goblin.presence {
        Presence::Standing(_) => "standing",
        Presence::Down(_) => "down",
        Presence::KnockedOut => "knocked_out",
    };
    GoblinView {
        id: goblin.id.to_string(),
        archetype: serde_json::to_value(goblin.archetype)
            .ok()
            .and_then(|name| name.as_str().map(str::to_string))
            .unwrap_or_default(),
        square: goblin.square().map(Into::into),
        condition: condition.to_string(),
        health: goblin.health,
        max_health: goblin.characteristics.health,
        in_stance: goblin.in_stance,
        can_act: goblin.can_act(),
        attack: goblin.characteristics.attack,
        defense: goblin.characteristics.defense,
        agility: goblin.characteristics.agility,
        movement: goblin.characteristics.movement,
    }
}

fn assault_options(
    arena: &Arena,
    ruleset: &Ruleset,
    snapshot: &MatchSnapshot,
    seat: TeamId,
    hints: OddsHints,
) -> Vec<AssaultOption> {
    let state = &snapshot.state;
    state
        .goblins_of(seat)
        .filter(|goblin| goblin.can_act())
        .flat_map(|attacker| {
            let square = attacker.square();
            state
                .goblins
                .iter()
                .filter(move |defender| {
                    defender.team() != seat
                        && square
                            .zip(defender.square())
                            .is_some_and(|(from, to)| from.is_orthogonally_adjacent(to))
                })
                .map(move |defender| (attacker, defender))
        })
        .map(|(attacker, defender)| {
            let order = Activation {
                goblin: attacker.id,
                path: Vec::new(),
                action: Some(Action::Assault {
                    target: defender.id,
                }),
            };
            let chance = activation_odds(arena, ruleset, state, seat, &order)
                .map(|odds| odds.overall_success)
                .unwrap_or(Probability::IMPOSSIBLE);
            let dice = assault_dice(ruleset, state, attacker.id, defender.id);
            AssaultOption {
                attacker: attacker.id.to_string(),
                target: defender.id.to_string(),
                dice: dice
                    .filter(|_| hints != OddsHints::None)
                    .map(|dice| describe_dice(&dice)),
                success_percent: (hints == OddsHints::Full)
                    .then(|| (chance.as_percentage() * 10.0).round() / 10.0),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    fn walk_with_a_dodge_then_an_assault() -> ActivationOdds {
        let dice = ContestDice {
            actor_faces: 4,
            opponent_faces: 4,
            opponent_modifier: 1,
        };
        ActivationOdds {
            risky_steps: vec![RiskyStep::Dodge {
                from: goblins_rules::square::Square::new(5, 7),
                tackler: "A1".parse().unwrap(),
                dice,
                success: Probability::new(3, 8),
            }],
            path_success: Probability::new(3, 8),
            scores_on_the_way: false,
            action: Some(ActionOdds::Assault {
                target: "A2".parse().unwrap(),
                dice: Some(ContestDice {
                    opponent_modifier: 0,
                    ..dice
                }),
                success: Probability::new(5, 8),
            }),
            overall_success: Probability::new(15, 64),
        }
    }

    #[test]
    fn given_dice_hints_when_describing_odds_then_the_dice_without_chances() {
        assert_eq!(
            odds_lines(&walk_with_a_dodge_then_an_assault(), OddsHints::Dice),
            [
                "dodge out of [5, 7] past A1: d4 vs d4+1",
                "assault A2: d4 vs d4"
            ]
        );
    }

    #[test]
    fn given_full_hints_when_describing_odds_then_dice_and_chances() {
        assert_eq!(
            odds_lines(&walk_with_a_dodge_then_an_assault(), OddsHints::Full).first(),
            Some(&"dodge out of [5, 7] past A1: d4 vs d4+1, 37.5 % (3/8)".to_string())
        );
    }

    #[test]
    fn given_no_hints_when_describing_odds_then_nothing() {
        assert!(odds_lines(&walk_with_a_dodge_then_an_assault(), OddsHints::None).is_empty());
    }

    #[test]
    fn given_a_downed_defender_when_describing_dice_then_the_malus_is_signed() {
        let dice = ContestDice {
            actor_faces: 3,
            opponent_faces: 2,
            opponent_modifier: -2,
        };
        assert_eq!(describe_dice(&dice), "d3 vs d2-2");
    }

    use super::*;
    use goblins_rules::host::{MatchHost, MatchSetup, SeatKind};
    use goblins_rules::roster::Roster;

    fn kick_off() -> MatchHost {
        MatchHost::start(MatchSetup {
            ruleset: Ruleset::default(),
            rosters: [Roster::DEFAULT, Roster::DEFAULT],
            seed: 42,
            seats: [SeatKind::Agent, SeatKind::Agent],
            turn_seconds: [60, 60],
            agent_ready_grace_seconds: 15,
        })
        .unwrap()
        .0
    }

    fn view_for(host: &MatchHost, seat: TeamId, hints: OddsHints) -> CoachView {
        coach_view(
            host.game().arena(),
            host.game().ruleset(),
            &host.snapshot(),
            seat,
            host.events_since(0),
            hints,
        )
    }

    #[test]
    fn given_the_seat_playing_when_viewed_then_your_turn() {
        let host = kick_off();
        let seat = host.snapshot().state.active_team().unwrap();
        assert_eq!(
            view_for(&host, seat, OddsHints::Full).status,
            CoachStatus::YourTurn
        );
    }

    #[test]
    fn given_the_idle_seat_when_viewed_then_waiting() {
        let host = kick_off();
        let idle = host.snapshot().state.active_team().unwrap().duel_opponent();
        assert_eq!(
            view_for(&host, idle, OddsHints::Full).status,
            CoachStatus::Waiting
        );
    }

    #[test]
    fn given_a_running_clock_when_viewed_then_the_time_left_shows() {
        let host = kick_off();
        assert_eq!(
            view_for(&host, TeamId::HOME, OddsHints::Full).time_left_ms,
            Some(60_000)
        );
    }

    #[test]
    fn given_the_home_seat_when_viewed_then_it_is_told_where_it_scores() {
        let host = kick_off();
        assert!(
            view_for(&host, TeamId::HOME, OddsHints::Full)
                .you
                .contains("score in the east den (column 14)")
        );
    }

    #[test]
    fn given_the_kick_off_when_viewed_then_eighteen_goblins_and_sixteen_board_lines() {
        let host = kick_off();
        let view = view_for(&host, TeamId::HOME, OddsHints::Full);
        assert_eq!((view.goblins.len(), view.board.len()), (18, 16));
    }

    #[test]
    fn given_unseen_events_when_viewed_then_they_are_passed_on_as_json() {
        let host = kick_off();
        let view = view_for(&host, TeamId::HOME, OddsHints::Full);
        assert_eq!(
            view.events
                .first()
                .and_then(|event| event.get("event"))
                .and_then(|name| name.as_str()),
            Some("coin_tossed")
        );
    }

    #[test]
    fn given_full_hints_when_describing_five_eighths_then_percent_and_fraction() {
        assert_eq!(
            describe_chance(Probability::new(5, 8), OddsHints::Full),
            Some("62.5 % (5/8)".to_string())
        );
    }

    #[test]
    fn given_hidden_hints_when_describing_a_chance_then_nothing() {
        assert_eq!(
            describe_chance(Probability::new(5, 8), OddsHints::Dice),
            None
        );
    }

    #[test]
    fn given_the_coach_view_schema_when_generated_then_no_integer_width_format_remains() {
        let schema = serde_json::to_string(&schemars::schema_for!(CoachView)).unwrap();
        assert!(!schema.contains("\"format\":\"uint") && !schema.contains("\"format\":\"int"));
    }
}
