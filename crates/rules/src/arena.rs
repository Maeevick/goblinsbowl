//! The bowl: which squares are in bounds and which belong to each team's den.

use thiserror::Error;

use crate::square::Square;
use crate::team::TeamId;

/// Mask of the default bowl: a 15 × 15 grid with its four 3 × 3 corners cut out (189 squares).
///
/// Legend: `x` out of bounds, `O` in bounds, `H` home den, `A` away den.
pub const BOWL_15_MASK: [&str; 15] = [
    "xxxOOOOOOOOOxxx",
    "xxxOOOOOOOOOxxx",
    "xxxOOOOOOOOOxxx",
    "HOOOOOOOOOOOOOA",
    "HOOOOOOOOOOOOOA",
    "HOOOOOOOOOOOOOA",
    "HOOOOOOOOOOOOOA",
    "HOOOOOOOOOOOOOA",
    "HOOOOOOOOOOOOOA",
    "HOOOOOOOOOOOOOA",
    "HOOOOOOOOOOOOOA",
    "HOOOOOOOOOOOOOA",
    "xxxOOOOOOOOOxxx",
    "xxxOOOOOOOOOxxx",
    "xxxOOOOOOOOOxxx",
];

/// Mask of the large bowl: a 21 × 21 grid with its four 5 × 5 corners cut out (341 squares),
/// each den 11 squares long.
pub const BOWL_21_MASK: [&str; 21] = [
    "xxxxxOOOOOOOOOOOxxxxx",
    "xxxxxOOOOOOOOOOOxxxxx",
    "xxxxxOOOOOOOOOOOxxxxx",
    "xxxxxOOOOOOOOOOOxxxxx",
    "xxxxxOOOOOOOOOOOxxxxx",
    "HOOOOOOOOOOOOOOOOOOOA",
    "HOOOOOOOOOOOOOOOOOOOA",
    "HOOOOOOOOOOOOOOOOOOOA",
    "HOOOOOOOOOOOOOOOOOOOA",
    "HOOOOOOOOOOOOOOOOOOOA",
    "HOOOOOOOOOOOOOOOOOOOA",
    "HOOOOOOOOOOOOOOOOOOOA",
    "HOOOOOOOOOOOOOOOOOOOA",
    "HOOOOOOOOOOOOOOOOOOOA",
    "HOOOOOOOOOOOOOOOOOOOA",
    "HOOOOOOOOOOOOOOOOOOOA",
    "xxxxxOOOOOOOOOOOxxxxx",
    "xxxxxOOOOOOOOOOOxxxxx",
    "xxxxxOOOOOOOOOOOxxxxx",
    "xxxxxOOOOOOOOOOOxxxxx",
    "xxxxxOOOOOOOOOOOxxxxx",
];

/// Mask of the round bowl: a pixelated disc 21 squares across (349 squares), each den the arc of
/// 11 squares at its west or east edge.
pub const ROUND_BOWL_21_MASK: [&str; 21] = [
    "xxxxxxxOOOOOOOxxxxxxx",
    "xxxxxOOOOOOOOOOOxxxxx",
    "xxxxOOOOOOOOOOOOOxxxx",
    "xxxOOOOOOOOOOOOOOOxxx",
    "xxOOOOOOOOOOOOOOOOOxx",
    "xHOOOOOOOOOOOOOOOOOAx",
    "xHOOOOOOOOOOOOOOOOOAx",
    "HOOOOOOOOOOOOOOOOOOOA",
    "HOOOOOOOOOOOOOOOOOOOA",
    "HOOOOOOOOOOOOOOOOOOOA",
    "HOOOOOOOOOOOOOOOOOOOA",
    "HOOOOOOOOOOOOOOOOOOOA",
    "HOOOOOOOOOOOOOOOOOOOA",
    "HOOOOOOOOOOOOOOOOOOOA",
    "xHOOOOOOOOOOOOOOOOOAx",
    "xHOOOOOOOOOOOOOOOOOAx",
    "xxOOOOOOOOOOOOOOOOOxx",
    "xxxOOOOOOOOOOOOOOOxxx",
    "xxxxOOOOOOOOOOOOOxxxx",
    "xxxxxOOOOOOOOOOOxxxxx",
    "xxxxxxxOOOOOOOxxxxxxx",
];

