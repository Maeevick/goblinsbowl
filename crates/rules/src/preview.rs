//! What a planned activation risks, computed exactly before any die is rolled.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::arena::Arena;
use crate::bounces::{
    CHAOS_BOUNCES, MOST_DIRECTION_ROLLS, MOST_MOVES_IN_A_BOUNCE, direction_die, landing,
    settling_square,
};
use crate::command::{Action, Activation, AssaultStyle, ThrowStyle};
use crate::flight::squares_crossed;
use crate::odds::{AgilityDice, ContestDice, Probability, agility_success};
use crate::roster::GoblinId;
use crate::ruleset::Ruleset;
use crate::square::{Reach, Square};
use crate::state::{Ball, Focus, GameState, Presence};
use crate::team::TeamId;
use crate::validation::{projection_landing, validate_activation};
use crate::violation::RuleViolation;

/// The dice of a dodge, as the ruleset decides.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "test", rename_all = "snake_case")]
pub enum DodgeDice {
    /// Agility die against the Defense die and Stance bonus of the enemy most likely to stop the goblin.
    Contest {
        /// That enemy.
        tackler: GoblinId,
        /// The dice facing off.
        contest: ContestDice,
    },
    /// Agility test, each adjacent standing enemy a malus of one, two when it stands in Stance.
    Agility(AgilityDice),
}

impl DodgeDice {
    /// Chance to slip away.
    pub fn success(self) -> Probability {
        match self {
            DodgeDice::Contest { contest, .. } => contest.success(),
            DodgeDice::Agility(dice) => dice.success(),
        }
    }
}

/// A moment of an activation that can end the turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "risk", rename_all = "snake_case")]
pub enum RiskyStep {
    /// Leaving a square next to standing enemies.
    Dodge {
        /// Square left.
        from: Square,
        /// The dice rolled.
        dice: DodgeDice,
        /// Chance to slip away.
        success: Probability,
    },
    /// Stepping onto the loose ball.
    Pick {
        /// The ball's square.
        at: Square,
        /// Chance to pick it up.
        success: Probability,
    },
}

/// Chances of the action that ends the activation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ActionOdds {
    /// Bracing never fails.
    Stance,
    /// Focusing never fails.
    Focus,
    /// Chances of an assault: the attacker keeps its feet, knocking the target down or standing off.
    Assault {
        /// The target.
        target: GoblinId,
        /// The dice facing off, unless the target is gone.
        dice: Option<ContestDice>,
        /// Chance the attacker does not fall, so the turn goes on.
        success: Probability,
        /// Chance the target falls, or loses Health when it already lies down.
        knock_down: Probability,
        /// Chance nobody falls: a tie, under rules where ties stand off.
        stand_off: Probability,
    },
    /// Chances of a chaos throw: no throw roll and no catch, the interceptions on the way, then
    /// three bounces from the receiver's square that a teammate has to end up holding.
    Chaos {
        /// The teammate the ball flies towards.
        receiver: GoblinId,
        /// Each opponent on or next to the flight line, in order, with its chance to intercept.
        interceptions: Vec<(GoblinId, Probability)>,
        /// Chance a teammate holds the ball once its three bounces are over.
        kept: Probability,
        /// Chance nobody intercepts it and a teammate holds it after its bounces, so the turn goes on.
        success: Probability,
    },
    /// Chances along a throw: the throw roll, each interception, the catch.
    Throw {
        /// The receiver.
        receiver: GoblinId,
        /// Chance the throw roll succeeds.
        throw: Probability,
        /// Each opponent on the flight line, in order, with its chance to intercept.
        interceptions: Vec<(GoblinId, Probability)>,
        /// Chance the receiver catches.
        catch: Probability,
        /// Chance the receiver ends up holding the ball.
        success: Probability,
    },
}

/// Exact chances of a planned activation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivationOdds {
    /// Every roll on the way, in order.
    pub risky_steps: Vec<RiskyStep>,
    /// Chance to complete the whole path.
    pub path_success: Probability,
    /// Whether the goblin scores on its way, which ends the turn before its action.
    pub scores_on_the_way: bool,
    /// Chances of the final action, if one is planned and reached.
    pub action: Option<ActionOdds>,
    /// Chance the activation ends without a knockdown or a fumble
    /// (a ball dropped by a knocked-down target and missed by a teammate is not counted).
    pub overall_success: Probability,
}

/// Defense bonus Stance gives goblin `id`: one if it braces itself, one per braced adjacent teammate
/// up to the neighbours' cap, the whole within the Stance bonus cap.
pub fn stance_bonus(ruleset: &Ruleset, state: &GameState, id: GoblinId) -> u8 {
    let support = state.stance_support(id);
    u8::from(support.itself)
        .saturating_add(support.neighbours.min(ruleset.neighbour_stance_bonus_cap))
        .min(ruleset.stance_bonus_cap)
}

/// The dice of `attacker` assaulting `defender`: Attack against Defense plus the defender's Stance
/// bonus, minus the downed malus when the defender lies on the ground and the carrier's malus when
/// it holds the ball.
pub fn assault_dice(
    ruleset: &Ruleset,
    state: &GameState,
    attacker: GoblinId,
    defender: GoblinId,
) -> Option<ContestDice> {
    let actor_faces = state.goblin(attacker)?.characteristics.attack;
    let target = state.goblin(defender)?;
    let downed_malus = if matches!(target.presence, Presence::Down(_)) {
        i16::from(ruleset.downed_defense_malus)
    } else {
        0
    };
    let carrier_malus = if state.carrier() == Some(defender) {
        i16::from(ruleset.carrier_defense_malus)
    } else {
        0
    };
    let modifier = i16::from(stance_bonus(ruleset, state, defender)) - downed_malus - carrier_malus;
    Some(ContestDice {
        actor_modifier: 0,
        actor_faces,
        opponent_faces: target.characteristics.defense,
        opponent_modifier: i8::try_from(modifier).unwrap_or(i8::MIN),
    })
}

/// The dice of `attacker` assaulting `defender` in `style`: the situation's dice, the style's
/// modifier on the attack roll.
pub fn styled_assault_dice(
    ruleset: &Ruleset,
    state: &GameState,
    attacker: GoblinId,
    defender: GoblinId,
    style: AssaultStyle,
) -> Option<ContestDice> {
    assault_dice(ruleset, state, attacker, defender).map(|dice| ContestDice {
        actor_modifier: style.attack_modifier(),
        ..dice
    })
}

