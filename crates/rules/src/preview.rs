//! What a planned activation risks, computed exactly before any die is rolled.

use serde::{Deserialize, Serialize};

use crate::arena::Arena;
use crate::command::{Action, Activation};
use crate::flight::squares_crossed;
use crate::odds::{Probability, ball_roll_success, contest_success};
use crate::roster::GoblinId;
use crate::ruleset::Ruleset;
use crate::square::Square;
use crate::state::{Ball, GameState, Presence};
use crate::team::TeamId;
use crate::validation::validate_activation;
use crate::violation::RuleViolation;

/// A moment of an activation that can end the turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "risk", rename_all = "snake_case")]
pub enum RiskyStep {
    /// Leaving a square next to standing enemies.
    Dodge {
        /// Square left.
        from: Square,
        /// The enemy most likely to stop the goblin.
        tackler: GoblinId,
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
    /// Chance that the attacker wins (knocks down or hurts the target).
    Assault {
        /// The target.
        target: GoblinId,
        /// Chance to win the contest.
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

/// The adjacent standing enemy most likely to stop `mover` leaving `from`, with the mover's chance to escape.
pub fn most_dangerous_tackler(
    state: &GameState,
    mover: GoblinId,
    from: Square,
) -> Option<(GoblinId, Probability)> {
    let mover = state.goblin(mover)?;
    let agility = mover.characteristics.agility;
    state
        .standing_opponents_next_to(from, mover.team())
        .map(|tackler| {
            let bonus = i8::try_from(state.stance_bonus(tackler.id)).unwrap_or(i8::MAX);
            (
                contest_success(agility, tackler.characteristics.defense, bonus),
                tackler.id,
            )
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
    validate_activation(arena, state, team, activation)?;
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
        if let Some((tackler, escape)) = most_dangerous_tackler(&sketch, id, from) {
            risky_steps.push(RiskyStep::Dodge {
                from,
                tackler,
                success: escape,
            });
            path_success = path_success.and(escape);
        }
        if let Some(goblin) = sketch.goblin_mut(id) {
            goblin.presence = Presence::Standing(to);
        }
        if sketch.ball == (Ball::Loose { at: to }) {
            let malus = adjacent_opponent_count(&sketch, to, team);
            let pick = ball_roll_success(agility, malus, ruleset.ball_roll_target);
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
            .map(|action| action_odds(ruleset, &sketch, id, action))
    };
    let action_success = match &action {
        None | Some(ActionOdds::Stance) => Probability::CERTAIN,
        Some(ActionOdds::Assault { success, .. } | ActionOdds::Throw { success, .. }) => *success,
    };
    Ok(ActivationOdds {
        risky_steps,
        path_success,
        scores_on_the_way,
        action,
        overall_success: path_success.and(action_success),
    })
}

fn action_odds(
    ruleset: &Ruleset,
    sketch: &GameState,
    actor: GoblinId,
    action: Action,
) -> ActionOdds {
    match action {
        Action::Stance => ActionOdds::Stance,
        Action::Assault { target } => ActionOdds::Assault {
            target,
            success: assault_success(ruleset, sketch, actor, target),
        },
        Action::Throw { receiver } => throw_odds(ruleset, sketch, actor, receiver),
    }
}

fn assault_success(
    ruleset: &Ruleset,
    sketch: &GameState,
    attacker: GoblinId,
    target: GoblinId,
) -> Probability {
    let attack = sketch
        .goblin(attacker)
        .map_or(1, |goblin| goblin.characteristics.attack);
    let Some(defender) = sketch.goblin(target) else {
        return Probability::IMPOSSIBLE;
    };
    let downed_malus = if matches!(defender.presence, Presence::Down(_)) {
        i16::from(ruleset.downed_defense_malus)
    } else {
        0
    };
    let modifier = i16::from(sketch.stance_bonus(target)) - downed_malus;
    contest_success(
        attack,
        defender.characteristics.defense,
        i8::try_from(modifier).unwrap_or(i8::MIN),
    )
}

fn throw_odds(
    ruleset: &Ruleset,
    sketch: &GameState,
    thrower: GoblinId,
    receiver: GoblinId,
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
    let distance_malus = thrower_square
        .orthogonal_distance(receiver_square)
        .saturating_sub(ruleset.throw_free_range);
    let throw = ball_roll_success(
        thrower_goblin.map_or(1, |goblin| goblin.characteristics.agility),
        adjacent_opponent_count(sketch, thrower_square, team).saturating_add(distance_malus),
        target,
    );
    let interceptions: Vec<(GoblinId, Probability)> =
        squares_crossed(thrower_square, receiver_square)
            .into_iter()
            .filter_map(|square| sketch.goblin_at(square))
            .filter(|goblin| goblin.is_standing() && goblin.team() != team)
            .map(|interceptor| {
                let malus = adjacent_opponent_count(
                    sketch,
                    interceptor.square().unwrap_or(thrower_square),
                    interceptor.team(),
                )
                .saturating_add(ruleset.interception_surprise_malus);
                (
                    interceptor.id,
                    ball_roll_success(interceptor.characteristics.agility, malus, target),
                )
            })
            .collect();
    let catch = ball_roll_success(
        receiver_goblin.map_or(1, |goblin| goblin.characteristics.agility),
        adjacent_opponent_count(sketch, receiver_square, team),
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

fn adjacent_opponent_count(state: &GameState, square: Square, team: TeamId) -> u8 {
    u8::try_from(state.standing_opponents_next_to(square, team).count()).unwrap_or(u8::MAX)
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
                tackler: id("A1"),
                success: Probability::new(5, 8)
            }]
        );
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
        let assault = order("H1", &[], Some(Action::Assault { target: id("A1") }));
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
        let pass = order("H1", &[], Some(Action::Throw { receiver: id("H2") }));
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
        let pass = order("H1", &[], Some(Action::Throw { receiver: id("H2") }));
        assert!(matches!(
            odds(scenario, &pass).unwrap().action,
            Some(ActionOdds::Throw { interceptions, .. })
                if interceptions == vec![(id("A1"), Probability::new(1, 2))]
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
            most_dangerous_tackler(game.state(), id("H1"), Square::new(5, 7)),
            Some((id("A2"), Probability::new(5, 8)))
        );
    }
}