/// Mask of the compact bowl: a 12 × 12 grid with its four 3 × 3 corners cut out (108 squares).
pub const BOWL_12_MASK: [&str; 12] = [
    "xxxOOOOOOxxx",
    "xxxOOOOOOxxx",
    "xxxOOOOOOxxx",
    "HOOOOOOOOOOA",
    "HOOOOOOOOOOA",
    "HOOOOOOOOOOA",
    "HOOOOOOOOOOA",
    "HOOOOOOOOOOA",
    "HOOOOOOOOOOA",
    "xxxOOOOOOxxx",
    "xxxOOOOOOxxx",
    "xxxOOOOOOxxx",
];

/// Why an arena mask could not be turned into an [`Arena`].
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ArenaError {
    /// The mask has no row or its first row is empty.
    #[error("the arena mask is empty")]
    EmptyMask,
    /// A row does not have the same width as the first one.
    #[error("row {row} has {found} squares, expected {expected}")]
    RaggedRow {
        /// Index of the offending row.
        row: usize,
        /// Width of the first row.
        expected: usize,
        /// Width of the offending row.
        found: usize,
    },
    /// The mask is wider or taller than 255 squares.
    #[error("the arena mask exceeds 255 squares in one dimension")]
    TooLarge,
    /// A character is not part of the legend (`x`, `O`, `H`, `A`).
    #[error("unknown symbol {symbol:?} at [{x}, {y}]")]
    UnknownSymbol {
        /// The unexpected character.
        symbol: char,
        /// Its column.
        x: usize,
        /// Its row.
        y: usize,
    },
}

/// What a square of the mask is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ground {
    OutOfBounds,
    Field,
    Den(TeamId),
}

/// The playing area: a grid where some squares are in bounds and some are team dens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arena {
    width: u8,
    height: u8,
    grounds: Vec<Ground>,
}

impl Arena {
    /// Parses an ASCII mask (`x` out of bounds, `O` in bounds, `H` home den, `A` away den).
    ///
    /// # Errors
    /// Returns an [`ArenaError`] when the mask is empty, ragged, larger than 255 squares
    /// in a dimension, or contains an unknown symbol.
    pub fn from_mask<S: AsRef<str>>(rows: &[S]) -> Result<Arena, ArenaError> {
        let expected_width = rows
            .first()
            .map(|row| row.as_ref().chars().count())
            .filter(|width| *width > 0)
            .ok_or(ArenaError::EmptyMask)?;
        let width = u8::try_from(expected_width).map_err(|_| ArenaError::TooLarge)?;
        let height = u8::try_from(rows.len()).map_err(|_| ArenaError::TooLarge)?;
        let mut grounds = Vec::with_capacity(expected_width * rows.len());
        for (y, row) in rows.iter().enumerate() {
            let found = row.as_ref().chars().count();
            if found != expected_width {
                return Err(ArenaError::RaggedRow {
                    row: y,
                    expected: expected_width,
                    found,
                });
            }
            for (x, symbol) in row.as_ref().chars().enumerate() {
                grounds.push(ground_from_symbol(symbol).ok_or(ArenaError::UnknownSymbol {
                    symbol,
                    x,
                    y,
                })?);
            }
        }
        Ok(Arena {
            width,
            height,
            grounds,
        })
    }

    /// The default 15 × 15 bowl.
    pub fn bowl_15() -> Arena {
        match Arena::from_mask(&BOWL_15_MASK) {
            Ok(arena) => arena,
            Err(error) => unreachable!("the built-in bowl mask is valid: {error}"),
        }
    }

    /// Number of columns of the grid, including out-of-bounds squares.
    pub fn width(&self) -> u8 {
        self.width
    }

