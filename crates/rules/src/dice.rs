//! The dice port: every random number of a match comes through a [`DiceRoller`].

use std::collections::VecDeque;

use rand_chacha::ChaCha8Rng;
use rand_core::{Rng, SeedableRng};

/// Source of die results for the rules engine.
pub trait DiceRoller {
    /// Rolls a die with `faces` faces and returns a value between 1 and `faces`
    /// (always 1 for a die with zero or one face).
    fn roll(&mut self, faces: u8) -> u8;
}

/// Reproducible dice: a ChaCha8 stream built from a 64-bit seed, faces drawn by rejection sampling
/// so the sequence depends only on the seed, never on a random-range algorithm.
#[derive(Clone, Debug)]
pub struct SeededDice {
    generator: ChaCha8Rng,
    seed: u64,
}

impl SeededDice {
    /// Dice whose whole sequence is determined by `seed`.
    pub fn from_seed(seed: u64) -> Self {
        let mut seed_bytes = [0_u8; 32];
        seed_bytes[..8].copy_from_slice(&seed.to_le_bytes());
        Self {
            generator: ChaCha8Rng::from_seed(seed_bytes),
            seed,
        }
    }

    /// The seed these dice were built from, to be stored in match records.
    pub fn seed(&self) -> u64 {
        self.seed
    }
}

impl DiceRoller for SeededDice {
    fn roll(&mut self, faces: u8) -> u8 {
        if faces <= 1 {
            return 1;
        }
        let faces = u64::from(faces);
        let draw_range = 1_u64 << 32;
        let acceptance_limit = draw_range - draw_range % faces;
        loop {
            let draw = u64::from(self.generator.next_u32());
            if draw < acceptance_limit {
                return u8::try_from(draw % faces + 1).unwrap_or(u8::MAX);
            }
        }
    }
}

/// Test fake: hands out pre-written results in order and remembers which dice were asked for.
///
/// Panics when the script runs out or when a result cannot appear on the requested die,
/// so a test fails loudly if the rules roll differently than the test expects.
#[derive(Clone, Debug, Default)]
pub struct ScriptedDice {
    pending_results: VecDeque<u8>,
    faces_requested: Vec<u8>,
}

impl ScriptedDice {
    /// Dice that will return `results` one by one.
    pub fn new(results: impl IntoIterator<Item = u8>) -> Self {
        Self {
            pending_results: results.into_iter().collect(),
            faces_requested: Vec::new(),
        }
    }

    /// Number of scripted results not used yet.
    pub fn remaining(&self) -> usize {
        self.pending_results.len()
    }

    /// Faces of every die rolled so far, in order.
    pub fn faces_requested(&self) -> &[u8] {
        &self.faces_requested
    }
}

impl DiceRoller for ScriptedDice {
    fn roll(&mut self, faces: u8) -> u8 {
        let rolls_so_far = self.faces_requested.len();
        let result = self.pending_results.pop_front().unwrap_or_else(|| {
            panic!("scripted dice ran out after {rolls_so_far} rolls (asked for a d{faces})")
        });
        assert!(
            (1..=faces.max(1)).contains(&result),
            "a d{faces} cannot show {result} (scripted roll {})",
            rolls_so_far + 1
        );
        self.faces_requested.push(faces);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rolls(dice: &mut impl DiceRoller, faces: u8, count: usize) -> Vec<u8> {
        (0..count).map(|_| dice.roll(faces)).collect()
    }

    #[test]
    fn given_same_seed_when_rolling_twice_then_identical_sequences() {
        let first = rolls(&mut SeededDice::from_seed(7), 6, 50);
        assert_eq!(rolls(&mut SeededDice::from_seed(7), 6, 50), first);
    }

    #[test]
    fn given_different_seeds_when_rolling_then_sequences_differ() {
        let first = rolls(&mut SeededDice::from_seed(7), 6, 50);
        assert_ne!(rolls(&mut SeededDice::from_seed(8), 6, 50), first);
    }

    #[test]
    fn given_d4_when_rolling_many_times_then_every_face_shows_and_nothing_else() {
        let mut faces_seen = rolls(&mut SeededDice::from_seed(3), 4, 400);
        faces_seen.sort_unstable();
        faces_seen.dedup();
        assert_eq!(faces_seen, vec![1, 2, 3, 4]);
    }

    #[test]
    fn given_one_faced_die_when_rolled_then_one() {
        assert_eq!(SeededDice::from_seed(1).roll(1), 1);
    }

    #[test]
    fn given_zero_faced_die_when_rolled_then_one() {
        assert_eq!(SeededDice::from_seed(1).roll(0), 1);
    }

    #[test]
    fn given_seed_42_when_rolling_d6_then_the_pinned_golden_sequence() {
        assert_eq!(
            rolls(&mut SeededDice::from_seed(42), 6, 12),
            vec![4, 6, 2, 3, 5, 2, 1, 1, 1, 4, 2, 3]
        );
    }

    #[test]
    fn given_script_when_rolled_then_results_come_in_order() {
        let mut dice = ScriptedDice::new([3, 1, 2]);
        assert_eq!(rolls(&mut dice, 4, 3), vec![3, 1, 2]);
    }

    #[test]
    fn given_script_when_rolled_then_requested_faces_are_remembered() {
        let mut dice = ScriptedDice::new([3, 1]);
        dice.roll(4);
        dice.roll(2);
        assert_eq!(dice.faces_requested(), &[4, 2]);
    }

    #[test]
    #[should_panic(expected = "ran out")]
    fn given_exhausted_script_when_rolled_then_panics() {
        ScriptedDice::new([]).roll(6);
    }

    #[test]
    #[should_panic(expected = "cannot show")]
    fn given_result_too_high_for_the_die_when_rolled_then_panics() {
        ScriptedDice::new([5]).roll(4);
    }
}