/// The dice of `mover` leaving `from`, none when no standing enemy stands next to it.
pub fn dodge_dice(
    ruleset: &Ruleset,
    state: &GameState,
    mover: GoblinId,
    from: Square,
) -> Option<DodgeDice> {
    let goblin = state.goblin(mover)?;
    if ruleset.dodges_test_agility {
        let malus = dodge_malus(ruleset, state, from, goblin.team());
        return (malus > 0).then_some(DodgeDice::Agility(AgilityDice {
            bonus: ruleset.dodge_bonus,
            faces: goblin.characteristics.agility,
            malus,
            target: ruleset.ball_roll_target,
        }));
    }
    let (tackler, _) = most_dangerous_tackler(ruleset, state, mover, from)?;
    tackle_dice(ruleset, state, mover, tackler)
        .map(|contest| DodgeDice::Contest { tackler, contest })
}

/// Malus on an agility dodge out of `square`: one per standing enemy within reach, two when it
/// stands in Stance under rules where braced tacklers weigh double.
fn dodge_malus(ruleset: &Ruleset, state: &GameState, square: Square, team: TeamId) -> u8 {
    state
        .standing_opponents_in_reach(square, team, ruleset.reach())
        .map(|enemy| 1 + u8::from(enemy.in_stance && ruleset.braced_tacklers_weigh_double))
        .fold(0, u8::saturating_add)
}

/// The dice of `mover` slipping away from `tackler`: Agility against Defense plus the tackler's Stance bonus.
fn tackle_dice(
    ruleset: &Ruleset,
    state: &GameState,
    mover: GoblinId,
    tackler: GoblinId,
) -> Option<ContestDice> {
    let actor_faces = state.goblin(mover)?.characteristics.agility;
    let opponent = state.goblin(tackler)?;
    Some(ContestDice {
        actor_modifier: 0,
        actor_faces,
        opponent_faces: opponent.characteristics.defense,
        opponent_modifier: i8::try_from(stance_bonus(ruleset, state, tackler)).unwrap_or(i8::MAX),
    })
}

/// The standing enemy within reach most likely to stop `mover` leaving `from`, with the mover's
/// chance to escape.
pub fn most_dangerous_tackler(
    ruleset: &Ruleset,
    state: &GameState,
    mover: GoblinId,
    from: Square,
) -> Option<(GoblinId, Probability)> {
    let team = state.goblin(mover)?.team();
    state
        .standing_opponents_in_reach(from, team, ruleset.reach())
        .filter_map(|tackler| {
            tackle_dice(ruleset, state, mover, tackler.id).map(|dice| (dice.success(), tackler.id))
        })
        .min()
        .map(|(escape, tackler)| (tackler, escape))
}

/// Exact chances of `activation` for `team`, assuming every roll before a given step succeeded.
///
/// # Errors
/// The [`RuleViolation`] the engine would raise for the same order.
pub fn activation_odds(
    arena: &Arena,
    ruleset: &Ruleset,
    state: &GameState,
    team: TeamId,
    activation: &Activation,
) -> Result<ActivationOdds, RuleViolation> {
    validate_activation(arena, ruleset, state, team, activation)?;
    let id = activation.goblin;
    let agility = state
        .goblin(id)
        .map_or(1, |goblin| goblin.characteristics.agility);
    let mut sketch = state.clone();
    let mut risky_steps = Vec::new();
    let mut path_success = Probability::CERTAIN;
    let mut scores_on_the_way = false;
    for to in activation.path.iter().copied() {
        let Some(from) = sketch.goblin(id).and_then(|goblin| goblin.square()) else {
            break;
        };
        if let Some(dice) = dodge_dice(ruleset, &sketch, id, from) {
            let escape = dice.success();
            risky_steps.push(RiskyStep::Dodge {
                from,
                dice,
                success: escape,
            });
            path_success = path_success.and(escape);
        }
        if let Some(goblin) = sketch.goblin_mut(id) {
            goblin.presence = Presence::Standing(to);
        }
        if sketch.ball == (Ball::Loose { at: to }) {
            let malus = opponents_in_reach(ruleset, &sketch, to, team);
            let pick = agility_success(agility, malus, ruleset.ball_roll_target);
            risky_steps.push(RiskyStep::Pick {
                at: to,
                success: pick,
            });
            path_success = path_success.and(pick);
            sketch.ball = Ball::Carried { by: id };
        }
        if sketch.carrier() == Some(id) && arena.is_scoring_square_for(team, to) {
            scores_on_the_way = true;
            break;
        }
    }
    let action = if scores_on_the_way {
        None
    } else {
        activation
            .action
            .map(|action| action_odds(arena, ruleset, &sketch, id, action, activation.path.len()))
    };
    let action_success = match &action {
        None | Some(ActionOdds::Stance | ActionOdds::Focus) => Probability::CERTAIN,
        Some(
            ActionOdds::Assault { success, .. }
            | ActionOdds::Throw { success, .. }
            | ActionOdds::Chaos { success, .. },
        ) => *success,
    };
    Ok(ActivationOdds {
        risky_steps,
        path_success,
        scores_on_the_way,
        action,
        overall_success: path_success.and(action_success),
    })
}

/// The board once `activation` is through with every roll on the way won: the goblin at the end of
/// its path with what the order used spent, the ball it picked up on the way, a won assault's target
/// down, a caught throw in its receiver's hands, a braced goblin in Stance. Dice decide the health a
/// won assault takes and where a ball knocked loose bounces, so health stays as it is and such a
/// ball lies on its carrier's square. A goblin scoring on its way ends there, its action never played.
///
/// # Errors
/// The [`RuleViolation`] the engine would raise for the same order.
pub fn board_after_success(
    arena: &Arena,
    ruleset: &Ruleset,
    state: &GameState,
    team: TeamId,
    activation: &Activation,
) -> Result<GameState, RuleViolation> {
    validate_activation(arena, ruleset, state, team, activation)?;
    let id = activation.goblin;
    let mut board = state.clone();
    for to in activation.path.iter().copied() {
        if let Some(goblin) = board.goblin_mut(id) {
            goblin.presence = Presence::Standing(to);
        }
        if board.ball == (Ball::Loose { at: to }) {
            board.ball = Ball::Carried { by: id };
        }
        if board.carrier() == Some(id) && arena.is_scoring_square_for(team, to) {
            return Ok(board);
        }
    }
    if let Some(goblin) = board.goblin_mut(id) {
        goblin.spend_order(activation, ruleset);
    }
    match activation.action {
        None => {}
        Some(Action::Focus) => {
            if let Some(goblin) = board.goblin_mut(id) {
                goblin.focus = Focus::Focusing;
            }
        }
        Some(Action::Stance) => {
            if let Some(goblin) = board.goblin_mut(id) {
                goblin.in_stance = true;
            }
        }
        Some(Action::Assault { target, style }) => {
            let target_carried_the_ball = board.carrier() == Some(target);
            let landing = match style {
                AssaultStyle::Projection(direction) => board
                    .goblin(id)
                    .and_then(|attacker| attacker.square())
                    .and_then(|from| {
                        projection_landing(arena, &board, id, from, target, direction)
                    }),
                AssaultStyle::Simple | AssaultStyle::Chirurgical | AssaultStyle::Brutal => None,
            };
            if let Some(defender) = board.goblin_mut(target)
                && let Presence::Standing(square) = defender.presence
            {
                let fallen_at = landing.unwrap_or(square);
                defender.presence = Presence::Down(fallen_at);
                defender.in_stance = false;
                if target_carried_the_ball {
                    board.ball = Ball::Loose { at: fallen_at };
                }
            }
        }
        Some(Action::Throw {
            receiver,
            style: ThrowStyle::Simple | ThrowStyle::Focused,
        }) => board.ball = Ball::Carried { by: receiver },
        Some(Action::Throw {
            receiver,
            style: ThrowStyle::Chaos,
        }) => {
            if let Some(at) = board.goblin(receiver).and_then(|goblin| goblin.square()) {
                board.ball = Ball::Loose { at };
            }
        }
    }
    Ok(board)
}

