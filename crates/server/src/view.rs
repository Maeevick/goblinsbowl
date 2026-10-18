//! What a coaching agent sees: a compact, self-explaining picture of the match from its seat.

use std::cmp::Reverse;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use goblins_rules::arena::Arena;
use goblins_rules::board::render_board;
use goblins_rules::command::{Action, Activation};
use goblins_rules::event::GameEvent;
use goblins_rules::host::{MatchSnapshot, TurnClock};
use goblins_rules::odds::{AgilityDice, ContestDice, Probability};
use goblins_rules::paths::{PlannedPath, reachable_squares};
use goblins_rules::preview::{
    ActionOdds, ActivationOdds, DodgeDice, RiskyStep, activation_odds, assault_dice,
};
use goblins_rules::roster::GoblinId;
use goblins_rules::ruleset::Ruleset;
use goblins_rules::state::{Ball, GameState, Goblin, Phase, Presence};
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
    /// May still be ordered this turn: it has steps or its action left (only meaningful for the
    /// team playing).
    pub can_act: bool,
    /// Steps it may still take this turn (only meaningful for the team playing).
    pub steps_left: u8,
    /// May still assault or throw this turn (only meaningful for the team playing).
    pub has_action_left: bool,
    /// Faces of the attack die.
    pub attack: u8,
    /// Faces of the defense die.
    pub defense: u8,
    /// Faces of the agility die.
    pub agility: u8,
    /// Squares per turn.
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

/// A ball move one of your goblins able to act could make this turn.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(transform = drop_integer_formats)]
pub struct BallOption {
    /// Your goblin.
    pub goblin: String,
    /// pick (walk onto the loose ball) or score (walk the carrier into the enemy den).
    pub kind: String,
    /// Destination to send as `to` in play; the safest path is taken.
    pub to: [u8; 2],
    /// Chance to get there without a knockdown or a fumble, in percent (full hints only).
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
    /// Turns each team plays per half.
    pub turns_per_half: u8,
    /// Turns each team started in this half.
    pub turns_played: PerTeam,
    /// Points scored.
    pub score: PerTeam,
    /// Where the ball is.
    pub ball: String,
    /// The bowl as text: H1 standing, h1 down, H1* carrier, o loose ball, : den.
    pub board: Vec<String>,
    /// Every goblin.
    pub goblins: Vec<GoblinView>,
    /// Assaults your goblins able to act could make right now, without moving.
    pub assault_options: Vec<AssaultOption>,
    /// Picking up the loose ball or scoring with your carrier, best first with full hints.
    pub ball_options: Vec<BallOption>,
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
    let (assault_options, ball_options) = if status == CoachStatus::YourTurn {
        (
            assault_options(arena, ruleset, snapshot, seat, hints),
            ball_options(arena, ruleset, state, seat, hints),
        )
    } else {
        (Vec::new(), Vec::new())
    };
    CoachView {
        status,
        you: describe_seat(arena, seat),
        version: snapshot.version,
        time_left_ms,
        half: state.half,
        turns_per_half: ruleset.turns_per_half,
        turns_played: per_team(&state.turns_played),
        score: per_team(&state.score),
        ball: describe_ball(snapshot),
        board: render_board(arena, state),
        goblins: state.goblins.iter().map(goblin_view).collect(),
        assault_options,
        ball_options,
        events: unseen_events
            .iter()
            .filter_map(|event| serde_json::to_value(event).ok())
            .collect(),
    }
}

