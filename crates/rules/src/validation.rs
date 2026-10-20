//! Pure validation of orders: what a coach may ask for, checked before any die is rolled.

use crate::arena::Arena;
use crate::characteristics::Archetype;
use crate::command::{Action, Activation, AssaultStyle, ThrowStyle};
use crate::roster::GoblinId;
use crate::ruleset::Ruleset;
use crate::square::{Compass, Square};
use crate::state::{Ball, GameState, Presence};
use crate::team::TeamId;
use crate::violation::RuleViolation;

/// Checks a whole activation against the rules before anything is resolved: the goblin, every step
/// of its path, then its action, assuming every roll along the way succeeds.
///
/// # Errors
/// The first [`RuleViolation`] met, in that order.
pub fn validate_activation(
    arena: &Arena,
    ruleset: &Ruleset,
    state: &GameState,
    team: TeamId,
    activation: &Activation,
) -> Result<(), RuleViolation> {
    let id = activation.goblin;
    let goblin = state
        .goblin(id)
        .ok_or(RuleViolation::UnknownGoblin { goblin: id })?;
    if goblin.team() != team {
        return Err(RuleViolation::NotYourGoblin { goblin: id });
    }
    let Presence::Standing(start) = goblin.presence else {
        return Err(RuleViolation::NotStanding { goblin: id });
    };
    if !goblin.can_act() {
        return Err(RuleViolation::AlreadyActed { goblin: id });
    }
    if activation.path.len() > usize::from(goblin.steps_left) {
        return Err(RuleViolation::PathTooLong {
            goblin: id,
            steps_left: goblin.steps_left,
            steps: activation.path.len(),
        });
    }
    if activation.action.is_some()
        && ruleset.actions_cost_a_step
        && activation.path.len() >= usize::from(goblin.steps_left)
    {
        return Err(RuleViolation::NoStepLeftToAct {
            goblin: id,
            steps_left: goblin.steps_left,
            steps: activation.path.len(),
        });
    }
    if let Some(action) = activation.action
        && !goblin.may_take(action, ruleset)
    {
        return Err(RuleViolation::ActionAlreadyUsed { goblin: id });
    }
    if let Some(Action::Assault { style, .. }) = activation.action
        && !style.is_simple()
        && !ruleset.assaults_have_styles
    {
        return Err(RuleViolation::StyleNotInTheseRules { goblin: id });
    }
    if let Some(Action::Assault {
        style: AssaultStyle::Brutal | AssaultStyle::Projection(_),
        ..
    }) = activation.action
        && ruleset.brutal_and_projection_need_a_bruiser
        && goblin.archetype != Archetype::Bruiser
    {
        return Err(RuleViolation::StyleNeedsABruiser { goblin: id });
    }
    if let Some(Action::Throw {
        style: ThrowStyle::Chaos,
        ..
    }) = activation.action
    {
        if !ruleset.throws_have_styles {
            return Err(RuleViolation::StyleNotInTheseRules { goblin: id });
        }
        if ruleset.only_throwers_throw_into_chaos && goblin.archetype != Archetype::Thrower {
            return Err(RuleViolation::OnlyThrowersThrowIntoChaos { goblin: id });
        }
        if !activation.path.is_empty() || goblin.steps_left < goblin.characteristics.movement {
            return Err(RuleViolation::MustStandStillFirst { goblin: id });
        }
    }
    if let Some(Action::Throw {
        style: ThrowStyle::Focused,
        ..
    }) = activation.action
    {
        if !ruleset.focused_throws_spend_steps_left {
            return Err(RuleViolation::StyleNotInTheseRules { goblin: id });
        }
        if goblin.archetype != Archetype::Thrower {
            return Err(RuleViolation::OnlyThrowersFocus { goblin: id });
        }
        if ruleset.focused_throw_bonus.is_some()
            && (!activation.path.is_empty() || goblin.steps_left < goblin.characteristics.movement)
        {
            return Err(RuleViolation::MustStandStillFirst { goblin: id });
        }
    }
    let mut visited = vec![start];
    for (index, square) in activation.path.iter().copied().enumerate() {
        let previous = visited.last().copied().unwrap_or(start);
        if !previous.is_orthogonally_adjacent(square) {
            return Err(RuleViolation::PathNotContiguous { index, square });
        }
        if !arena.is_in_bounds(square) {
            return Err(RuleViolation::PathLeavesTheBowl { square });
        }
        if visited.contains(&square) {
            return Err(RuleViolation::PathRevisitsSquare { square });
        }
        if state.goblin_at(square).is_some() {
            return Err(RuleViolation::PathBlocked { square });
        }
        visited.push(square);
    }
    let final_square = visited.last().copied().unwrap_or(start);
    match activation.action {
        None => Ok(()),
        Some(Action::Focus) if !ruleset.throws_have_styles => {
            Err(RuleViolation::StyleNotInTheseRules { goblin: id })
        }
        Some(Action::Focus) if ruleset.focused_throws_spend_steps_left => {
            Err(RuleViolation::FocusOnlyAsAThrow { goblin: id })
        }
        Some(Action::Focus)
            if !activation.path.is_empty()
                || goblin.steps_left < goblin.characteristics.movement =>
        {
            Err(RuleViolation::MustStandStillFirst { goblin: id })
        }
        Some(Action::Focus) if state.carrier() != Some(id) => {
            Err(RuleViolation::FocusNeedsTheBall { goblin: id })
        }
        Some(Action::Focus) => Ok(()),
        Some(Action::Stance)
            if ruleset.goblins_can_take_stance
                && !ruleset.carriers_can_brace
                && holds_the_ball_after(state, id, &activation.path) =>
        {
            Err(RuleViolation::CarrierCannotBrace { goblin: id })
        }
        Some(Action::Stance) if ruleset.goblins_can_take_stance => Ok(()),
        Some(Action::Stance) => Err(RuleViolation::NoStanceInTheseRules { goblin: id }),
        Some(Action::Assault { .. })
            if !ruleset.carriers_can_assault
                && holds_the_ball_after(state, id, &activation.path) =>
        {
            Err(RuleViolation::CarrierCannotAssault { goblin: id })
        }
        Some(Action::Assault { target, style }) => {
            validate_assault(ruleset, state, team, final_square, target)?;
            match style {
                AssaultStyle::Projection(direction) => {
                    validate_projection(arena, state, id, final_square, target, direction)
                }
                AssaultStyle::Simple | AssaultStyle::Chirurgical | AssaultStyle::Brutal => Ok(()),
            }
        }
        Some(Action::Throw { receiver, .. }) => {
            validate_throw(state, team, id, &activation.path, receiver)
        }
    }
}