fn action_odds(
    arena: &Arena,
    ruleset: &Ruleset,
    sketch: &GameState,
    actor: GoblinId,
    action: Action,
    steps_walked: usize,
) -> ActionOdds {
    match action {
        Action::Stance => ActionOdds::Stance,
        Action::Focus => ActionOdds::Focus,
        Action::Assault { target, style } => {
            let dice = styled_assault_dice(ruleset, sketch, actor, target, style);
            let chance_of = |chance: fn(ContestDice) -> Probability| {
                dice.map_or(Probability::IMPOSSIBLE, chance)
            };
            let (knock_down, stand_off) = if ruleset.assault_ties_knock_down {
                (chance_of(ContestDice::success), Probability::IMPOSSIBLE)
            } else {
                (
                    chance_of(ContestDice::clear_win),
                    chance_of(ContestDice::tie),
                )
            };
            ActionOdds::Assault {
                target,
                dice,
                success: chance_of(ContestDice::success),
                knock_down,
                stand_off,
            }
        }
        Action::Throw {
            receiver,
            style: ThrowStyle::Simple,
        } => throw_odds(ruleset, sketch, actor, receiver, 0),
        Action::Throw {
            receiver,
            style: ThrowStyle::Focused,
        } => {
            let focused_throw_bonus = sketch.goblin(actor).map_or(0, |goblin| {
                goblin.focused_throw_bonus(steps_walked, ruleset)
            });
            throw_odds(ruleset, sketch, actor, receiver, focused_throw_bonus)
        }
        Action::Throw {
            receiver,
            style: ThrowStyle::Chaos,
        } => chaos_odds(arena, ruleset, sketch, actor, receiver),
    }
}

fn throw_odds(
    ruleset: &Ruleset,
    sketch: &GameState,
    thrower: GoblinId,
    receiver: GoblinId,
    focused_throw_bonus: u8,
) -> ActionOdds {
    let target = ruleset.ball_roll_target;
    let thrower_goblin = sketch.goblin(thrower);
    let team = thrower_goblin.map_or(TeamId::HOME, |goblin| goblin.team());
    let thrower_square = thrower_goblin.and_then(|goblin| goblin.square());
    let receiver_goblin = sketch.goblin(receiver);
    let receiver_square = receiver_goblin.and_then(|goblin| goblin.square());
    let (Some(thrower_square), Some(receiver_square)) = (thrower_square, receiver_square) else {
        return ActionOdds::Throw {
            receiver,
            throw: Probability::IMPOSSIBLE,
            interceptions: Vec::new(),
            catch: Probability::IMPOSSIBLE,
            success: Probability::IMPOSSIBLE,
        };
    };
    let distance = thrower_square.orthogonal_distance(receiver_square);
    let distance_malus = distance.saturating_sub(ruleset.throw_free_range);
    let hands_off = ruleset.adjacent_throws_are_hand_offs && distance == 1;
    let throw = if hands_off {
        Probability::CERTAIN
    } else {
        AgilityDice {
            faces: thrower_goblin.map_or(1, |goblin| goblin.characteristics.agility),
            malus: opponents_in_reach(ruleset, sketch, thrower_square, team)
                .saturating_add(distance_malus),
            bonus: thrower_goblin
                .map_or(0, |goblin| goblin.focus.throw_bonus())
                .saturating_add(focused_throw_bonus),
            target,
        }
        .success()
    };
    let interceptions =
        interception_chances(ruleset, sketch, thrower_square, receiver_square, team);
    let catch = agility_success(
        receiver_goblin.map_or(1, |goblin| goblin.characteristics.agility),
        opponents_in_reach(ruleset, sketch, receiver_square, team),
        target,
    );
    let success = interceptions
        .iter()
        .fold(throw, |chance, (_, interception)| {
            chance.and(interception.complement())
        })
        .and(catch);
    ActionOdds::Throw {
        receiver,
        throw,
        interceptions,
        catch,
        success,
    }
}

/// Standing enemies of `team` that may try to intercept a ball thrown from `from` to `to`, in the
/// order the ball reaches them: those on a square it flies over and, where a goblin's reach covers
/// the diagonals, those standing next to one of those squares too.
pub fn interceptors(
    ruleset: &Ruleset,
    state: &GameState,
    from: Square,
    to: Square,
    team: TeamId,
) -> Vec<GoblinId> {
    let mut found: Vec<GoblinId> = Vec::new();
    for square in squares_crossed(from, to) {
        for enemy in state
            .goblins
            .iter()
            .filter(|goblin| goblin.team() != team && goblin.is_standing())
        {
            let on_the_line = enemy.square().is_some_and(|at| {
                at == square
                    || (ruleset.reach_covers_diagonals && at.is_within(square, Reach::AllAround))
            });
            if on_the_line && !found.contains(&enemy.id) {
                found.push(enemy.id);
            }
        }
    }
    found
}

/// Each enemy that may intercept a ball thrown from `from` to `to`, in order, with its chance.
fn interception_chances(
    ruleset: &Ruleset,
    sketch: &GameState,
    from: Square,
    to: Square,
    team: TeamId,
) -> Vec<(GoblinId, Probability)> {
    interceptors(ruleset, sketch, from, to, team)
        .into_iter()
        .filter_map(|id| sketch.goblin(id))
        .map(|interceptor| {
            let malus = opponents_in_reach(
                ruleset,
                sketch,
                interceptor.square().unwrap_or(from),
                interceptor.team(),
            )
            .saturating_add(ruleset.interception_surprise_malus);
            (
                interceptor.id,
                agility_success(
                    interceptor.characteristics.agility,
                    malus,
                    ruleset.ball_roll_target,
                ),
            )
        })
        .collect()
}

