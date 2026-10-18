//! Exact odds of every roll, as fractions, so the same numbers come out on every machine.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

/// Which dice face off in a contest: the actor's die plus a modifier against the opponent's die
/// plus a modifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContestDice {
    /// Faces of the actor's die.
    pub actor_faces: u8,
    /// Added to the actor's face: +1 for a chirurgical assault, −1 for a brutal one.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub actor_modifier: i8,
    /// Faces of the opponent's die.
    pub opponent_faces: u8,
    /// Added to the opponent's face.
    pub opponent_modifier: i8,
}

impl ContestDice {
    /// Chance that the actor wins (ties go to the actor).
    pub fn success(self) -> Probability {
        contest_share(self, |margin| margin >= 0)
    }

    /// Chance that the actor beats the opponent outright, a tie left out.
    pub fn clear_win(self) -> Probability {
        contest_share(self, |margin| margin > 0)
    }

    /// Chance that both totals tie.
    pub fn tie(self) -> Probability {
        contest_share(self, |margin| margin == 0)
    }
}

/// Whether a modifier adds nothing, so it can be left out of a record.
pub fn is_zero(modifier: &i8) -> bool {
    *modifier == 0
}

fn contest_share(dice: ContestDice, counts: impl Fn(i16) -> bool) -> Probability {
    let actor_faces = dice.actor_faces.max(1);
    let opponent_faces = dice.opponent_faces.max(1);
    let counted_pairs = (1..=actor_faces)
        .flat_map(|actor| (1..=opponent_faces).map(move |opponent| (actor, opponent)))
        .filter(|(actor, opponent)| {
            counts(
                i16::from(*actor) + i16::from(dice.actor_modifier)
                    - (i16::from(*opponent) + i16::from(dice.opponent_modifier)),
            )
        })
        .count();
    Probability::new(
        u64::try_from(counted_pairs).unwrap_or(u64::MAX),
        u64::from(actor_faces) * u64::from(opponent_faces),
    )
}

/// The die of an agility test: the goblin's Agility die minus a malus plus a bonus, against a
/// target.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgilityDice {
    /// Faces of the goblin's Agility die.
    pub faces: u8,
    /// Subtracted from the face.
    pub malus: u8,
    /// Added to the face: +1 to the throw of a goblin that focused on its last turn.
    #[serde(default, skip_serializing_if = "is_zero_bonus")]
    pub bonus: u8,
    /// Value to reach after the malus and the bonus.
    pub target: u8,
}

impl AgilityDice {
    /// Chance that the face minus the malus plus the bonus reaches the target.
    pub fn success(self) -> Probability {
        agility_success(
            self.faces,
            self.malus,
            self.target.saturating_sub(self.bonus),
        )
    }
}

/// Whether a bonus adds nothing, so it can be left out of a record.
pub fn is_zero_bonus(bonus: &u8) -> bool {
    *bonus == 0
}

/// A probability kept as a reduced fraction; comparisons never go through floating point.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Probability {
    numerator: u64,
    denominator: u64,
}

impl Probability {
    /// Certain success.
    pub const CERTAIN: Probability = Probability {
        numerator: 1,
        denominator: 1,
    };

    /// Certain failure.
    pub const IMPOSSIBLE: Probability = Probability {
        numerator: 0,
        denominator: 1,
    };

    /// The fraction `numerator / denominator`, reduced; a zero denominator means impossible
    /// and a numerator above the denominator is capped at certain.
    pub fn new(numerator: u64, denominator: u64) -> Self {
        if denominator == 0 || numerator == 0 {
            return Probability::IMPOSSIBLE;
        }
        if numerator >= denominator {
            return Probability::CERTAIN;
        }
        let divisor = greatest_common_divisor(numerator, denominator);
        Probability {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        }
    }

    /// Numerator of the reduced fraction.
    pub fn numerator(&self) -> u64 {
        self.numerator
    }

    /// Denominator of the reduced fraction.
    pub fn denominator(&self) -> u64 {
        self.denominator
    }