    /// Number of rows of the grid, including out-of-bounds squares.
    pub fn height(&self) -> u8 {
        self.height
    }

    /// Whether `square` is part of the playing area.
    pub fn is_in_bounds(&self, square: Square) -> bool {
        !matches!(self.ground(square), Ground::OutOfBounds)
    }

    /// The team owning the den on `square`, if `square` belongs to a den.
    pub fn den_owner(&self, square: Square) -> Option<TeamId> {
        match self.ground(square) {
            Ground::Den(owner) => Some(owner),
            Ground::Field | Ground::OutOfBounds => None,
        }
    }

    /// Whether a goblin of `team` holding the ball on `square` scores: the square is a den owned by another team.
    pub fn is_scoring_square_for(&self, team: TeamId, square: Square) -> bool {
        self.den_owner(square).is_some_and(|owner| owner != team)
    }

    /// All in-bounds squares, row by row from the north-west corner.
    pub fn in_bounds_squares(&self) -> impl Iterator<Item = Square> + '_ {
        (0..self.height).flat_map(move |y| {
            (0..self.width)
                .map(move |x| Square::new(x, y))
                .filter(move |square| self.is_in_bounds(*square))
        })
    }

    /// The squares of the den owned by `team`, row by row.
    pub fn den_squares(&self, team: TeamId) -> impl Iterator<Item = Square> + '_ {
        self.in_bounds_squares()
            .filter(move |square| self.den_owner(*square) == Some(team))
    }

    /// In-bounds squares one orthogonal step away from `square`, in north, east, south, west order.
    pub fn in_bounds_neighbours(&self, square: Square) -> impl Iterator<Item = Square> + '_ {
        square
            .orthogonal_neighbours()
            .filter(move |neighbour| self.is_in_bounds(*neighbour))
    }

    /// The square where the ball drops at kick-off: the middle of the grid (rounded towards south-east).
    pub fn centre(&self) -> Square {
        Square::new(self.width / 2, self.height / 2)
    }

    fn ground(&self, square: Square) -> Ground {
        if square.x >= self.width || square.y >= self.height {
            return Ground::OutOfBounds;
        }
        let index = usize::from(square.y) * usize::from(self.width) + usize::from(square.x);
        self.grounds
            .get(index)
            .copied()
            .unwrap_or(Ground::OutOfBounds)
    }
}

