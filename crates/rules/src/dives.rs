//! Diving catches: who dives for a ball landing on an empty square, and how likely each Sprinter
//! around is to be the one diving, and to catch it or miss it.

use std::collections::HashMap;

use crate::characteristics::Archetype;
use crate::roster::GoblinId;
use crate::ruleset::Ruleset;
use crate::square::{Reach, Square};
use crate::state::GameState;

/// The standing Sprinters among the eight squares around `at` that dive for a ball landing there,
/// in the bowl's order, where the rules have diving catches.
pub(crate) fn divers_around(ruleset: &Ruleset, state: &GameState, at: Square) -> Vec<GoblinId> {
    if !ruleset.sprinters_dive_for_loose_balls {
        return Vec::new();
    }
    state
        .goblins
        .iter()
        .filter(|goblin| {
            goblin.archetype == Archetype::Sprinter
                && goblin.is_standing()
                && goblin
                    .square()
                    .is_some_and(|square| square.is_within(at, Reach::AllAround))
        })
        .map(|goblin| goblin.id)
        .collect()
}

/// A Sprinter's share of a dive: the chance it is the one diving and catches the ball, and the
/// chance it is the one diving and misses it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DiveChance {
    /// The Sprinter.
    pub(crate) diver: GoblinId,
    /// Chance it dives and catches the ball.
    pub(crate) catches: f64,
    /// Chance it dives and misses the ball.
    pub(crate) misses: f64,
}

/// How a ball landing on the empty square `at` is dived for: each Sprinter around rolls its
/// diving catch, the highest result dives and catches on the target, the tied ones rolling again
/// among themselves only to tell who dives.
pub(crate) fn dive_chances(ruleset: &Ruleset, state: &GameState, at: Square) -> Vec<DiveChance> {
    let divers: Vec<Diver> = divers_around(ruleset, state, at)
        .into_iter()
        .filter_map(|id| Diver::of(ruleset, state, id))
        .collect();
    let everyone = (1_u32 << divers.len()) - 1;
    let target = i16::from(ruleset.ball_roll_target);
    let mut contest = Contest {
        divers: &divers,
        tie_breaks: HashMap::new(),
    };
    (0..divers.len())
        .map(|index| {
            let (catches, misses) =
                divers[index]
                    .results()
                    .fold((0.0, 0.0), |(catches, misses), result| {
                        let (settled, all_tied) = contest.outcome_with(index, result, everyone);
                        let dives = divers[index].chance_of(result)
                            * (settled + all_tied * contest.tie_break(index, everyone));
                        if result >= target {
                            (catches + dives, misses)
                        } else {
                            (catches, misses + dives)
                        }
                    });
            DiveChance {
                diver: divers[index].id,
                catches,
                misses,
            }
        })
        .collect()
}

/// A diver's die: its Agility, less the diving malus and the standing enemies around it.
struct Diver {
    id: GoblinId,
    faces: u8,
    malus: u8,
}

impl Diver {
    fn of(ruleset: &Ruleset, state: &GameState, id: GoblinId) -> Option<Self> {
        let goblin = state.goblin(id)?;
        let square = goblin.square()?;
        let enemies = state
            .standing_opponents_in_reach(square, goblin.team(), ruleset.reach())
            .count();
        Some(Self {
            id,
            faces: goblin.characteristics.agility.max(1),
            malus: ruleset
                .diving_catch_malus
                .saturating_add(u8::try_from(enemies).unwrap_or(u8::MAX)),
        })
    }

    /// Every result its die can show: a face less the malus.
    fn results(&self) -> impl Iterator<Item = i16> + '_ {
        (1..=self.faces).map(|face| i16::from(face) - i16::from(self.malus))
    }

    fn chance_of(&self, result: i16) -> f64 {
        if self.results().any(|shown| shown == result) {
            1.0 / f64::from(self.faces)
        } else {
            0.0
        }
    }

    fn chance_below(&self, result: i16) -> f64 {
        let below = self.results().filter(|shown| *shown < result).count();
        f64::from(u8::try_from(below).unwrap_or(u8::MAX)) / f64::from(self.faces)
    }
}

/// Who dives among several Sprinters rolling for the same ball.
struct Contest<'a> {
    divers: &'a [Diver],
    tie_breaks: HashMap<(usize, u32), f64>,
}