    /// Probability that this event and an independent `other` both happen; a fraction too fine
    /// for 64 bits is rounded to the nearest one that fits.
    pub fn and(self, other: Probability) -> Probability {
        let numerator = u128::from(self.numerator) * u128::from(other.numerator);
        let denominator = u128::from(self.denominator) * u128::from(other.denominator);
        let divisor = greatest_common_divisor_wide(numerator, denominator).max(1);
        let (numerator, denominator) = (numerator / divisor, denominator / divisor);
        let bits_too_many = (u128::BITS - denominator.leading_zeros()).saturating_sub(u64::BITS);
        Probability::new(
            u64::try_from(numerator >> bits_too_many).unwrap_or(u64::MAX),
            u64::try_from(denominator >> bits_too_many).unwrap_or(u64::MAX),
        )
    }

    /// Probability that this event does not happen.
    pub fn complement(self) -> Probability {
        Probability::new(self.denominator - self.numerator, self.denominator)
    }

    /// Percentage for display only (never used to decide anything).
    pub fn as_percentage(self) -> f64 {
        100.0 * self.numerator as f64 / self.denominator as f64
    }
}

impl PartialOrd for Probability {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Probability {
    fn cmp(&self, other: &Self) -> Ordering {
        let left = u128::from(self.numerator) * u128::from(other.denominator);
        let right = u128::from(other.numerator) * u128::from(self.denominator);
        left.cmp(&right)
    }
}

/// Chance that the actor wins an opposed roll: its d`actor_faces` against the opponent's
/// d`opponent_faces` plus `opponent_modifier`, the actor winning ties.
pub fn contest_success(actor_faces: u8, opponent_faces: u8, opponent_modifier: i8) -> Probability {
    let actor_faces = actor_faces.max(1);
    let opponent_faces = opponent_faces.max(1);
    let winning_pairs = (1..=actor_faces)
        .flat_map(|actor| (1..=opponent_faces).map(move |opponent| (actor, opponent)))
        .filter(|(actor, opponent)| {
            i16::from(*actor) - (i16::from(*opponent) + i16::from(opponent_modifier)) >= 0
        })
        .count();
    Probability::new(
        u64::try_from(winning_pairs).unwrap_or(u64::MAX),
        u64::from(actor_faces) * u64::from(opponent_faces),
    )
}

/// Chance that a d`faces` agility test minus `malus` reaches `target`.
pub fn agility_success(faces: u8, malus: u8, target: u8) -> Probability {
    let faces = faces.max(1);
    let successful_faces = (1..=faces)
        .filter(|face| i16::from(*face) - i16::from(malus) >= i16::from(target))
        .count();
    Probability::new(
        u64::try_from(successful_faces).unwrap_or(u64::MAX),
        u64::from(faces),
    )
}

fn greatest_common_divisor(first: u64, second: u64) -> u64 {
    if second == 0 {
        first
    } else {
        greatest_common_divisor(second, first % second)
    }
}

fn greatest_common_divisor_wide(first: u128, second: u128) -> u128 {
    if second == 0 {
        first
    } else {
        greatest_common_divisor_wide(second, first % second)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn given_d4_against_d4_plus_one_when_contested_then_three_chances_in_eight() {
        let dice = ContestDice {
            actor_modifier: 0,
            actor_faces: 4,
            opponent_faces: 4,
            opponent_modifier: 1,
        };
        assert_eq!(dice.success(), Probability::new(3, 8));
    }

    use super::*;

    #[test]
    fn given_d4_against_d4_when_asking_for_a_clear_win_then_three_chances_in_eight() {
        let dice = ContestDice {
            actor_modifier: 0,
            actor_faces: 4,
            opponent_faces: 4,
            opponent_modifier: 0,
        };
        assert_eq!(dice.clear_win(), Probability::new(3, 8));
    }

    #[test]
    fn given_d4_against_d4_when_asking_for_a_tie_then_one_chance_in_four() {
        let dice = ContestDice {
            actor_modifier: 0,
            actor_faces: 4,
            opponent_faces: 4,
            opponent_modifier: 0,
        };
        assert_eq!(dice.tie(), Probability::new(1, 4));
    }

    #[test]
    fn given_two_quarters_when_built_then_reduced_to_one_half() {
        assert_eq!(Probability::new(2, 4), Probability::new(1, 2));
    }

    #[test]
    fn given_zero_denominator_when_built_then_impossible() {
        assert_eq!(Probability::new(3, 0), Probability::IMPOSSIBLE);
    }

    #[test]
    fn given_numerator_above_denominator_when_built_then_certain() {
        assert_eq!(Probability::new(5, 4), Probability::CERTAIN);
    }

    #[test]
    fn given_two_halves_when_combined_then_one_quarter() {
        assert_eq!(
            Probability::new(1, 2).and(Probability::new(1, 2)),
            Probability::new(1, 4)
        );
    }

    #[test]
    fn given_two_near_halves_whose_product_outgrows_64_bits_when_combined_then_near_one_quarter() {
        let powers_of_three = 3_u64.pow(39);
        let near_half = Probability::new(powers_of_three / 2, powers_of_three);

        let near_quarter = near_half.and(near_half);

        assert!((near_quarter.as_percentage() - 25.0).abs() < 1e-9);
    }

    #[test]
    fn given_one_quarter_when_complemented_then_three_quarters() {
        assert_eq!(Probability::new(1, 4).complement(), Probability::new(3, 4));
    }

    #[test]
    fn given_three_eighths_and_one_half_when_compared_then_three_eighths_is_smaller() {
        assert!(Probability::new(3, 8) < Probability::new(1, 2));
    }

    #[test]
    fn given_five_eighths_when_shown_as_percentage_then_62_5() {
        assert_eq!(Probability::new(5, 8).as_percentage(), 62.5);
    }

    #[test]
    fn given_bruiser_assaults_bruiser_when_computing_odds_then_five_eighths() {
        assert_eq!(contest_success(4, 4, 0), Probability::new(5, 8));
    }

    #[test]
    fn given_bruiser_assaults_a_bruiser_in_stance_when_computing_odds_then_three_eighths() {
        assert_eq!(contest_success(4, 4, 1), Probability::new(3, 8));
    }

    #[test]
    fn given_bruiser_assaults_a_two_goblin_stance_wall_when_computing_odds_then_three_sixteenths() {
        assert_eq!(contest_success(4, 4, 2), Probability::new(3, 16));
    }

    #[test]
    fn given_bruiser_assaults_a_sprinter_when_computing_odds_then_seven_eighths() {
        assert_eq!(contest_success(4, 2, 0), Probability::new(7, 8));
    }

    #[test]
    fn given_sprinter_assaults_a_bruiser_when_computing_odds_then_one_half() {
        assert_eq!(contest_success(3, 4, 0), Probability::new(1, 2));
    }

    #[test]
    fn given_bruiser_assaults_a_downed_bruiser_when_computing_odds_then_fifteen_sixteenths() {
        assert_eq!(contest_success(4, 4, -2), Probability::new(15, 16));
    }

    #[test]
    fn given_bruiser_dodges_away_from_a_bruiser_when_computing_odds_then_three_eighths() {
        assert_eq!(contest_success(2, 4, 0), Probability::new(3, 8));
    }

    #[test]
    fn given_agility_four_without_malus_when_computing_an_agility_test_then_three_quarters() {
        assert_eq!(agility_success(4, 0, 2), Probability::new(3, 4));
    }

    #[test]
    fn given_agility_four_with_one_malus_when_computing_an_agility_test_then_one_half() {
        assert_eq!(agility_success(4, 1, 2), Probability::new(1, 2));
    }

    #[test]
    fn given_agility_two_with_one_malus_when_computing_an_agility_test_then_impossible() {
        assert_eq!(agility_success(2, 1, 2), Probability::IMPOSSIBLE);
    }

    #[test]
    fn given_a_d4_with_a_malus_of_one_against_two_when_reading_its_chance_then_one_half() {
        let dice = AgilityDice {
            bonus: 0,
            faces: 4,
            malus: 1,
            target: 2,
        };
        assert_eq!(dice.success(), Probability::new(1, 2));
    }
}