/// Whether `goblin` holds the ball once it has walked `path`: it carries it already, or picks it
/// up on the way.
fn holds_the_ball_after(state: &GameState, goblin: GoblinId, path: &[Square]) -> bool {
    state.carrier() == Some(goblin)
        || matches!(state.ball, Ball::Loose { at } if path.contains(&at))
}

fn validate_assault(
    ruleset: &Ruleset,
    state: &GameState,
    team: TeamId,
    attacker_square: Square,
    target: GoblinId,
) -> Result<(), RuleViolation> {
    let defender = state
        .goblin(target)
        .ok_or(RuleViolation::UnknownGoblin { goblin: target })?;
    if defender.team() == team {
        return Err(RuleViolation::TargetNotAnOpponent { target });
    }
    let Some(defender_square) = defender.square() else {
        return Err(RuleViolation::TargetNotOnTheBowl { target });
    };
    if !attacker_square.is_within(defender_square, ruleset.assault_reach()) {
        return Err(RuleViolation::TargetNotAdjacent { target });
    }
    Ok(())
}

fn validate_projection(
    arena: &Arena,
    state: &GameState,
    attacker: GoblinId,
    attacker_square: Square,
    target: GoblinId,
    direction: Compass,
) -> Result<(), RuleViolation> {
    if !state
        .goblin(target)
        .is_some_and(|goblin| goblin.is_standing())
    {
        return Err(RuleViolation::ProjectionNeedsAStandingTarget { target });
    }
    projection_landing(arena, state, attacker, attacker_square, target, direction)
        .map(|_| ())
        .ok_or(RuleViolation::ProjectionLandingUnavailable { target })
}

/// The square `target` falls on when `attacker`, standing on `attacker_square`, projects it in
/// `direction`: on the bowl, and free or already its own.
pub fn projection_landing(
    arena: &Arena,
    state: &GameState,
    attacker: GoblinId,
    attacker_square: Square,
    target: GoblinId,
    direction: Compass,
) -> Option<Square> {
    let landing = direction.step_from(attacker_square)?;
    let free = state
        .goblin_at(landing)
        .is_none_or(|goblin| goblin.id == target || goblin.id == attacker);
    (arena.is_in_bounds(landing) && free).then_some(landing)
}

