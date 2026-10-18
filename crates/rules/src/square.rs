//! Squares of the bowl grid, their orthogonal geometry and the reach of a goblin around its square.

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

/// One of the eight directions around a square, numbered 1 to 8 clockwise from north as on a d8,
/// and written as its number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(into = "u8", try_from = "u8")]
pub enum Compass {
    /// Towards row 0.
    North,
    /// Towards row 0 and higher columns.
    NorthEast,
    /// Towards higher columns.
    East,
    /// Towards higher rows and higher columns.
    SouthEast,
    /// Towards higher rows.
    South,
    /// Towards higher rows and column 0.
    SouthWest,
    /// Towards column 0.
    West,
    /// Towards row 0 and column 0.
    NorthWest,
}

impl Compass {
    /// The eight directions clockwise from north, the order of their numbers 1 to 8.
    pub const ALL: [Compass; 8] = [
        Compass::North,
        Compass::NorthEast,
        Compass::East,
        Compass::SouthEast,
        Compass::South,
        Compass::SouthWest,
        Compass::West,
        Compass::NorthWest,
    ];

    /// Maps a d8 face, or a direction's number, from 1 to 8 to its direction.
    pub fn from_d8_face(face: u8) -> Option<Compass> {
        let index = usize::from(face.checked_sub(1)?);
        Compass::ALL.get(index).copied()
    }

    /// The same direction seen in a mirror between east and west.
    pub fn mirrored_east_west(self) -> Compass {
        match self {
            Compass::NorthEast => Compass::NorthWest,
            Compass::East => Compass::West,
            Compass::SouthEast => Compass::SouthWest,
            Compass::SouthWest => Compass::SouthEast,
            Compass::West => Compass::East,
            Compass::NorthWest => Compass::NorthEast,
            Compass::North | Compass::South => self,
        }
    }

    /// Its number, 1 for north to 8 for north-west.
    pub fn number(self) -> u8 {
        Compass::ALL
            .iter()
            .position(|direction| *direction == self)
            .and_then(|index| u8::try_from(index + 1).ok())
            .unwrap_or(1)
    }

    /// The square one step away from `origin` in this direction, or `None` below coordinate zero.
    pub fn step_from(self, origin: Square) -> Option<Square> {
        let (east, south): (i16, i16) = match self {
            Compass::North => (0, -1),
            Compass::NorthEast => (1, -1),
            Compass::East => (1, 0),
            Compass::SouthEast => (1, 1),
            Compass::South => (0, 1),
            Compass::SouthWest => (-1, 1),
            Compass::West => (-1, 0),
            Compass::NorthWest => (-1, -1),
        };
        let x = u8::try_from(i16::from(origin.x) + east).ok()?;
        let y = u8::try_from(i16::from(origin.y) + south).ok()?;
        Some(Square::new(x, y))
    }
}

impl From<Compass> for u8 {
    fn from(direction: Compass) -> Self {
        direction.number()
    }
}

impl TryFrom<u8> for Compass {
    type Error = String;

    fn try_from(number: u8) -> Result<Self, Self::Error> {
        Compass::from_d8_face(number)
            .ok_or_else(|| format!("{number} is not a direction from 1 to 8"))
    }
}

/// The squares a standing goblin weighs on around it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reach {
    /// The four squares one orthogonal step away.
    Orthogonal,
    /// The eight squares around, diagonals included.
    AllAround,
}

impl Square {
    /// Builds the square at column `x` and row `y`.
    pub const fn new(x: u8, y: u8) -> Self {
        Self { x, y }
    }

    /// Whether `other` lies within `reach` of this square, the square itself left out.
    pub fn is_within(self, other: Square, reach: Reach) -> bool {
        match reach {
            Reach::Orthogonal => self.is_orthogonally_adjacent(other),
            Reach::AllAround => {
                self != other && self.x.abs_diff(other.x) <= 1 && self.y.abs_diff(other.y) <= 1
            }
        }
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
    fn given_diagonal_squares_when_the_reach_covers_all_around_then_within_reach() {
        assert!(Square::new(4, 4).is_within(Square::new(5, 5), Reach::AllAround));
    }

    #[test]
    fn given_diagonal_squares_when_the_reach_is_orthogonal_then_out_of_reach() {
        assert!(!Square::new(4, 4).is_within(Square::new(5, 5), Reach::Orthogonal));
    }

    #[test]
    fn given_the_same_square_when_the_reach_covers_all_around_then_out_of_reach() {
        assert!(!Square::new(4, 4).is_within(Square::new(4, 4), Reach::AllAround));
    }

    #[test]
    fn given_squares_two_apart_when_the_reach_covers_all_around_then_out_of_reach() {
        assert!(!Square::new(4, 4).is_within(Square::new(6, 5), Reach::AllAround));
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
    fn given_d8_faces_when_mapped_then_clockwise_from_north() {
        let directions: Vec<Option<Compass>> = (1..=8).map(Compass::from_d8_face).collect();
        assert_eq!(
            directions,
            Compass::ALL.into_iter().map(Some).collect::<Vec<_>>()
        );
    }

    #[test]
    fn given_the_south_east_when_mirrored_east_west_then_the_south_west() {
        assert_eq!(Compass::SouthEast.mirrored_east_west(), Compass::SouthWest);
    }

    #[test]
    fn given_the_south_west_when_written_then_its_number_six() {
        assert_eq!(serde_json::to_string(&Compass::SouthWest).unwrap(), "6");
    }

    #[test]
    fn given_nine_when_read_as_a_direction_then_refused() {
        assert!(serde_json::from_str::<Compass>("9").is_err());
    }

    #[test]
    fn given_the_north_east_when_stepping_then_one_column_east_one_row_north() {
        assert_eq!(
            Compass::NorthEast.step_from(Square::new(4, 7)),
            Some(Square::new(5, 6))
        );
    }

    #[test]
    fn given_the_north_west_from_the_corner_when_stepping_then_off_the_grid() {
        assert_eq!(Compass::NorthWest.step_from(Square::new(0, 0)), None);
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