/// Chances of a chaos throw towards `receiver`: each interception on the way, then a teammate
/// holding the ball once it bounced three times from the receiver's square.
fn chaos_odds(
    arena: &Arena,
    ruleset: &Ruleset,
    sketch: &GameState,
    thrower: GoblinId,
    receiver: GoblinId,
) -> ActionOdds {
    let from = sketch.goblin(thrower).and_then(|goblin| goblin.square());
    let to = sketch.goblin(receiver).and_then(|goblin| goblin.square());
    let interceptions = from
        .zip(to)
        .map(|(from, to)| interception_chances(ruleset, sketch, from, to, thrower.team))
        .unwrap_or_default();
    let kept = to.map_or(Probability::IMPOSSIBLE, |to| {
        ChaosBounces::new(arena, ruleset, sketch, thrower.team).chance_kept_from(to)
    });
    let success = interceptions
        .iter()
        .fold(kept, |chance, (_, interception)| {
            chance.and(interception.complement())
        });
    ActionOdds::Chaos {
        receiver,
        interceptions,
        kept,
        success,
    }
}

/// The bounces of a ball thrown into chaos, weighed for the chance a goblin of `team` ends up
/// holding it: every square the direction die may send it to, each catch a goblin standing there
/// may make or miss, the ball going on past a miss or a goblin lying down.
struct ChaosBounces<'a> {
    arena: &'a Arena,
    ruleset: &'a Ruleset,
    state: &'a GameState,
    team: TeamId,
    known: HashMap<(Square, u8, usize), f64>,
}

impl<'a> ChaosBounces<'a> {
    fn new(arena: &'a Arena, ruleset: &'a Ruleset, state: &'a GameState, team: TeamId) -> Self {
        Self {
            arena,
            ruleset,
            state,
            team,
            known: HashMap::new(),
        }
    }

    /// Chance a teammate holds the ball once it bounced [`CHAOS_BOUNCES`] times from `square`.
    fn chance_kept_from(mut self, square: Square) -> Probability {
        Probability::nearest(self.kept(square, CHAOS_BOUNCES, 0))
    }

    /// Chance a teammate ends up holding a ball on `square` with `bounces_left` bounces to make,
    /// `moves` squares into the current one; a bounce sent on [`MOST_MOVES_IN_A_BOUNCE`] times in a
    /// row counts as lost.
    fn kept(&mut self, square: Square, bounces_left: u8, moves: usize) -> f64 {
        if bounces_left == 0 || moves == MOST_MOVES_IN_A_BOUNCE {
            return 0.0;
        }
        if let Some(chance) = self.known.get(&(square, bounces_left, moves)) {
            return *chance;
        }
        let chance = bounce_chances(self.arena, self.ruleset, square)
            .into_iter()
            .map(|(to, weight)| weight * self.kept_after_landing(to, bounces_left, moves))
            .sum();
        self.known.insert((square, bounces_left, moves), chance);
        chance
    }

    fn kept_after_landing(&mut self, square: Square, bounces_left: u8, moves: usize) -> f64 {
        let state = self.state;
        let Some(goblin) = state.goblin_at(square) else {
            return self.kept(square, bounces_left - 1, 0);
        };
        if !goblin.is_standing() {
            return self.kept(square, bounces_left, moves + 1);
        }
        let catch = share(agility_success(
            goblin.characteristics.agility,
            opponents_in_reach(self.ruleset, state, square, goblin.team()),
            self.ruleset.ball_roll_target,
        ));
        let caught_by_a_teammate = if goblin.team() == self.team {
            catch
        } else {
            0.0
        };
        caught_by_a_teammate + (1.0 - catch) * self.kept(square, bounces_left, moves + 1)
    }
}

/// Where a ball on `from` may bounce, with the chance of each square: the in-bounds squares the
/// direction die points to, alike, and its settling square once every roll pointed out of bounds.
fn bounce_chances(arena: &Arena, ruleset: &Ruleset, from: Square) -> Vec<(Square, f64)> {
    let faces = direction_die(ruleset);
    let landings: Vec<Square> = (1..=faces)
        .filter_map(|face| landing(arena, ruleset, from, face))
        .collect();
    let pointing_out = (f64::from(faces) - landings.len() as f64) / f64::from(faces);
    let out_every_roll = (0..MOST_DIRECTION_ROLLS).fold(1.0, |chance, _| chance * pointing_out);
    let each = (1.0 - out_every_roll) / landings.len().max(1) as f64;
    landings
        .into_iter()
        .map(|square| (square, each))
        .chain((out_every_roll > 0.0).then(|| (settling_square(arena, from), out_every_roll)))
        .collect()
}

fn share(chance: Probability) -> f64 {
    chance.numerator() as f64 / chance.denominator() as f64
}

