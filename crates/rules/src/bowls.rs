//! The bowls to pick from: three shapes in three sizes, each den the 7 middle squares of its
//! outermost column, west for home and east for away.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::ruleset::Ruleset;

/// Squares in a den, whatever the bowl.
pub const DEN_SQUARES: u8 = 7;

/// The outline of a bowl.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BowlShape {
    /// A pixelated disc.
    Circle,
    /// A square with its four corners cut out.
    Cross,
    /// A rectangle about twice as long, between the dens, as it is wide.
    Rectangle,
}

impl BowlShape {
    /// Every shape, in the order a menu shows them.
    pub const ALL: [BowlShape; 3] = [BowlShape::Circle, BowlShape::Cross, BowlShape::Rectangle];
}

/// How big a bowl is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BowlSize {
    /// 21 squares from den to den: more fights.
    Tiny,
    /// 31 squares from den to den.
    Medium,
    /// 51 squares from den to den: more room to run.
    Huge,
}

impl BowlSize {
    /// Every size, in the order a menu shows them.
    pub const ALL: [BowlSize; 3] = [BowlSize::Tiny, BowlSize::Medium, BowlSize::Huge];

    /// Squares from one den to the other.
    pub fn length(self) -> u8 {
        match self {
            BowlSize::Tiny => 21,
            BowlSize::Medium => 31,
            BowlSize::Huge => 51,
        }
    }
}

/// A name that is not a bowl shape or size.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("unknown bowl {0}: circle, cross or rectangle, and tiny, medium or huge")]
pub struct UnknownBowl(pub String);

impl FromStr for BowlShape {
    type Err = UnknownBowl;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "circle" => Ok(BowlShape::Circle),
            "cross" => Ok(BowlShape::Cross),
            "rectangle" => Ok(BowlShape::Rectangle),
            other => Err(UnknownBowl(other.to_string())),
        }
    }
}

impl fmt::Display for BowlShape {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            BowlShape::Circle => "circle",
            BowlShape::Cross => "cross",
            BowlShape::Rectangle => "rectangle",
        })
    }
}

impl FromStr for BowlSize {
    type Err = UnknownBowl;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "tiny" => Ok(BowlSize::Tiny),
            "medium" => Ok(BowlSize::Medium),
            "huge" => Ok(BowlSize::Huge),
            other => Err(UnknownBowl(other.to_string())),
        }
    }
}

impl fmt::Display for BowlSize {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            BowlSize::Tiny => "tiny",
            BowlSize::Medium => "medium",
            BowlSize::Huge => "huge",
        })
    }
}

/// The mask of a bowl of `shape` and `size`: `x` out of bounds, `O` in bounds, `H` the home den on
/// the west edge and `A` the away den on the east edge, each the 7 middle squares of its column.
pub fn bowl_mask(shape: BowlShape, size: BowlSize) -> Vec<String> {
    let length = size.length();
    let width = match shape {
        BowlShape::Circle | BowlShape::Cross => length,
        BowlShape::Rectangle => (length / 2) | 1,
    };
    let middle_row = (width - 1) / 2;
    let den_rows = middle_row - DEN_SQUARES / 2..=middle_row + DEN_SQUARES / 2;
    (0..width)
        .map(|y| {
            (0..length)
                .map(|x| {
                    if !in_bounds(shape, length, width, x, y) {
                        'x'
                    } else if den_rows.contains(&y) && x == 0 {
                        'H'
                    } else if den_rows.contains(&y) && x == length - 1 {
                        'A'
                    } else {
                        'O'
                    }
                })
                .collect()
        })
        .collect()
}

/// `ruleset` played in the bowl of `shape` and `size`, a circle and tiny when one is left out; the
/// ruleset's own bowl when both are.
pub fn with_bowl(ruleset: Ruleset, shape: Option<BowlShape>, size: Option<BowlSize>) -> Ruleset {
    if shape.is_none() && size.is_none() {
        return ruleset;
    }
    Ruleset {
        arena: bowl_mask(
            shape.unwrap_or(BowlShape::Circle),
            size.unwrap_or(BowlSize::Tiny),
        ),
        ..ruleset
    }
}

