//! A coach that picks legal commands at random: the sparring partner of property tests.

use crate::arena::Arena;
use crate::command::{Action, Activation, Command};
use crate::dice::DiceRoller;
use crate::paths::reachable_squares;
use crate::ruleset::Ruleset;
use crate::state::GameState;
use crate::team::TeamId;
use crate::validation::validate_activation;

/// A random legal command for `team`, drawn with `dice`.
///
/// It ends the turn one time in six or when no goblin can act; otherwise it activates a random
/// goblin along the safest path to a random reachable square (or in place) with a random legal action.
pub fn random_command(
    arena: &Arena,
    ruleset: &Ruleset,
    state: &GameState,
    team: TeamId,
    dice: &mut impl DiceRoller,
) -> Command {
    let movers: Vec<_> = state
        .goblins_of(team)
        .filter(|goblin| goblin.can_act())
        .map(|goblin| goblin.id)
        .collect();
    if movers.is_empty() || dice.roll(6) == 1 {
        return Command::EndTurn;
    }
    let mover = movers[pick_index(dice, movers.len())];
    let destinations = reachable_squares(arena, ruleset, state, mover);
    let choice = pick_index(dice, destinations.len() + 1);
    let path = destinations
        .get(choice)
        .map(|planned| planned.squares.clone())
        .unwrap_or_default();
    let mut actions = vec![None, Some(Action::Stance)];
    actions.extend(
        state
            .goblins
            .iter()
            .filter(|other| other.team() != team)
            .map(|other| Some(Action::Assault { target: other.id })),
    );
    actions.extend(
        state
            .goblins_of(team)
            .filter(|teammate| teammate.id != mover)
            .map(|teammate| {
                Some(Action::Throw {
                    receiver: teammate.id,
                })
            }),
    );
    let legal_actions: Vec<_> = actions
        .into_iter()
        .filter(|action| {
            let candidate = Activation {
                goblin: mover,
                path: path.clone(),
                action: *action,
            };
            validate_activation(arena, ruleset, state, team, &candidate).is_ok()
        })
        .collect();
    let action = legal_actions
        .get(pick_index(dice, legal_actions.len()))
        .copied()
        .flatten();
    Command::Act(Activation {
        goblin: mover,
        path,
        action,
    })
}

fn pick_index(dice: &mut impl DiceRoller, count: usize) -> usize {
    let faces = u8::try_from(count.clamp(1, usize::from(u8::MAX))).unwrap_or(u8::MAX);
    usize::from(dice.roll(faces)) - 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dice::SeededDice;
    use crate::game::Game;
    use crate::roster::Roster;
    use crate::validation::validate_activation;

    #[test]
    fn given_many_draws_at_kick_off_when_choosing_then_every_command_is_legal() {
        let (game, _) = Game::start(
            Ruleset::default(),
            [Roster::DEFAULT, Roster::DEFAULT],
            SeededDice::from_seed(5),
        )
        .unwrap();
        let team = game.state().active_team().unwrap();
        let mut dice = SeededDice::from_seed(11);
        let all_legal = (0..200).all(|_| {
            match random_command(game.arena(), game.ruleset(), game.state(), team, &mut dice) {
                Command::Act(activation) => validate_activation(
                    game.arena(),
                    game.ruleset(),
                    game.state(),
                    team,
                    &activation,
                )
                .is_ok(),
                Command::EndTurn | Command::TimeUp => true,
            }
        });
        assert!(all_legal);
    }

    #[test]
    fn given_many_draws_at_kick_off_when_choosing_then_some_goblins_are_activated() {
        let (game, _) = Game::start(
            Ruleset::default(),
            [Roster::DEFAULT, Roster::DEFAULT],
            SeededDice::from_seed(5),
        )
        .unwrap();
        let team = game.state().active_team().unwrap();
        let mut dice = SeededDice::from_seed(11);
        let activations = (0..50)
            .filter(|_| {
                matches!(
                    random_command(game.arena(), game.ruleset(), game.state(), team, &mut dice),
                    Command::Act(_)
                )
            })
            .count();
        assert!(activations > 25);
    }
}