fn validate_throw(
    state: &GameState,
    team: TeamId,
    thrower: GoblinId,
    path: &[Square],
    receiver: GoblinId,
) -> Result<(), RuleViolation> {
    if !holds_the_ball_after(state, thrower, path) {
        return Err(RuleViolation::ThrowWithoutBall { goblin: thrower });
    }
    let can_receive = state.goblin(receiver).is_some_and(|goblin| {
        goblin.team() == team && goblin.id != thrower && goblin.is_standing()
    });
    if !can_receive {
        return Err(RuleViolation::ReceiverUnavailable { receiver });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::characteristics::Archetype::Sprinter;
    use crate::game::scenario::Scenario;

    fn order(goblin: &str, path: &[(u8, u8)]) -> Activation {
        Activation {
            goblin: goblin.parse().unwrap(),
            path: path.iter().map(|(x, y)| Square::new(*x, *y)).collect(),
            action: None,
        }
    }

    #[test]
    fn given_a_legal_order_when_validated_on_its_own_then_accepted() {
        let game = Scenario::new().goblin("H1", Sprinter, 3, 7).game([]);
        assert_eq!(
            validate_activation(
                game.arena(),
                game.ruleset(),
                game.state(),
                TeamId::HOME,
                &order("H1", &[(4, 7)])
            ),
            Ok(())
        );
    }

    #[test]
    fn given_an_order_leaving_the_bowl_when_validated_on_its_own_then_refused() {
        let game = Scenario::new().goblin("H1", Sprinter, 3, 0).game([]);
        assert_eq!(
            validate_activation(
                game.arena(),
                game.ruleset(),
                game.state(),
                TeamId::HOME,
                &order("H1", &[(2, 0)])
            ),
            Err(RuleViolation::PathLeavesTheBowl {
                square: Square::new(2, 0)
            })
        );
    }

    #[test]
    fn given_carriers_that_cannot_assault_when_the_carrier_assaults_then_refused() {
        let game = Scenario::new()
            .rules(|ruleset| ruleset.carriers_can_assault = false)
            .goblin("H1", Sprinter, 3, 7)
            .goblin("A1", Sprinter, 4, 7)
            .ball_carried_by("H1")
            .game([]);
        let assault = Activation {
            action: Some(Action::assault("A1".parse().unwrap())),
            ..order("H1", &[])
        };
        assert_eq!(
            validate_activation(
                game.arena(),
                game.ruleset(),
                game.state(),
                TeamId::HOME,
                &assault
            ),
            Err(RuleViolation::CarrierCannotAssault {
                goblin: "H1".parse().unwrap()
            })
        );
    }

    #[test]
    fn given_carriers_that_cannot_assault_when_a_goblin_picks_the_ball_up_on_its_way_to_assault_then_refused()
     {
        let game = Scenario::new()
            .rules(|ruleset| ruleset.carriers_can_assault = false)
            .goblin("H1", Sprinter, 3, 7)
            .goblin("A1", Sprinter, 5, 7)
            .ball_loose_at(4, 7)
            .game([]);
        let assault = Activation {
            action: Some(Action::assault("A1".parse().unwrap())),
            ..order("H1", &[(4, 7)])
        };
        assert_eq!(
            validate_activation(
                game.arena(),
                game.ruleset(),
                game.state(),
                TeamId::HOME,
                &assault
            ),
            Err(RuleViolation::CarrierCannotAssault {
                goblin: "H1".parse().unwrap()
            })
        );
    }

    #[test]
    fn given_carriers_that_cannot_brace_when_the_carrier_braces_then_refused() {
        let game = Scenario::new()
            .rules(|ruleset| ruleset.carriers_can_brace = false)
            .goblin("H1", Sprinter, 3, 7)
            .ball_carried_by("H1")
            .game([]);
        let brace = Activation {
            action: Some(Action::Stance),
            ..order("H1", &[])
        };
        assert_eq!(
            validate_activation(
                game.arena(),
                game.ruleset(),
                game.state(),
                TeamId::HOME,
                &brace
            ),
            Err(RuleViolation::CarrierCannotBrace {
                goblin: "H1".parse().unwrap()
            })
        );
    }

    #[test]
    fn given_carriers_that_cannot_brace_when_a_goblin_picks_the_ball_up_on_its_way_to_brace_then_refused()
     {
        let game = Scenario::new()
            .rules(|ruleset| ruleset.carriers_can_brace = false)
            .goblin("H1", Sprinter, 3, 7)
            .ball_loose_at(4, 7)
            .game([]);
        let brace = Activation {
            action: Some(Action::Stance),
            ..order("H1", &[(4, 7)])
        };
        assert_eq!(
            validate_activation(
                game.arena(),
                game.ruleset(),
                game.state(),
                TeamId::HOME,
                &brace
            ),
            Err(RuleViolation::CarrierCannotBrace {
                goblin: "H1".parse().unwrap()
            })
        );
    }

    #[test]
    fn given_rules_without_stance_when_a_goblin_braces_then_refused() {
        let game = Scenario::new()
            .rules(|ruleset| ruleset.goblins_can_take_stance = false)
            .goblin("H1", Sprinter, 3, 7)
            .game([]);
        let brace = Activation {
            action: Some(Action::Stance),
            ..order("H1", &[])
        };
        assert_eq!(
            validate_activation(
                game.arena(),
                game.ruleset(),
                game.state(),
                TeamId::HOME,
                &brace
            ),
            Err(RuleViolation::NoStanceInTheseRules {
                goblin: "H1".parse().unwrap()
            })
        );
    }
}