fn opponents_in_reach(ruleset: &Ruleset, state: &GameState, square: Square, team: TeamId) -> u8 {
    u8::try_from(
        state
            .standing_opponents_in_reach(square, team, ruleset.reach())
            .count(),
    )
    .unwrap_or(u8::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::characteristics::Archetype::{Bruiser, Sprinter, Thrower};
    use crate::command::Action;
    use crate::game::scenario::Scenario;

    fn id(text: &str) -> GoblinId {
        text.parse().unwrap()
    }

    fn order(goblin: &str, path: &[(u8, u8)], action: Option<Action>) -> Activation {
        Activation {
            goblin: id(goblin),
            path: path.iter().map(|(x, y)| Square::new(*x, *y)).collect(),
            action,
        }
    }

    fn played_out(scenario: Scenario, activation: &Activation) -> Result<GameState, RuleViolation> {
        let game = scenario
            .rules(|ruleset| ruleset.keeps_moving_after_action = true)
            .game([]);
        board_after_success(
            game.arena(),
            game.ruleset(),
            game.state(),
            TeamId::HOME,
            activation,
        )
    }

    #[test]
    fn given_goblins_that_keep_moving_when_a_walk_is_played_out_then_it_stands_at_its_end_with_the_rest_of_its_move()
     {
        let board = played_out(
            Scenario::new().goblin("H5", Sprinter, 3, 7),
            &order("H5", &[(4, 7), (5, 7)], None),
        )
        .unwrap();
        let sprinter = board.goblin(id("H5")).unwrap();
        assert_eq!(
            (
                sprinter.presence,
                sprinter.steps_left,
                sprinter.has_action_left
            ),
            (Presence::Standing(Square::new(5, 7)), 3, true)
        );
    }

    #[test]
    fn given_a_walk_onto_the_loose_ball_when_played_out_then_the_goblin_carries_it() {
        let board = played_out(
            Scenario::new()
                .goblin("H5", Sprinter, 3, 7)
                .ball_loose_at(5, 7),
            &order("H5", &[(4, 7), (5, 7)], None),
        )
        .unwrap();
        assert_eq!(board.ball, Ball::Carried { by: id("H5") });
    }

    #[test]
    fn given_a_throw_when_played_out_then_the_receiver_holds_the_ball_and_the_thrower_s_action_is_used()
     {
        let board = played_out(
            Scenario::new()
                .goblin("H9", Thrower, 4, 7)
                .goblin("H5", Sprinter, 8, 7)
                .ball_carried_by("H9"),
            &order("H9", &[], Some(Action::throw(id("H5")))),
        )
        .unwrap();
        assert_eq!(
            (board.ball, board.goblin(id("H9")).unwrap().has_action_left),
            (Ball::Carried { by: id("H5") }, false)
        );
    }

    #[test]
    fn given_an_assault_on_the_enemy_carrier_when_played_out_then_it_is_down_and_the_ball_lies_on_its_square()
     {
        let board = played_out(
            Scenario::new()
                .goblin("H1", Bruiser, 5, 7)
                .goblin("A5", Sprinter, 6, 7)
                .ball_carried_by("A5"),
            &order("H1", &[], Some(Action::assault(id("A5")))),
        )
        .unwrap();
        assert_eq!(
            (board.goblin(id("A5")).unwrap().presence, board.ball),
            (
                Presence::Down(Square::new(6, 7)),
                Ball::Loose {
                    at: Square::new(6, 7)
                }
            )
        );
    }

    #[test]
    fn given_a_projection_of_the_enemy_carrier_when_played_out_then_it_is_down_on_its_landing_square_with_the_ball()
     {
        let board = played_out(
            Scenario::new()
                .rules(|ruleset| ruleset.assaults_have_styles = true)
                .goblin("H1", Bruiser, 5, 7)
                .goblin("A5", Sprinter, 6, 7)
                .ball_carried_by("A5"),
            &order(
                "H1",
                &[],
                Some(Action::Assault {
                    target: id("A5"),
                    style: AssaultStyle::Projection(crate::square::Compass::South),
                }),
            ),
        )
        .unwrap();
        assert_eq!(
            (board.goblin(id("A5")).unwrap().presence, board.ball),
            (
                Presence::Down(Square::new(5, 8)),
                Ball::Loose {
                    at: Square::new(5, 8)
                }
            )
        );
    }

    #[test]
    fn given_a_stance_when_played_out_then_the_goblin_is_braced() {
        let board = played_out(
            Scenario::new().goblin("H1", Bruiser, 5, 7),
            &order("H1", &[], Some(Action::Stance)),
        )
        .unwrap();
        assert!(board.goblin(id("H1")).unwrap().in_stance);
    }

    #[test]
    fn given_a_walk_beyond_the_steps_left_when_played_out_then_the_engine_violation() {
        let refusal = played_out(
            Scenario::new().goblin("H1", Bruiser, 5, 7),
            &order("H1", &[(5, 6), (5, 5), (5, 4), (5, 3)], None),
        );
        assert_eq!(
            refusal,
            Err(RuleViolation::PathTooLong {
                goblin: id("H1"),
                steps_left: 3,
                steps: 4
            })
        );
    }

    fn odds(scenario: Scenario, activation: &Activation) -> Result<ActivationOdds, RuleViolation> {
        let game = scenario.game([]);
        activation_odds(
            game.arena(),
            game.ruleset(),
            game.state(),
            TeamId::HOME,
            activation,
        )
    }

    fn dice_of<Dice>(
        scenario: Scenario,
        read: impl FnOnce(&Ruleset, &GameState) -> Option<Dice>,
    ) -> Option<Dice> {
        let game = scenario.game([]);
        read(game.ruleset(), game.state())
    }

    #[test]
    fn given_a_defender_braced_beside_a_braced_teammate_when_reading_assault_dice_then_its_die_gets_two()
     {
        let scenario = Scenario::new()
            .goblin("H1", Bruiser, 5, 7)
            .goblin("A1", Sprinter, 6, 7)
            .goblin("A2", Bruiser, 6, 8)
            .edit("A1", |goblin| goblin.in_stance = true)
            .edit("A2", |goblin| goblin.in_stance = true);
        assert_eq!(
            dice_of(scenario, |ruleset, state| assault_dice(
                ruleset,
                state,
                id("H1"),
                id("A1")
            )),
            Some(ContestDice {
                actor_modifier: 0,
                actor_faces: 4,
                opponent_faces: 2,
                opponent_modifier: 2
            })
        );
    }

    #[test]
    fn given_a_stance_cap_of_one_when_reading_the_dice_against_a_braced_wall_then_only_one_counts()
    {
        let scenario = Scenario::new()
            .goblin("H1", Bruiser, 5, 7)
            .goblin("A1", Sprinter, 6, 7)
            .goblin("A2", Bruiser, 6, 8)
            .edit("A1", |goblin| goblin.in_stance = true)
            .edit("A2", |goblin| goblin.in_stance = true);
        let game = scenario.game([]);
        let capped = Ruleset {
            stance_bonus_cap: 1,
            ..game.ruleset().clone()
        };
        assert_eq!(
            assault_dice(&capped, game.state(), id("H1"), id("A1"))
                .map(|dice| dice.opponent_modifier),
            Some(1)
        );
    }

    #[test]
    fn given_a_braced_defender_between_two_braced_teammates_when_neighbours_count_once_then_its_die_gets_two()
     {
        let scenario = Scenario::new()
            .goblin("H1", Bruiser, 5, 7)
            .goblin("A1", Bruiser, 6, 7)
            .goblin("A2", Bruiser, 6, 6)
            .goblin("A3", Bruiser, 6, 8)
            .edit("A1", |goblin| goblin.in_stance = true)
            .edit("A2", |goblin| goblin.in_stance = true)
            .edit("A3", |goblin| goblin.in_stance = true);
        let game = scenario.game([]);
        let neighbours_once = Ruleset {
            neighbour_stance_bonus_cap: 1,
            ..game.ruleset().clone()
        };
        assert_eq!(
            assault_dice(&neighbours_once, game.state(), id("H1"), id("A1"))
                .map(|dice| dice.opponent_modifier),
            Some(2)
        );
    }

    #[test]
    fn given_a_downed_defender_when_reading_assault_dice_then_its_die_loses_the_downed_malus() {
        let scenario = Scenario::new()
            .goblin("H1", Sprinter, 5, 7)
            .goblin("A1", Bruiser, 6, 7)
            .edit("A1", |goblin| {
                goblin.presence = Presence::Down(Square::new(6, 7))
            });
        assert_eq!(
            dice_of(scenario, |ruleset, state| assault_dice(
                ruleset,
                state,
                id("H1"),
                id("A1")
            ))
            .map(|dice| dice.opponent_modifier),
            Some(-2)
        );
    }

    #[test]
    fn given_a_carrier_malus_of_one_when_reading_the_dice_against_the_carrier_then_its_die_loses_one()
     {
        let scenario = Scenario::new()
            .rules(|ruleset| ruleset.carrier_defense_malus = 1)
            .goblin("H1", Bruiser, 5, 7)
            .goblin("A1", Sprinter, 6, 7)
            .ball_carried_by("A1");
        assert_eq!(
            dice_of(scenario, |ruleset, state| assault_dice(
                ruleset,
                state,
                id("H1"),
                id("A1")
            ))
            .map(|dice| dice.opponent_modifier),
            Some(-1)
        );
    }

    #[test]
    fn given_a_carrier_malus_of_one_when_reading_the_dice_against_another_goblin_then_none() {
        let scenario = Scenario::new()
            .rules(|ruleset| ruleset.carrier_defense_malus = 1)
            .goblin("H1", Bruiser, 5, 7)
            .goblin("A1", Sprinter, 6, 7)
            .goblin("A2", Sprinter, 9, 9)
            .ball_carried_by("A2");
        assert_eq!(
            dice_of(scenario, |ruleset, state| assault_dice(
                ruleset,
                state,
                id("H1"),
                id("A1")
            ))
            .map(|dice| dice.opponent_modifier),
            Some(0)
        );
    }

    #[test]
    fn given_an_unknown_defender_when_reading_assault_dice_then_none() {
        let scenario = Scenario::new().goblin("H1", Bruiser, 5, 7);
        assert_eq!(
            dice_of(scenario, |ruleset, state| assault_dice(
                ruleset,
                state,
                id("H1"),
                id("A9")
            )),
            None
        );
    }

    #[test]
    fn given_a_braced_tackler_when_reading_dodge_dice_then_agility_against_its_defense_plus_one() {
        let scenario = sprinter_next_to_a_bruiser().edit("A1", |goblin| goblin.in_stance = true);
        assert_eq!(
            dice_of(scenario, |ruleset, state| dodge_dice(
                ruleset,
                state,
                id("H1"),
                Square::new(5, 7)
            )),
            Some(DodgeDice::Contest {
                tackler: id("A1"),
                contest: ContestDice {
                    actor_modifier: 0,
                    actor_faces: 4,
                    opponent_faces: 4,
                    opponent_modifier: 1
                }
            })
        );
    }

    #[test]
    fn given_agility_dodges_when_a_sprinter_leaves_a_plain_and_a_braced_enemy_then_its_d4_takes_three()
     {
        let scenario = sprinter_next_to_a_bruiser()
            .rules(|ruleset| ruleset.dodges_test_agility = true)
            .goblin("A2", Sprinter, 5, 6)
            .edit("A1", |goblin| goblin.in_stance = true);
        assert_eq!(
            dice_of(scenario, |ruleset, state| dodge_dice(
                ruleset,
                state,
                id("H1"),
                Square::new(5, 7)
            )),
            Some(DodgeDice::Agility(AgilityDice {
                bonus: 0,
                faces: 4,
                malus: 3,
                target: 2
            }))
        );
    }

    #[test]
    fn given_braced_tacklers_that_weigh_once_when_a_sprinter_leaves_a_plain_and_a_braced_enemy_then_its_d4_takes_two()
     {
        let scenario = sprinter_next_to_a_bruiser()
            .rules(|ruleset| {
                ruleset.dodges_test_agility = true;
                ruleset.braced_tacklers_weigh_double = false;
            })
            .goblin("A2", Sprinter, 5, 6)
            .edit("A1", |goblin| goblin.in_stance = true);
        assert_eq!(
            dice_of(scenario, |ruleset, state| dodge_dice(
                ruleset,
                state,
                id("H1"),
                Square::new(5, 7)
            )),
            Some(DodgeDice::Agility(AgilityDice {
                bonus: 0,
                faces: 4,
                malus: 2,
                target: 2
            }))
        );
    }

    #[test]
    fn given_a_reach_all_around_when_a_sprinter_leaves_an_enemy_on_its_diagonal_then_it_dodges_at_one()
     {
        let scenario = Scenario::new()
            .rules(|ruleset| {
                ruleset.dodges_test_agility = true;
                ruleset.reach_covers_diagonals = true;
            })
            .goblin("H1", Sprinter, 5, 7)
            .goblin("A1", Bruiser, 6, 6);
        assert_eq!(
            dice_of(scenario, |ruleset, state| dodge_dice(
                ruleset,
                state,
                id("H1"),
                Square::new(5, 7)
            )),
            Some(DodgeDice::Agility(AgilityDice {
                bonus: 0,
                faces: 4,
                malus: 1,
                target: 2
            }))
        );
    }

    #[test]
    fn given_a_reach_all_around_when_a_throw_flies_past_an_enemy_next_to_the_line_then_its_interception_is_listed()
     {
        let scenario = Scenario::new()
            .rules(|ruleset| ruleset.reach_covers_diagonals = true)
            .goblin("H1", Thrower, 4, 7)
            .goblin("H2", Sprinter, 8, 7)
            .goblin("A1", Sprinter, 6, 8)
            .ball_carried_by("H1");
        let pass = order("H1", &[], Some(Action::throw(id("H2"))));
        assert!(matches!(
            odds(scenario, &pass).unwrap().action,
            Some(ActionOdds::Throw { interceptions, .. })
                if interceptions == vec![(id("A1"), Probability::new(1, 2))]
        ));
    }

    #[test]
    fn given_agility_dodges_when_the_only_enemy_around_is_down_then_no_dodge() {
        let scenario = sprinter_next_to_a_bruiser()
            .rules(|ruleset| ruleset.dodges_test_agility = true)
            .edit("A1", |goblin| {
                goblin.presence = Presence::Down(Square::new(6, 7));
            });
        assert_eq!(
            dice_of(scenario, |ruleset, state| dodge_dice(
                ruleset,
                state,
                id("H1"),
                Square::new(5, 7)
            )),
            None
        );
    }

    fn sprinter_next_to_a_bruiser() -> Scenario {
        Scenario::new()
            .goblin("H1", Sprinter, 5, 7)
            .goblin("A1", Bruiser, 6, 7)
    }

    #[test]
    fn given_a_free_walk_when_previewed_then_certain() {
        let walk = order("H1", &[(5, 8)], None);
        let preview = odds(Scenario::new().goblin("H1", Sprinter, 5, 7), &walk).unwrap();
        assert_eq!(preview.overall_success, Probability::CERTAIN);
    }

    #[test]
    fn given_hand_offs_when_previewing_a_throw_to_an_adjacent_teammate_then_only_the_catch_is_at_risk()
     {
        let scenario = Scenario::new()
            .rules(|ruleset| ruleset.adjacent_throws_are_hand_offs = true)
            .goblin("H1", Thrower, 4, 7)
            .goblin("H2", Sprinter, 5, 7)
            .ball_carried_by("H1");
        let hand_off = order("H1", &[], Some(Action::throw(id("H2"))));
        let preview = odds(scenario, &hand_off).unwrap();
        assert!(matches!(
            preview.action,
            Some(ActionOdds::Throw { throw, catch, .. })
                if throw == Probability::CERTAIN && catch == Probability::new(3, 4)
        ));
    }

    #[test]
    fn given_a_step_away_from_a_bruiser_when_previewed_then_five_eighths() {
        let walk = order("H1", &[(4, 7)], None);
        let preview = odds(sprinter_next_to_a_bruiser(), &walk).unwrap();
        assert_eq!(preview.overall_success, Probability::new(5, 8));
    }

    #[test]
    fn given_a_step_away_from_a_bruiser_when_previewed_then_the_dodge_is_listed() {
        let walk = order("H1", &[(4, 7)], None);
        let preview = odds(sprinter_next_to_a_bruiser(), &walk).unwrap();
        assert_eq!(
            preview.risky_steps,
            vec![RiskyStep::Dodge {
                from: Square::new(5, 7),
                dice: DodgeDice::Contest {
                    tackler: id("A1"),
                    contest: ContestDice {
                        actor_modifier: 0,
                        actor_faces: 4,
                        opponent_faces: 4,
                        opponent_modifier: 0
                    }
                },
                success: Probability::new(5, 8)
            }]
        );
    }

    #[test]
    fn given_agility_dodges_when_a_sprinter_steps_away_from_a_bruiser_then_one_half() {
        let walk = order("H1", &[(4, 7)], None);
        let scenario =
            sprinter_next_to_a_bruiser().rules(|ruleset| ruleset.dodges_test_agility = true);
        let preview = odds(scenario, &walk).unwrap();
        assert_eq!(
            preview.risky_steps,
            vec![RiskyStep::Dodge {
                from: Square::new(5, 7),
                dice: DodgeDice::Agility(AgilityDice {
                    bonus: 0,
                    faces: 4,
                    malus: 1,
                    target: 2
                }),
                success: Probability::new(1, 2)
            }]
        );
    }

    #[test]
    fn given_an_assault_on_a_braced_defender_when_previewed_then_its_dice_are_listed() {
        let scenario = Scenario::new()
            .goblin("H1", Bruiser, 5, 7)
            .goblin("A1", Bruiser, 6, 7)
            .edit("A1", |goblin| goblin.in_stance = true);
        let assault = order("H1", &[], Some(Action::assault(id("A1"))));
        assert_eq!(
            odds(scenario, &assault).unwrap().action,
            Some(ActionOdds::Assault {
                target: id("A1"),
                dice: Some(ContestDice {
                    actor_modifier: 0,
                    actor_faces: 4,
                    opponent_faces: 4,
                    opponent_modifier: 1
                }),
                success: Probability::new(3, 8),
                knock_down: Probability::new(3, 8),
                stand_off: Probability::IMPOSSIBLE
            })
        );
    }

    #[test]
    fn given_assault_styles_when_a_bruiser_assaults_a_bruiser_chirurgically_then_five_eighths_knock_down_and_three_sixteenths_stand_off()
     {
        let scenario = Scenario::new()
            .rules(|ruleset| {
                ruleset.assault_ties_knock_down = false;
                ruleset.assaults_have_styles = true;
            })
            .goblin("H1", Bruiser, 5, 7)
            .goblin("A1", Bruiser, 6, 7);
        let chirurgical = order(
            "H1",
            &[],
            Some(Action::Assault {
                target: id("A1"),
                style: AssaultStyle::Chirurgical,
            }),
        );
        assert!(matches!(
            odds(scenario, &chirurgical).unwrap().action,
            Some(ActionOdds::Assault { success, knock_down, stand_off, .. })
                if (success, knock_down, stand_off)
                    == (Probability::new(13, 16), Probability::new(5, 8), Probability::new(3, 16))
        ));
    }

    #[test]
    fn given_ties_that_stand_off_when_two_bruisers_face_off_then_three_eighths_knock_down_and_one_quarter_stand_off()
     {
        let scenario = Scenario::new()
            .rules(|ruleset| ruleset.assault_ties_knock_down = false)
            .goblin("H1", Bruiser, 5, 7)
            .goblin("A1", Bruiser, 6, 7);
        let assault = order("H1", &[], Some(Action::assault(id("A1"))));
        assert!(matches!(
            odds(scenario, &assault).unwrap().action,
            Some(ActionOdds::Assault { success, knock_down, stand_off, .. })
                if (success, knock_down, stand_off)
                    == (Probability::new(5, 8), Probability::new(3, 8), Probability::new(1, 4))
        ));
    }

    #[test]
    fn given_a_dodge_then_a_pick_when_previewed_then_both_chances_multiply() {
        let walk = order("H1", &[(4, 7), (3, 7)], None);
        let preview = odds(sprinter_next_to_a_bruiser().ball_loose_at(3, 7), &walk).unwrap();
        assert_eq!(preview.overall_success, Probability::new(15, 32));
    }

    #[test]
    fn given_an_assault_on_a_braced_bruiser_when_previewed_then_three_eighths() {
        let scenario = Scenario::new()
            .goblin("H1", Bruiser, 5, 7)
            .goblin("A1", Bruiser, 6, 7)
            .edit("A1", |goblin| goblin.in_stance = true);
        let assault = order("H1", &[], Some(Action::assault(id("A1"))));
        assert_eq!(
            odds(scenario, &assault).unwrap().overall_success,
            Probability::new(3, 8)
        );
    }

    #[test]
    fn given_a_throw_past_an_interceptor_when_previewed_then_throw_times_miss_times_catch() {
        let scenario = Scenario::new()
            .goblin("H1", Thrower, 4, 7)
            .goblin("H2", Sprinter, 8, 7)
            .goblin("A1", Sprinter, 6, 7)
            .ball_carried_by("H1");
        let pass = order("H1", &[], Some(Action::throw(id("H2"))));
        assert_eq!(
            odds(scenario, &pass).unwrap().overall_success,
            Probability::new(9, 32)
        );
    }

    #[test]
    fn given_a_throw_past_an_interceptor_when_previewed_then_its_interception_chance_is_listed() {
        let scenario = Scenario::new()
            .goblin("H1", Thrower, 4, 7)
            .goblin("H2", Sprinter, 8, 7)
            .goblin("A1", Sprinter, 6, 7)
            .ball_carried_by("H1");
        let pass = order("H1", &[], Some(Action::throw(id("H2"))));
        assert!(matches!(
            odds(scenario, &pass).unwrap().action,
            Some(ActionOdds::Throw { interceptions, .. })
                if interceptions == vec![(id("A1"), Probability::new(1, 2))]
        ));
    }

    #[test]
    fn given_chaos_throws_when_a_chaos_throw_flies_over_an_enemy_then_only_its_interception_is_at_risk()
     {
        let scenario = Scenario::new()
            .rules(|ruleset| ruleset.throws_have_styles = true)
            .goblin("H1", Thrower, 4, 7)
            .goblin("H2", Sprinter, 8, 7)
            .goblin("A1", Sprinter, 6, 7)
            .ball_carried_by("H1");
        let chaos = order(
            "H1",
            &[],
            Some(Action::Throw {
                receiver: id("H2"),
                style: ThrowStyle::Chaos,
            }),
        );
        let preview = odds(scenario, &chaos).unwrap();
        assert!(matches!(
            preview.action,
            Some(ActionOdds::Chaos { interceptions, kept, success, .. })
                if interceptions == vec![(id("A1"), Probability::new(1, 2))]
                    && success == kept.and(Probability::new(1, 2))
        ));
    }

    fn chaos_towards(receiver: &str) -> Activation {
        order(
            "H1",
            &[],
            Some(Action::Throw {
                receiver: id(receiver),
                style: ThrowStyle::Chaos,
            }),
        )
    }

    fn sure_hands_and_chaos(bouncing_all_around: bool) -> Scenario {
        Scenario::new()
            .rules(|ruleset| {
                ruleset.throws_have_styles = true;
                ruleset.ball_roll_target = 1;
                ruleset.ball_bounces_all_around = bouncing_all_around;
            })
            .goblin("H1", Thrower, 4, 7)
            .goblin("H2", Sprinter, 8, 7)
            .ball_carried_by("H1")
    }

    #[test]
    fn given_a_lone_receiver_with_sure_hands_when_previewing_a_chaos_throw_then_only_a_bounce_back_to_it_keeps_the_ball()
     {
        let preview = odds(sure_hands_and_chaos(true), &chaos_towards("H2")).unwrap();
        assert!(matches!(
            preview.action,
            Some(ActionOdds::Chaos { kept, success, .. })
                if kept == Probability::new(11, 64) && success == kept
        ));
    }

    #[test]
    fn given_a_receiver_ringed_by_teammates_with_sure_hands_when_previewing_a_chaos_throw_then_the_ball_is_kept()
     {
        let scenario = sure_hands_and_chaos(false)
            .goblin("H3", Sprinter, 8, 6)
            .goblin("H4", Sprinter, 9, 7)
            .goblin("H5", Sprinter, 8, 8)
            .goblin("H6", Sprinter, 7, 7);
        let preview = odds(scenario, &chaos_towards("H2")).unwrap();
        assert_eq!(preview.overall_success, Probability::CERTAIN);
    }

    #[test]
    fn given_focused_throws_when_previewing_a_thrower_s_focused_throw_then_three_steps_sacrificed_make_its_throw_certain()
     {
        let scenario = Scenario::new()
            .rules(|ruleset| {
                ruleset.keeps_moving_after_action = true;
                ruleset.actions_cost_a_step = true;
                ruleset.focused_throws_spend_steps_left = true;
            })
            .goblin("H1", Thrower, 4, 7)
            .goblin("H2", Sprinter, 8, 7)
            .ball_carried_by("H1");
        let focused = order(
            "H1",
            &[],
            Some(Action::Throw {
                receiver: id("H2"),
                style: ThrowStyle::Focused,
            }),
        );
        assert!(matches!(
            odds(scenario, &focused).unwrap().action,
            Some(ActionOdds::Throw { throw, .. }) if throw == Probability::CERTAIN
        ));
    }

    #[test]
    fn given_a_flat_focus_bonus_of_three_when_previewing_a_standing_thrower_s_eleven_step_focused_pass_then_one_chance_in_four()
     {
        let scenario = Scenario::new()
            .rules(|ruleset| {
                ruleset.keeps_moving_after_action = true;
                ruleset.focused_throws_spend_steps_left = true;
                ruleset.focused_throw_bonus = Some(3);
            })
            .goblin("H1", Thrower, 2, 7)
            .goblin("H2", Sprinter, 13, 7)
            .ball_carried_by("H1");
        let focused = order(
            "H1",
            &[],
            Some(Action::Throw {
                receiver: id("H2"),
                style: ThrowStyle::Focused,
            }),
        );

        let preview = odds(scenario, &focused).unwrap();

        assert!(matches!(
            preview.action,
            Some(ActionOdds::Throw { throw, .. }) if throw == Probability::new(1, 4)
        ));
    }

    #[test]
    fn given_a_carrier_reaching_the_enemy_den_when_previewed_then_it_scores_before_acting() {
        let scenario = Scenario::new()
            .goblin("H1", Sprinter, 13, 7)
            .ball_carried_by("H1");
        let dash = order("H1", &[(14, 7)], Some(Action::Stance));
        let preview = odds(scenario, &dash).unwrap();
        assert_eq!((preview.scores_on_the_way, preview.action), (true, None));
    }

    #[test]
    fn given_an_illegal_order_when_previewed_then_the_engine_violation() {
        let walk = order("H1", &[(7, 7)], None);
        assert_eq!(
            odds(sprinter_next_to_a_bruiser(), &walk),
            Err(RuleViolation::PathNotContiguous {
                index: 0,
                square: Square::new(7, 7)
            })
        );
    }

    #[test]
    fn given_a_sprinter_between_two_enemies_when_finding_the_tackler_then_the_bruiser() {
        let game = Scenario::new()
            .goblin("H1", Sprinter, 5, 7)
            .goblin("A1", Sprinter, 6, 7)
            .goblin("A2", Bruiser, 5, 6)
            .game([]);
        assert_eq!(
            most_dangerous_tackler(game.ruleset(), game.state(), id("H1"), Square::new(5, 7)),
            Some((id("A2"), Probability::new(5, 8)))
        );
    }
}