impl Contest<'_> {
    /// With diver `index` showing `result` among the divers of `rolling`: the chance it dives
    /// once the others' rolls are in, ties with only some of them settled by their rolling again,
    /// and the chance every other one tied with it.
    fn outcome_with(&mut self, index: usize, result: i16, rolling: u32) -> (f64, f64) {
        let divers = self.divers;
        let others: Vec<usize> = members(rolling).filter(|other| *other != index).collect();
        let all_of_them = (1_u32 << others.len()) - 1;
        let mut settled = 0.0;
        let mut all_tied = 0.0;
        for tied_with in 0..=all_of_them {
            let mut share = 1.0;
            let mut tied = 1_u32 << index;
            for (bit, other) in others.iter().enumerate() {
                if tied_with & (1 << bit) == 0 {
                    share *= divers[*other].chance_below(result);
                } else {
                    share *= divers[*other].chance_of(result);
                    tied |= 1 << *other;
                }
            }
            if share == 0.0 {
                continue;
            }
            if tied_with == all_of_them && !others.is_empty() {
                all_tied += share;
            } else if tied == 1 << index {
                settled += share;
            } else {
                settled += share * self.tie_break(index, tied);
            }
        }
        (settled, all_tied)
    }

    /// Chance diver `index` comes out on top when the divers of `tied` roll again among
    /// themselves, as many times as it takes; when they can never stand apart, the first of them
    /// in the bowl's order dives, as in a match.
    fn tie_break(&mut self, index: usize, tied: u32) -> f64 {
        if let Some(chance) = self.tie_breaks.get(&(index, tied)) {
            return *chance;
        }
        let diver = &self.divers[index];
        let (settled, all_tied) = diver.results().collect::<Vec<_>>().into_iter().fold(
            (0.0, 0.0),
            |(settled, all_tied), result| {
                let chance = diver.chance_of(result);
                let (settles, ties) = self.outcome_with(index, result, tied);
                (settled + chance * settles, all_tied + chance * ties)
            },
        );
        let chance = if all_tied < 1.0 {
            settled / (1.0 - all_tied)
        } else if members(tied).next() == Some(index) {
            1.0
        } else {
            0.0
        };
        self.tie_breaks.insert((index, tied), chance);
        chance
    }
}

fn members(divers: u32) -> impl Iterator<Item = usize> {
    (0..32).filter(move |index| divers & (1 << index) != 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::characteristics::Archetype::{Bruiser, Sprinter};
    use crate::game::scenario::Scenario;

    fn id(text: &str) -> GoblinId {
        text.parse().unwrap()
    }

    fn landing() -> Square {
        Square::new(6, 6)
    }

    fn diving_with_a_malus_of_one(scenario: Scenario) -> Vec<DiveChance> {
        let game = scenario
            .rules(|ruleset| {
                ruleset.sprinters_dive_for_loose_balls = true;
                ruleset.diving_catch_malus = 1;
            })
            .game([]);
        dive_chances(game.ruleset(), game.state(), landing())
    }

    fn close_to(chance: f64, expected: f64) -> bool {
        (chance - expected).abs() < 1e-12
    }

    #[test]
    fn given_a_lone_sprinter_with_a_d4_next_to_the_landing_ball_when_weighing_its_dive_then_it_catches_half_the_time()
     {
        let dives = diving_with_a_malus_of_one(Scenario::new().goblin("H1", Sprinter, 7, 5));

        assert!(matches!(
            dives.as_slice(),
            [DiveChance { diver, catches, misses }]
                if *diver == id("H1") && close_to(*catches, 0.5) && close_to(*misses, 0.5)
        ));
    }

    #[test]
    fn given_two_alike_sprinters_next_to_the_landing_ball_when_weighing_their_dives_then_each_catches_three_times_in_eight()
     {
        let dives = diving_with_a_malus_of_one(
            Scenario::new()
                .goblin("H1", Sprinter, 7, 5)
                .goblin("H2", Sprinter, 5, 7),
        );

        assert!(
            dives
                .iter()
                .all(|dive| close_to(dive.catches, 3.0 / 8.0) && close_to(dive.misses, 1.0 / 8.0))
        );
    }

    #[test]
    fn given_a_sprinter_marked_by_a_bruiser_next_to_a_free_one_when_weighing_their_dives_then_the_free_one_dives_more_and_every_dive_adds_up_to_certain()
     {
        let dives = diving_with_a_malus_of_one(
            Scenario::new()
                .goblin("H1", Sprinter, 7, 5)
                .goblin("H2", Bruiser, 4, 7)
                .goblin("A1", Sprinter, 5, 7),
        );
        let share = |diver: &str| {
            dives
                .iter()
                .find(|dive| dive.diver == id(diver))
                .map(|dive| dive.catches + dive.misses)
                .unwrap_or_default()
        };

        assert!(
            share("H1") > share("A1")
                && close_to(
                    dives.iter().map(|dive| dive.catches + dive.misses).sum(),
                    1.0
                )
        );
    }

    #[test]
    fn given_a_bruiser_next_to_the_landing_ball_when_listing_its_divers_then_nobody() {
        let game = Scenario::new()
            .rules(|ruleset| ruleset.sprinters_dive_for_loose_balls = true)
            .goblin("H1", Bruiser, 7, 5)
            .game([]);

        assert_eq!(divers_around(game.ruleset(), game.state(), landing()), []);
    }
}
