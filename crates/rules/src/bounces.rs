//! Where a loose ball bounces: one square in the direction a die picks, among four or eight.

use crate::arena::Arena;
use crate::ruleset::Ruleset;
use crate::square::{Compass, Direction, Square};

/// Times a ball thrown into chaos bounces from its receiver's square.
pub const CHAOS_BOUNCES: u8 = 3;

/// Rolls of the direction die before a ball that keeps pointing out of bounds settles on its
/// [`settling_square`] instead.
pub const MOST_DIRECTION_ROLLS: usize = 32;

/// Moves a single bounce makes in a row, sent on by missed catches and goblins lying down, before
/// the ball rests on the nearest empty square.
pub const MOST_MOVES_IN_A_BOUNCE: usize = 32;

/// Rounds of rolls among Sprinters tied for a dive before the first of them in the bowl's order
/// dives.
pub const MOST_DIVE_TIE_BREAKS: usize = 32;

/// Faces of the die picking a bounce's direction: eight where the ball bounces all around, four
/// where it bounces orthogonally.
pub fn direction_die(ruleset: &Ruleset) -> u8 {
    if ruleset.ball_bounces_all_around {
        8
    } else {
        4
    }
}

/// The square face `face` of the direction die sends a ball on `from` to, numbered clockwise from
/// north; none when it points out of bounds, the die then rolled again.
pub fn landing(arena: &Arena, ruleset: &Ruleset, from: Square, face: u8) -> Option<Square> {
    let pointed = if ruleset.ball_bounces_all_around {
        Compass::from_d8_face(face).and_then(|direction| direction.step_from(from))
    } else {
        Direction::from_d4_face(face).and_then(|direction| direction.step_from(from))
    };
    pointed.filter(|square| arena.is_in_bounds(*square))
}

/// Where a ball on `from` settles once [`MOST_DIRECTION_ROLLS`] rolls all pointed out of bounds:
/// its first in-bounds orthogonal neighbour, or `from` itself.
pub fn settling_square(arena: &Arena, from: Square) -> Square {
    arena.in_bounds_neighbours(from).next().unwrap_or(from)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_round_bowl() -> Arena {
        Arena::from_mask(&Ruleset::default().arena).unwrap()
    }

    fn bouncing_all_around(all_around: bool) -> Ruleset {
        Ruleset {
            ball_bounces_all_around: all_around,
            ..Ruleset::default()
        }
    }

    #[test]
    fn given_balls_bouncing_all_around_when_the_die_shows_2_then_the_ball_lands_north_east() {
        assert_eq!(
            landing(
                &tiny_round_bowl(),
                &bouncing_all_around(true),
                Square::new(10, 10),
                2
            ),
            Some(Square::new(11, 9))
        );
    }

    #[test]
    fn given_balls_bouncing_orthogonally_when_the_die_shows_2_then_the_ball_lands_east() {
        assert_eq!(
            landing(
                &tiny_round_bowl(),
                &bouncing_all_around(false),
                Square::new(10, 10),
                2
            ),
            Some(Square::new(11, 10))
        );
    }

    #[test]
    fn given_the_end_of_the_bowl_s_top_row_when_the_die_points_east_then_no_landing() {
        assert_eq!(
            landing(
                &tiny_round_bowl(),
                &bouncing_all_around(true),
                Square::new(13, 0),
                3
            ),
            None
        );
    }

    #[test]
    fn given_balls_bouncing_all_around_when_asking_the_direction_die_then_a_d8() {
        assert_eq!(direction_die(&bouncing_all_around(true)), 8);
    }
}
