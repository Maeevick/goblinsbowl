//! The path of a thrown ball: the squares crossed by the straight line between two square centres.

use crate::square::Square;

/// Squares the ball flies over between `from` and `to`, in flight order, both ends excluded.
///
/// A square counts when the line between the two centres passes through its inside; a line passing
/// exactly through a corner goes straight on diagonally, touching neither side square.
pub fn squares_crossed(from: Square, to: Square) -> Vec<Square> {
    let horizontal_distance = i32::from(from.x.abs_diff(to.x));
    let vertical_distance = i32::from(from.y.abs_diff(to.y));
    let step_x = if to.x > from.x { 1 } else { -1 };
    let step_y = if to.y > from.y { 1 } else { -1 };
    let (mut x, mut y) = (i32::from(from.x), i32::from(from.y));
    let (mut steps_x, mut steps_y) = (0, 0);
    let mut crossed = Vec::new();
    while steps_x < horizontal_distance || steps_y < vertical_distance {
        let next_crossing =
            (1 + 2 * steps_x) * vertical_distance - (1 + 2 * steps_y) * horizontal_distance;
        if next_crossing <= 0 {
            x += step_x;
            steps_x += 1;
        }
        if next_crossing >= 0 {
            y += step_y;
            steps_y += 1;
        }
        crossed.push(Square::new(
            u8::try_from(x).unwrap_or_default(),
            u8::try_from(y).unwrap_or_default(),
        ));
    }
    crossed.pop();
    crossed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn crossed(from: (u8, u8), to: (u8, u8)) -> Vec<(u8, u8)> {
        squares_crossed(Square::new(from.0, from.1), Square::new(to.0, to.1))
            .into_iter()
            .map(|square| (square.x, square.y))
            .collect()
    }

    #[test]
    fn given_a_straight_east_throw_when_tracing_then_every_square_in_between() {
        assert_eq!(crossed((0, 0), (3, 0)), vec![(1, 0), (2, 0)]);
    }

    #[test]
    fn given_a_straight_north_throw_when_tracing_then_squares_in_flight_order() {
        assert_eq!(crossed((2, 5), (2, 1)), vec![(2, 4), (2, 3), (2, 2)]);
    }

    #[test]
    fn given_an_exact_diagonal_throw_when_tracing_then_only_diagonal_squares() {
        assert_eq!(crossed((0, 0), (3, 3)), vec![(1, 1), (2, 2)]);
    }

    #[test]
    fn given_a_shallow_throw_through_a_corner_when_tracing_then_the_corner_is_skipped() {
        assert_eq!(crossed((0, 0), (3, 1)), vec![(1, 0), (2, 1)]);
    }

    #[test]
    fn given_the_same_throw_backwards_when_tracing_then_the_mirror_path() {
        assert_eq!(crossed((3, 1), (0, 0)), vec![(2, 1), (1, 0)]);
    }

    #[test]
    fn given_a_knight_move_throw_when_tracing_then_both_squares_crossed() {
        assert_eq!(crossed((0, 0), (1, 2)), vec![(0, 1), (1, 1)]);
    }

    #[test]
    fn given_neighbouring_squares_when_tracing_then_nothing_in_between() {
        assert_eq!(crossed((4, 4), (5, 4)), vec![]);
    }

    #[test]
    fn given_the_same_square_when_tracing_then_nothing_in_between() {
        assert_eq!(crossed((4, 4), (4, 4)), vec![]);
    }
}
