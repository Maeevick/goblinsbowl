//! Where a goblin can go with the steps it has left this turn, and the safest way to get there.

use std::cmp::Reverse;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::arena::Arena;
use crate::command::{Action, Activation};
use crate::odds::Probability;
use crate::preview::activation_odds;
use crate::roster::GoblinId;
use crate::ruleset::Ruleset;
use crate::square::Square;
use crate::state::GameState;
use crate::team::TeamId;

/// The safest known path to a destination and its chance to be completed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedPath {
    /// Where the path ends.
    pub destination: Square,
    /// Squares stepped into, in order.
    pub squares: Vec<Square>,
    /// Chance to complete it without a knockdown or a fumble.
    pub success: Probability,
}

/// The safest path to every square `goblin` can reach with its steps left, sorted by destination.
///
/// Among paths to the same square the highest chance wins, then the fewest steps, then the
/// smallest squares in order. A carrier's path stops on the first enemy den square, where it scores.
pub fn reachable_squares(
    arena: &Arena,
    ruleset: &Ruleset,
    state: &GameState,
    goblin: GoblinId,
) -> Vec<PlannedPath> {
    let Some(mover) = state.goblin(goblin) else {
        return Vec::new();
    };
    let Some(start) = mover.square() else {
        return Vec::new();
    };
    let team = mover.team();
    let carries_the_ball = state.carrier() == Some(goblin);
    let mut candidates = Vec::new();
    let mut prefix = Vec::new();
    collect_paths(
        &PathSearch {
            arena,
            state,
            start,
            steps_left: usize::from(mover.steps_left),
            scoring_stops_the_walk: carries_the_ball,
            team,
        },
        &mut prefix,
        &mut candidates,
    );
    let mut best: BTreeMap<Square, PlannedPath> = BTreeMap::new();
    for squares in candidates {
        let Some(destination) = squares.last().copied() else {
            continue;
        };
        let order = Activation {
            goblin,
            path: squares.clone(),
            action: None,
        };
        let Ok(odds) = activation_odds(arena, ruleset, state, team, &order) else {
            continue;
        };
        let candidate = PlannedPath {
            destination,
            squares,
            success: odds.overall_success,
        };
        let keeps_current = best
            .get(&destination)
            .is_some_and(|current| ranking(current) >= ranking(&candidate));
        if !keeps_current {
            best.insert(destination, candidate);
        }
    }
    best.into_values().collect()
}

/// The best chance on `state` of an order for `goblin` with the same purpose: the same assault or
/// throw along any legal path, or else a walk ending on `destination`; none when no such order is legal.
pub fn best_chance_for(
    arena: &Arena,
    ruleset: &Ruleset,
    state: &GameState,
    team: TeamId,
    goblin: GoblinId,
    destination: Square,
    action: Option<Action>,
) -> Option<Probability> {
    let start = state.goblin(goblin)?.square()?;
    let same_purpose = |path: &Vec<Square>| match action {
        Some(Action::Assault { .. } | Action::Throw { .. }) => true,
        Some(Action::Stance) | None => path.last().copied().unwrap_or(start) == destination,
    };
    std::iter::once(Vec::new())
        .chain(
            reachable_squares(arena, ruleset, state, goblin)
                .into_iter()
                .map(|planned| planned.squares),
        )
        .filter(same_purpose)
        .filter_map(|path| {
            activation_odds(
                arena,
                ruleset,
                state,
                team,
                &Activation {
                    goblin,
                    path,
                    action,
                },
            )
            .ok()
            .map(|odds| odds.overall_success)
        })
        .max()
}

struct PathSearch<'a> {
    arena: &'a Arena,
    state: &'a GameState,
    start: Square,
    steps_left: usize,
    scoring_stops_the_walk: bool,
    team: TeamId,
}

fn collect_paths(search: &PathSearch<'_>, prefix: &mut Vec<Square>, found: &mut Vec<Vec<Square>>) {
    if prefix.len() == search.steps_left {
        return;
    }
    let here = prefix.last().copied().unwrap_or(search.start);
    for next in search.arena.in_bounds_neighbours(here) {
        let already_visited = next == search.start || prefix.contains(&next);
        if already_visited || search.state.goblin_at(next).is_some() {
            continue;
        }
        prefix.push(next);
        found.push(prefix.clone());
        let walk_ends_here =
            search.scoring_stops_the_walk && search.arena.is_scoring_square_for(search.team, next);
        if !walk_ends_here {
            collect_paths(search, prefix, found);
        }
        prefix.pop();
    }
}

fn ranking(path: &PlannedPath) -> (Probability, Reverse<usize>, Reverse<Vec<Square>>) {
    (
        path.success,
        Reverse(path.squares.len()),
        Reverse(path.squares.clone()),
    )
}