fn ball_options(
    arena: &Arena,
    ruleset: &Ruleset,
    state: &GameState,
    seat: TeamId,
    hints: OddsHints,
) -> Vec<BallOption> {
    let option = |goblin: GoblinId, kind: &str, path: &PlannedPath| BallOption {
        goblin: goblin.to_string(),
        kind: kind.to_string(),
        to: [path.destination.x, path.destination.y],
        success_percent: (hints == OddsHints::Full)
            .then(|| (path.success.as_percentage() * 10.0).round() / 10.0),
    };
    let mut ranked: Vec<(Probability, BallOption)> = match state.ball {
        Ball::Loose { at } => state
            .goblins_of(seat)
            .filter(|goblin| goblin.can_act())
            .filter_map(|goblin| {
                reachable_squares(arena, ruleset, state, goblin.id)
                    .into_iter()
                    .find(|path| path.destination == at)
                    .map(|path| (path.success, option(goblin.id, "pick", &path)))
            })
            .collect(),
        Ball::Carried { by } => state
            .goblin(by)
            .filter(|carrier| carrier.team() == seat && carrier.can_act())
            .and_then(|carrier| {
                reachable_squares(arena, ruleset, state, carrier.id)
                    .into_iter()
                    .filter(|path| arena.is_scoring_square_for(seat, path.destination))
                    .min_by_key(|path| (Reverse(path.success), path.squares.len()))
                    .map(|path| (path.success, option(carrier.id, "score", &path)))
            })
            .into_iter()
            .collect(),
    };
    if hints == OddsHints::Full {
        ranked.sort_by_key(|(chance, _)| Reverse(*chance));
        ranked.truncate(3);
    } else {
        ranked.truncate(5);
    }
    ranked.into_iter().map(|(_, option)| option).collect()
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

/// An agility test's die, for example "d4-3 needs 2".
pub fn describe_agility_dice(dice: &AgilityDice) -> String {
    if dice.malus == 0 {
        format!("d{} needs {}", dice.faces, dice.target)
    } else {
        format!("d{}-{} needs {}", dice.faces, dice.malus, dice.target)
    }
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
                dice,
                success,
            } => {
                let out_of = format!("dodge out of [{}, {}]", from.x, from.y);
                let (text, dice) = match dice {
                    DodgeDice::Contest { tackler, contest } => {
                        (format!("{out_of} past {tackler}"), describe_dice(contest))
                    }
                    DodgeDice::Agility(dice) => (out_of, describe_agility_dice(dice)),
                };
                annotate(text, Some(dice), *success)
            }
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
            ..
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
    let columns = |team: TeamId| {
        let xs: Vec<u8> = arena.den_squares(team).map(|square| square.x).collect();
        match (xs.iter().min(), xs.iter().max()) {
            (Some(first), Some(last)) if first == last => format!("column {first}"),
            (Some(first), Some(last)) => format!("columns {first} to {last}"),
            _ => "no column".to_string(),
        }
    };
    let (name, own_side, enemy_side) = if seat == TeamId::HOME {
        ("home (H)", "west", "east")
    } else {
        ("away (A)", "east", "west")
    };
    format!(
        "{name}: you defend the {own_side} den ({}) and score in the {enemy_side} den ({})",
        columns(seat),
        columns(seat.duel_opponent())
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
        can_act: goblin.can_act(),
        steps_left: goblin.steps_left,
        has_action_left: goblin.has_action_left,
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
        .filter(|goblin| goblin.can_act() && goblin.has_action_left)
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
        .filter_map(|(attacker, defender)| {
            let order = Activation {
                goblin: attacker.id,
                path: Vec::new(),
                action: Some(Action::Assault {
                    target: defender.id,
                }),
            };
            let chance = activation_odds(arena, ruleset, state, seat, &order)
                .ok()?
                .overall_success;
            let dice = assault_dice(ruleset, state, attacker.id, defender.id);
            Some(AssaultOption {
                attacker: attacker.id.to_string(),
                target: defender.id.to_string(),
                dice: dice
                    .filter(|_| hints != OddsHints::None)
                    .map(|dice| describe_dice(&dice)),
                success_percent: (hints == OddsHints::Full)
                    .then(|| (chance.as_percentage() * 10.0).round() / 10.0),
            })
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
                dice: DodgeDice::Contest {
                    tackler: "A1".parse().unwrap(),
                    contest: dice,
                },
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
                knock_down: Probability::new(5, 8),
                stand_off: Probability::IMPOSSIBLE,
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
    fn given_full_hints_when_describing_an_agility_dodge_then_its_die_malus_target_and_chance() {
        let odds = ActivationOdds {
            risky_steps: vec![RiskyStep::Dodge {
                from: goblins_rules::square::Square::new(5, 7),
                dice: DodgeDice::Agility(AgilityDice {
                    faces: 4,
                    malus: 3,
                    target: 2,
                }),
                success: Probability::IMPOSSIBLE,
            }],
            path_success: Probability::IMPOSSIBLE,
            scores_on_the_way: false,
            action: None,
            overall_success: Probability::IMPOSSIBLE,
        };
        assert_eq!(
            odds_lines(&odds, OddsHints::Full).first(),
            Some(&"dodge out of [5, 7]: d4-3 needs 2, 0.0 % (0/1)".to_string())
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
        kick_off_on(Ruleset::default())
    }

    fn kick_off_on(ruleset: Ruleset) -> MatchHost {
        MatchHost::start(MatchSetup {
            ruleset,
            rosters: [Roster::DEFAULT, Roster::DEFAULT],
            seed: 2,
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
    fn given_the_kick_off_with_full_hints_when_viewed_then_the_best_pick_comes_with_its_chance() {
        let host = kick_off();
        let seat = host.snapshot().state.active_team().unwrap();
        let options = view_for(&host, seat, OddsHints::Full).ball_options;
        assert!(
            options
                .first()
                .is_some_and(|option| option.kind == "pick" && option.success_percent.is_some())
        );
    }

    #[test]
    fn given_the_kick_off_with_hidden_hints_when_viewed_then_picks_come_without_chances() {
        let host = kick_off();
        let seat = host.snapshot().state.active_team().unwrap();
        let options = view_for(&host, seat, OddsHints::None).ball_options;
        assert!(
            !options.is_empty()
                && options
                    .iter()
                    .all(|option| option.success_percent.is_none())
        );
    }

    #[test]
    fn given_a_carrier_two_steps_from_the_den_when_viewed_then_a_scoring_option() {
        let host = kick_off();
        let mut snapshot = host.snapshot();
        let seat = snapshot.state.active_team().unwrap();
        let near_the_den = if seat == TeamId::HOME {
            goblins_rules::square::Square::new(18, 10)
        } else {
            goblins_rules::square::Square::new(2, 10)
        };
        let carrier = snapshot.state.goblins_of(seat).nth(4).unwrap().id;
        for goblin in &mut snapshot.state.goblins {
            if goblin.id == carrier {
                goblin.presence = Presence::Standing(near_the_den);
            }
        }
        snapshot.state.ball = Ball::Carried { by: carrier };
        let view = coach_view(
            host.game().arena(),
            host.game().ruleset(),
            &snapshot,
            seat,
            &[],
            OddsHints::Full,
        );
        assert_eq!(
            view.ball_options.first().map(|option| option.kind.as_str()),
            Some("score")
        );
    }

    fn attacker_next_to_an_enemy(host: &MatchHost, has_action_left: bool) -> CoachView {
        let (snapshot, seat, _) = beside_an_enemy(host, has_action_left);
        coach_view(
            host.game().arena(),
            host.game().ruleset(),
            &snapshot,
            seat,
            &[],
            OddsHints::Full,
        )
    }

    fn beside_an_enemy(
        host: &MatchHost,
        has_action_left: bool,
    ) -> (MatchSnapshot, TeamId, GoblinId) {
        let mut snapshot = host.snapshot();
        let seat = snapshot.state.active_team().unwrap();
        let attacker = snapshot.state.goblins_of(seat).next().unwrap().id;
        let defender = snapshot
            .state
            .goblins_of(seat.duel_opponent())
            .next()
            .unwrap();
        let target = defender.square().unwrap();
        let beside = if seat == TeamId::HOME {
            goblins_rules::square::Square::new(target.x - 1, target.y)
        } else {
            goblins_rules::square::Square::new(target.x + 1, target.y)
        };
        for goblin in &mut snapshot.state.goblins {
            if goblin.id == attacker {
                goblin.presence = Presence::Standing(beside);
                goblin.has_action_left = has_action_left;
            }
        }
        (snapshot, seat, attacker)
    }

    #[test]
    fn given_carriers_that_cannot_assault_when_the_carrier_stands_next_to_an_enemy_then_no_assault_option()
     {
        let host = kick_off();
        let (mut snapshot, seat, attacker) = beside_an_enemy(&host, true);
        snapshot.state.ball = Ball::Carried { by: attacker };
        let ruleset = Ruleset {
            carriers_can_assault: false,
            ..host.game().ruleset().clone()
        };
        let view = coach_view(
            host.game().arena(),
            &ruleset,
            &snapshot,
            seat,
            &[],
            OddsHints::Full,
        );
        assert!(view.assault_options.is_empty());
    }

    #[test]
    fn given_an_attacker_next_to_an_enemy_when_viewed_then_an_assault_option() {
        let view = attacker_next_to_an_enemy(&kick_off(), true);
        assert_eq!(view.assault_options.len(), 1);
    }

    #[test]
    fn given_an_attacker_next_to_an_enemy_that_already_used_its_action_when_viewed_then_no_assault_option()
     {
        let view = attacker_next_to_an_enemy(&kick_off(), false);
        assert!(view.assault_options.is_empty());
    }

    #[test]
    fn given_a_goblin_that_used_its_action_when_viewed_then_its_steps_left_show_without_its_action()
    {
        let mut snapshot = kick_off().snapshot();
        let seat = snapshot.state.active_team().unwrap();
        let runner = snapshot.state.goblins_of(seat).nth(4).unwrap().id;
        for goblin in &mut snapshot.state.goblins {
            if goblin.id == runner {
                goblin.steps_left = 2;
                goblin.has_action_left = false;
            }
        }
        let host = kick_off();
        let view = coach_view(
            host.game().arena(),
            host.game().ruleset(),
            &snapshot,
            seat,
            &[],
            OddsHints::Full,
        );
        let seen = view
            .goblins
            .iter()
            .find(|goblin| goblin.id == runner.to_string())
            .unwrap();
        assert_eq!(
            (seen.steps_left, seen.has_action_left, seen.can_act),
            (2, false, true)
        );
    }

    #[test]
    fn given_the_kick_off_when_viewed_then_the_turns_per_half_of_the_rules_show() {
        let host = kick_off();
        assert_eq!(
            view_for(&host, TeamId::HOME, OddsHints::Full).turns_per_half,
            host.game().ruleset().turns_per_half
        );
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

    fn bowl(mask: &[&str]) -> Ruleset {
        Ruleset {
            arena: mask.iter().map(|row| (*row).to_string()).collect(),
            ..Ruleset::default()
        }
    }

    #[test]
    fn given_the_home_seat_on_the_cross_bowl_when_viewed_then_it_scores_in_the_east_column() {
        let host = kick_off_on(bowl(&goblins_rules::arena::BOWL_21_MASK));
        assert!(
            view_for(&host, TeamId::HOME, OddsHints::Full)
                .you
                .contains("score in the east den (column 20)")
        );
    }

    #[test]
    fn given_the_home_seat_on_the_round_bowl_when_viewed_then_it_scores_in_the_east_arc_over_two_columns()
     {
        let host = kick_off_on(bowl(&goblins_rules::arena::ROUND_BOWL_21_MASK));
        assert!(
            view_for(&host, TeamId::HOME, OddsHints::Full)
                .you
                .contains("score in the east den (columns 19 to 20)")
        );
    }

    #[test]
    fn given_the_kick_off_when_viewed_then_eighteen_goblins_and_twenty_two_board_lines() {
        let host = kick_off();
        let view = view_for(&host, TeamId::HOME, OddsHints::Full);
        assert_eq!((view.goblins.len(), view.board.len()), (18, 22));
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