fn ground_from_symbol(symbol: char) -> Option<Ground> {
    match symbol {
        'x' => Some(Ground::OutOfBounds),
        'O' => Some(Ground::Field),
        'H' => Some(Ground::Den(TeamId::HOME)),
        'A' => Some(Ground::Den(TeamId::AWAY)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_bowl_15_mask_when_parsed_then_189_squares_are_in_bounds() {
        assert_eq!(Arena::bowl_15().in_bounds_squares().count(), 189);
    }

    #[test]
    fn given_bowl_21_mask_when_parsed_then_341_squares_are_in_bounds() {
        let arena = Arena::from_mask(&BOWL_21_MASK).unwrap();
        assert_eq!(arena.in_bounds_squares().count(), 341);
    }

    #[test]
    fn given_bowl_21_mask_when_listing_the_away_den_then_eleven_squares_on_the_east_edge() {
        let arena = Arena::from_mask(&BOWL_21_MASK).unwrap();
        let den: Vec<Square> = arena.den_squares(TeamId::AWAY).collect();
        assert_eq!(
            den,
            (5..=15).map(|y| Square::new(20, y)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn given_the_round_bowl_mask_when_parsed_then_349_squares_are_in_bounds() {
        let arena = Arena::from_mask(&ROUND_BOWL_21_MASK).unwrap();
        assert_eq!(arena.in_bounds_squares().count(), 349);
    }

    #[test]
    fn given_the_round_bowl_mask_when_listing_the_home_den_then_an_arc_of_eleven_squares_on_the_west_edge()
     {
        let arena = Arena::from_mask(&ROUND_BOWL_21_MASK).unwrap();
        let den: Vec<Square> = arena.den_squares(TeamId::HOME).collect();
        let arc: Vec<Square> = (5..=15)
            .map(|y| Square::new(if (7..=13).contains(&y) { 0 } else { 1 }, y))
            .collect();
        assert_eq!(den, arc);
    }

    #[test]
    fn given_bowl_12_mask_when_parsed_then_108_squares_are_in_bounds() {
        let arena = Arena::from_mask(&BOWL_12_MASK).unwrap();
        assert_eq!(arena.in_bounds_squares().count(), 108);
    }

    #[test]
    fn given_bowl_15_when_checking_a_cut_corner_then_out_of_bounds() {
        assert!(!Arena::bowl_15().is_in_bounds(Square::new(2, 2)));
    }

    #[test]
    fn given_bowl_15_when_checking_an_arm_tip_then_in_bounds() {
        assert!(Arena::bowl_15().is_in_bounds(Square::new(3, 0)));
    }

    #[test]
    fn given_bowl_15_when_checking_beyond_the_grid_then_out_of_bounds() {
        assert!(!Arena::bowl_15().is_in_bounds(Square::new(15, 7)));
    }

    #[test]
    fn given_bowl_15_when_listing_home_den_then_nine_squares_on_the_west_edge() {
        let den: Vec<Square> = Arena::bowl_15().den_squares(TeamId::HOME).collect();
        assert_eq!(den, (3..=11).map(|y| Square::new(0, y)).collect::<Vec<_>>());
    }

    #[test]
    fn given_bowl_15_when_checking_east_edge_then_away_owns_the_den() {
        assert_eq!(
            Arena::bowl_15().den_owner(Square::new(14, 7)),
            Some(TeamId::AWAY)
        );
    }

    #[test]
    fn given_home_goblin_in_away_den_when_checking_scoring_then_scores() {
        assert!(Arena::bowl_15().is_scoring_square_for(TeamId::HOME, Square::new(14, 5)));
    }

    #[test]
    fn given_home_goblin_in_home_den_when_checking_scoring_then_does_not_score() {
        assert!(!Arena::bowl_15().is_scoring_square_for(TeamId::HOME, Square::new(0, 5)));
    }

    #[test]
    fn given_home_goblin_on_plain_field_when_checking_scoring_then_does_not_score() {
        assert!(!Arena::bowl_15().is_scoring_square_for(TeamId::HOME, Square::new(13, 5)));
    }

    #[test]
    fn given_den_corner_when_listing_neighbours_then_only_in_bounds_ones() {
        let neighbours: Vec<Square> = Arena::bowl_15()
            .in_bounds_neighbours(Square::new(0, 3))
            .collect();
        assert_eq!(neighbours, vec![Square::new(1, 3), Square::new(0, 4)]);
    }

    #[test]
    fn given_bowl_15_when_asking_centre_then_seven_seven() {
        assert_eq!(Arena::bowl_15().centre(), Square::new(7, 7));
    }

    #[test]
    fn given_empty_mask_when_parsed_then_empty_mask_error() {
        let rows: [&str; 0] = [];
        assert_eq!(Arena::from_mask(&rows), Err(ArenaError::EmptyMask));
    }

    #[test]
    fn given_ragged_mask_when_parsed_then_ragged_row_error() {
        assert_eq!(
            Arena::from_mask(&["OOO", "OO"]),
            Err(ArenaError::RaggedRow {
                row: 1,
                expected: 3,
                found: 2
            })
        );
    }

    #[test]
    fn given_unknown_symbol_when_parsed_then_unknown_symbol_error() {
        assert_eq!(
            Arena::from_mask(&["OOO", "O?O"]),
            Err(ArenaError::UnknownSymbol {
                symbol: '?',
                x: 1,
                y: 1
            })
        );
    }

    #[test]
    fn given_mask_wider_than_255_when_parsed_then_too_large_error() {
        let wide_row = "O".repeat(256);
        assert_eq!(Arena::from_mask(&[wide_row]), Err(ArenaError::TooLarge));
    }
}
