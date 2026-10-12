//! Pure validation of orders: what a coach may ask for, checked before any die is rolled.

use crate::arena::Arena;
use crate::command::{Action, Activation};
use crate::roster::GoblinId;
use crate::square::Square;
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
    if activation.action.is_some() && !goblin.has_action_left {
        return Err(RuleViolation::ActionAlreadyUsed { goblin: id });
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
        None | Some(Action::Stance) => Ok(()),
        Some(Action::Assault { target }) => validate_assault(state, team, final_square, target),
        Some(Action::Throw { receiver }) => {
            validate_throw(state, team, id, &activation.path, receiver)
        }
    }
}

fn validate_assault(
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
    if !attacker_square.is_orthogonally_adjacent(defender_square) {
        return Err(RuleViolation::TargetNotAdjacent { target });
    }
    Ok(())
}

fn validate_throw(
    state: &GameState,
    team: TeamId,
    thrower: GoblinId,
    path: &[Square],
    receiver: GoblinId,
) -> Result<(), RuleViolation> {
    let holds_the_ball = state.carrier() == Some(thrower);
    let picks_it_on_the_way = matches!(state.ball, Ball::Loose { at } if path.contains(&at));
    if !holds_the_ball && !picks_it_on_the_way {
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
                game.state(),
                TeamId::HOME,
                &order("H1", &[(2, 0)])
            ),
            Err(RuleViolation::PathLeavesTheBowl {
                square: Square::new(2, 0)
            })
        );
    }
}