fn in_bounds(shape: BowlShape, length: u8, width: u8, x: u8, y: u8) -> bool {
    match shape {
        BowlShape::Circle => {
            let radius = i32::from(length - 1) / 2;
            let east = i32::from(x) - radius;
            let south = i32::from(y) - radius;
            4 * (east * east + south * south) <= (2 * radius + 1) * (2 * radius + 1)
        }
        BowlShape::Cross => {
            let corner = length / 4;
            let in_a_corner_column = x < corner || x >= length - corner;
            let in_a_corner_row = y < corner || y >= width - corner;
            !(in_a_corner_column && in_a_corner_row)
        }
        BowlShape::Rectangle => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::{Arena, ROUND_BOWL_21_MASK};
    use crate::square::Square;
    use crate::team::TeamId;

    fn arena(shape: BowlShape, size: BowlSize) -> Arena {
        Arena::from_mask(&bowl_mask(shape, size)).unwrap()
    }

    #[test]
    fn given_the_tiny_circle_when_drawn_then_it_is_the_round_bowl_of_rules_0_8() {
        assert_eq!(
            bowl_mask(BowlShape::Circle, BowlSize::Tiny),
            ROUND_BOWL_21_MASK
                .iter()
                .map(|row| (*row).to_string())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn given_the_medium_rectangle_when_drawn_then_31_long_and_15_wide() {
        let rectangle = arena(BowlShape::Rectangle, BowlSize::Medium);
        assert_eq!((rectangle.width(), rectangle.height()), (31, 15));
    }

    #[test]
    fn given_the_tiny_cross_when_listing_the_home_den_then_the_seven_middle_squares_of_the_west_column()
     {
        let den: Vec<Square> = arena(BowlShape::Cross, BowlSize::Tiny)
            .den_squares(TeamId::HOME)
            .collect();
        assert_eq!(den, (7..=13).map(|y| Square::new(0, y)).collect::<Vec<_>>());
    }

    #[test]
    fn given_the_huge_circle_when_listing_the_away_den_then_seven_squares_of_the_east_column() {
        let den: Vec<Square> = arena(BowlShape::Circle, BowlSize::Huge)
            .den_squares(TeamId::AWAY)
            .collect();
        assert_eq!(
            den,
            (22..=28).map(|y| Square::new(50, y)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn given_every_shape_and_size_when_used_with_the_shipped_rules_then_a_valid_ruleset() {
        let invalid: Vec<(BowlShape, BowlSize)> = BowlShape::ALL
            .into_iter()
            .flat_map(|shape| BowlSize::ALL.into_iter().map(move |size| (shape, size)))
            .filter(|(shape, size)| {
                Ruleset {
                    arena: bowl_mask(*shape, *size),
                    ..Ruleset::default()
                }
                .validate()
                .is_err()
            })
            .collect();
        assert_eq!(invalid, []);
    }

    #[test]
    fn given_only_a_size_when_picking_the_bowl_then_a_circle_of_that_size() {
        let ruleset = with_bowl(Ruleset::default(), None, Some(BowlSize::Medium));
        assert_eq!(
            ruleset.arena,
            bowl_mask(BowlShape::Circle, BowlSize::Medium)
        );
    }

    #[test]
    fn given_no_shape_nor_size_when_picking_the_bowl_then_the_rules_keep_theirs() {
        let ruleset = with_bowl(Ruleset::default(), None, None);
        assert_eq!(ruleset, Ruleset::default());
    }

    #[test]
    fn given_the_name_rectangle_when_read_as_a_shape_then_the_rectangle() {
        assert_eq!("rectangle".parse(), Ok(BowlShape::Rectangle));
    }

    #[test]
    fn given_an_unknown_size_when_read_then_an_error_naming_the_choices() {
        assert_eq!(
            "gigantic".parse::<BowlSize>(),
            Err(UnknownBowl("gigantic".to_string()))
        );
    }
}