/// The safest path from `goblin`'s square to `destination`, if its steps left get it there.
pub fn safest_path(
    arena: &Arena,
    ruleset: &Ruleset,
    state: &GameState,
    goblin: GoblinId,
    destination: Square,
) -> Option<PlannedPath> {
    reachable_squares(arena, ruleset, state, goblin)
        .into_iter()
        .find(|path| path.destination == destination)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::characteristics::Archetype::{Bruiser, Sprinter};
    use crate::game::scenario::Scenario;

    fn id(text: &str) -> GoblinId {
        text.parse().unwrap()
    }

    #[test]
    fn given_a_free_walk_when_asking_its_best_chance_then_certain() {
        let game = Scenario::new()
            .goblin("H5", Sprinter, 5, 7)
            .ball_loose_at(1, 10)
            .game([]);
        assert_eq!(
            best_chance_for(
                game.arena(),
                game.ruleset(),
                game.state(),
                TeamId::HOME,
                id("H5"),
                Square::new(7, 7),
                None
            ),
            Some(Probability::CERTAIN)
        );
    }

    #[test]
    fn given_a_standing_bruiser_when_asking_the_best_chance_of_assaulting_it_then_below_certain() {
        let game = Scenario::new()
            .goblin("H1", Bruiser, 5, 7)
            .goblin("A1", Bruiser, 6, 7)
            .ball_loose_at(1, 10)
            .game([]);
        assert_eq!(
            best_chance_for(
                game.arena(),
                game.ruleset(),
                game.state(),
                TeamId::HOME,
                id("H1"),
                Square::new(5, 7),
                Some(Action::assault(id("A1")))
            ),
            Some(Probability::new(5, 8))
        );
    }

    fn reachable(scenario: Scenario, goblin: &str) -> Vec<PlannedPath> {
        let game = scenario.game([]);
        reachable_squares(game.arena(), game.ruleset(), game.state(), id(goblin))
    }

    fn safest(scenario: Scenario, goblin: &str, x: u8, y: u8) -> Option<PlannedPath> {
        let game = scenario.game([]);
        safest_path(
            game.arena(),
            game.ruleset(),
            game.state(),
            id(goblin),
            Square::new(x, y),
        )
    }

    #[test]
    fn given_a_sprinter_alone_in_the_middle_when_listing_destinations_then_sixty_squares() {
        let scenario = Scenario::new()
            .goblin("H1", Sprinter, 7, 7)
            .ball_loose_at(0, 3);
        assert_eq!(reachable(scenario, "H1").len(), 60);
    }

    #[test]
    fn given_a_sprinter_with_two_steps_left_when_listing_destinations_then_twelve_squares() {
        let scenario = Scenario::new()
            .goblin("H1", Sprinter, 7, 7)
            .edit("H1", |goblin| goblin.steps_left = 2)
            .ball_loose_at(0, 3);
        assert_eq!(reachable(scenario, "H1").len(), 12);
    }

    #[test]
    fn given_a_sprinter_alone_when_going_three_squares_east_then_a_certain_three_step_path() {
        let scenario = Scenario::new()
            .goblin("H1", Sprinter, 7, 7)
            .ball_loose_at(0, 3);
        assert_eq!(
            safest(scenario, "H1", 10, 7),
            Some(PlannedPath {
                destination: Square::new(10, 7),
                squares: vec![Square::new(8, 7), Square::new(9, 7), Square::new(10, 7)],
                success: Probability::CERTAIN
            })
        );
    }

    #[test]
    fn given_a_destination_beyond_the_movement_when_planning_then_none() {
        let scenario = Scenario::new()
            .goblin("H1", Bruiser, 7, 7)
            .ball_loose_at(0, 3);
        assert_eq!(safest(scenario, "H1", 11, 7), None);
    }

    #[test]
    fn given_an_occupied_destination_when_planning_then_none() {
        let scenario = Scenario::new()
            .goblin("H1", Sprinter, 7, 7)
            .goblin("H2", Sprinter, 9, 7)
            .ball_loose_at(0, 3);
        assert_eq!(safest(scenario, "H1", 9, 7), None);
    }

    #[test]
    fn given_the_ball_on_the_straight_route_when_walking_past_then_the_path_avoids_the_pick() {
        let scenario = Scenario::new()
            .goblin("H1", Sprinter, 7, 7)
            .ball_loose_at(8, 7);
        let path = safest(scenario, "H1", 9, 7).unwrap();
        assert_eq!(path.success, Probability::CERTAIN);
    }

    #[test]
    fn given_an_enemy_next_to_the_start_when_planning_then_the_first_step_carries_the_dodge() {
        let scenario = Scenario::new()
            .goblin("H1", Sprinter, 7, 7)
            .goblin("A1", Bruiser, 8, 7)
            .ball_loose_at(0, 3);
        let path = safest(scenario, "H1", 7, 5).unwrap();
        assert_eq!(path.success, Probability::new(5, 8));
    }

    #[test]
    fn given_a_carrier_next_to_the_enemy_den_when_listing_destinations_then_nothing_beyond_the_den()
    {
        let scenario = Scenario::new()
            .goblin("H1", Sprinter, 13, 7)
            .ball_carried_by("H1");
        assert!(reachable(scenario, "H1").iter().all(|path| {
            !path.squares[..path.squares.len() - 1]
                .iter()
                .any(|square| square.x == 14)
        }));
    }
}
