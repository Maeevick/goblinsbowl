//! Squares of the bowl grid and their orthogonal geometry.

use serde::{Deserialize, Serialize};

/// A square of the bowl grid: `x` grows from west to east, `y` from north to south.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(from = "[u8; 2]", into = "[u8; 2]")]
pub struct Square {
    /// Column, counted from the west edge.
    pub x: u8,
    /// Row, counted from the north edge.
    pub y: u8,
}

/// One of the four orthogonal directions a goblin or a bouncing ball can take.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
    /// Towards row 0.
    North,
    /// Towards higher columns.
    East,
    /// Towards higher rows.
    South,
    /// Towards column 0.
    West,
}

impl Direction {
    /// The four directions in the fixed order used everywhere for determinism: north, east, south, west.
    pub const ALL: [Direction; 4] = [
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ];

    /// Maps a d4 face (1 to 4) to a direction, in the order of [`Direction::ALL`].
    ///
    /// Returns `None` for any other face value.
    pub fn from_d4_face(face: u8) -> Option<Direction> {
        let index = usize::from(face.checked_sub(1)?);
        Direction::ALL.get(index).copied()
    }

    /// The square one step away from `origin` in this direction, or `None` below coordinate zero.
    pub fn step_from(self, origin: Square) -> Option<Square> {
        match self {
            Direction::North => Some(Square::new(origin.x, origin.y.checked_sub(1)?)),
            Direction::East => Some(Square::new(origin.x.checked_add(1)?, origin.y)),
            Direction::South => Some(Square::new(origin.x, origin.y.checked_add(1)?)),
            Direction::West => Some(Square::new(origin.x.checked_sub(1)?, origin.y)),
        }
    }
}

impl Square {
    /// Builds the square at column `x` and row `y`.
    pub const fn new(x: u8, y: u8) -> Self {
        Self { x, y }
    }

    /// Squares one orthogonal step away, in [`Direction::ALL`] order, skipping coordinates below zero.
    ///
    /// The upper bounds are not checked: filter with the arena to keep inbound squares only.
    pub fn orthogonal_neighbours(self) -> impl Iterator<Item = Square> {
        Direction::ALL
            .into_iter()
            .filter_map(move |direction| direction.step_from(self))
    }

    /// Number of orthogonal steps between two squares (Manhattan distance), ignoring obstacles.
    pub fn orthogonal_distance(self, other: Square) -> u8 {
        self.x.abs_diff(other.x) + self.y.abs_diff(other.y)
    }

    /// Whether `other` is exactly one orthogonal step away.
    pub fn is_orthogonally_adjacent(self, other: Square) -> bool {
        self.orthogonal_distance(other) == 1
    }
}

impl From<[u8; 2]> for Square {
    fn from([x, y]: [u8; 2]) -> Self {
        Square::new(x, y)
    }
}

impl From<Square> for [u8; 2] {
    fn from(square: Square) -> Self {
        [square.x, square.y]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_two_squares_when_measuring_then_distance_counts_orthogonal_steps() {
        assert_eq!(Square::new(2, 3).orthogonal_distance(Square::new(5, 1)), 5);
    }

    #[test]
    fn given_diagonal_squares_when_checking_adjacency_then_not_adjacent() {
        assert!(!Square::new(4, 4).is_orthogonally_adjacent(Square::new(5, 5)));
    }

    #[test]
    fn given_side_by_side_squares_when_checking_adjacency_then_adjacent() {
        assert!(Square::new(4, 4).is_orthogonally_adjacent(Square::new(4, 5)));
    }

    #[test]
    fn given_same_square_when_checking_adjacency_then_not_adjacent() {
        assert!(!Square::new(4, 4).is_orthogonally_adjacent(Square::new(4, 4)));
    }

    #[test]
    fn given_corner_origin_when_listing_neighbours_then_only_east_and_south() {
        let neighbours: Vec<Square> = Square::new(0, 0).orthogonal_neighbours().collect();
        assert_eq!(neighbours, vec![Square::new(1, 0), Square::new(0, 1)]);
    }

    #[test]
    fn given_d4_faces_when_mapped_then_follow_north_east_south_west() {
        let directions: Vec<Option<Direction>> = (1..=4).map(Direction::from_d4_face).collect();
        assert_eq!(
            directions,
            Direction::ALL.into_iter().map(Some).collect::<Vec<_>>()
        );
    }

    #[test]
    fn given_face_outside_d4_when_mapped_then_none() {
        assert_eq!(Direction::from_d4_face(5), None);
    }

    #[test]
    fn given_square_when_serialised_then_json_is_an_xy_pair() {
        assert_eq!(serde_json::to_string(&Square::new(7, 3)).unwrap(), "[7,3]");
    }
}
