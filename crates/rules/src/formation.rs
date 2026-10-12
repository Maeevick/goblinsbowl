//! Kick-off formations: where each goblin stands when play starts or restarts.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::arena::Arena;
use crate::characteristics::Archetype;
use crate::roster::Roster;
use crate::square::Square;
use crate::team::TeamId;

/// A line of the home team's kick-off formation, mirrored for away: squares at one depth behind
/// the centre column, taken by the archetypes the line prefers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormationLine {
    /// Columns between the centre column and the line, towards the team's own den.
    pub depth: u8,
    /// Rows of the line's squares, counted from the centre row, negative towards the north.
    pub across: Vec<i8>,
    /// Archetypes that take the line's squares first, in order; any goblin left fills the rest.
    pub prefers: Vec<Archetype>,
}

/// Where goblins stand at kick-off.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Formation {
    /// Rules 0.1: Bruisers hold the front line next to the centre, Sprinters the line two steps
    /// behind, Throwers two steps further, each group spread evenly across the rows of the team's
    /// den; a group larger than the den spills onto the next column back.
    #[default]
    DenLines,
    /// Lines at chosen depths, filled in order, each square by the goblin its line prefers.
    Lines(Vec<FormationLine>),
}

/// Why a formation cannot line a team up.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum FormationError {
    /// Fewer squares than goblins in a team.
    #[error("the formation has {squares} squares for {team_size} goblins")]
    TooFewSquares {
        /// Squares named by the formation.
        squares: usize,
        /// Goblins per team.
        team_size: u8,
    },
    /// A square sits on the centre column, between the two halves.
    #[error("the formation square across {across} sits on the centre column (depth 0)")]
    OnTheCentreColumn {
        /// Its row from the centre row.
        across: i8,
    },
    /// A square, or its mirror for the away team, is out of bounds.
    #[error("the formation square at depth {depth}, across {across} is off the bowl")]
    OffTheBowl {
        /// Its column behind the centre column.
        depth: u8,
        /// Its row from the centre row.
        across: i8,
    },
    /// Two squares of the formation are the same.
    #[error("the formation names the square at depth {depth}, across {across} twice")]
    SquareNamedTwice {
        /// Its column behind the centre column.
        depth: u8,
        /// Its row from the centre row.
        across: i8,
    },
}

impl Formation {
    /// Whether this is the rules 0.1 formation, which match records of rules 0.1 leave out.
    pub fn is_den_lines(&self) -> bool {
        matches!(self, Formation::DenLines)
    }

    /// Checks that the formation lines `team_size` goblins up on distinct in-bounds squares of
    /// their own half, on both sides of the bowl.
    ///
    /// # Errors
    /// The first [`FormationError`] met, line by line, then [`FormationError::TooFewSquares`].
    pub fn validate(&self, arena: &Arena, team_size: u8) -> Result<(), FormationError> {
        let Formation::Lines(lines) = self else {
            return Ok(());
        };
        let mut squares = Vec::new();
        for line in lines {
            for across in line.across.iter().copied() {
                if line.depth == 0 {
                    return Err(FormationError::OnTheCentreColumn { across });
                }
                let square = home_square(arena, line.depth, across)
                    .filter(|square| {
                        arena.is_in_bounds(*square)
                            && arena.is_in_bounds(mirrored_east_west(arena, *square))
                    })
                    .ok_or(FormationError::OffTheBowl {
                        depth: line.depth,
                        across,
                    })?;
                if squares.contains(&square) {
                    return Err(FormationError::SquareNamedTwice {
                        depth: line.depth,
                        across,
                    });
                }
                squares.push(square);
            }
        }
        if squares.len() < usize::from(team_size) {
            return Err(FormationError::TooFewSquares {
                squares: squares.len(),
                team_size,
            });
        }
        Ok(())
    }
}

/// Kick-off square of every goblin of `team`, in shirt-number order.
///
/// Home stands in the west half; away mirrors it in the east half.
pub fn kickoff_squares(
    arena: &Arena,
    formation: &Formation,
    team: TeamId,
    roster: &Roster,
) -> Vec<Square> {
    let home_side_squares = match formation {
        Formation::DenLines => den_lines(arena, team, roster),
        Formation::Lines(lines) => formation_lines(arena, lines, roster),
    };
    if team == TeamId::HOME {
        home_side_squares
    } else {
        home_side_squares
            .into_iter()
            .map(|square| mirrored_east_west(arena, square))
            .collect()
    }
}

fn den_lines(arena: &Arena, team: TeamId, roster: &Roster) -> Vec<Square> {
    let den_rows: Vec<u8> = arena.den_squares(team).map(|square| square.y).collect();
    let front_column = (arena.width().saturating_sub(1) / 2).saturating_sub(1);
    Archetype::ALL
        .into_iter()
        .zip([0_u8, 2, 4])
        .flat_map(|(archetype, columns_behind_front)| {
            spread_on_lines(
                front_column.saturating_sub(columns_behind_front),
                roster.count(archetype),
                &den_rows,
            )
        })
        .collect()
}

fn spread_on_lines(first_column: u8, goblin_count: u8, rows: &[u8]) -> Vec<Square> {
    if rows.is_empty() {
        return Vec::new();
    }
    let per_line = rows.len();
    let total = usize::from(goblin_count);
    (0..total)
        .map(|index| {
            let line = index / per_line;
            let on_this_line = (total - line * per_line).min(per_line);
            let position = index % per_line;
            let row_index = (2 * position + 1) * per_line / (2 * on_this_line);
            let column = first_column.saturating_sub(u8::try_from(line).unwrap_or(u8::MAX));
            Square::new(column, rows[row_index.min(per_line - 1)])
        })
        .collect()
}

fn formation_lines(arena: &Arena, lines: &[FormationLine], roster: &Roster) -> Vec<Square> {
    let archetypes = roster.archetypes_by_number();
    let mut placed: Vec<Option<Square>> = vec![None; archetypes.len()];
    for line in lines {
        for across in line.across.iter().copied() {
            let Some(square) = home_square(arena, line.depth, across) else {
                continue;
            };
            let next_goblin = line
                .prefers
                .iter()
                .chain(Archetype::ALL.iter())
                .find_map(|wanted| {
                    (0..archetypes.len())
                        .find(|number| placed[*number].is_none() && archetypes[*number] == *wanted)
                });
            if let Some(number) = next_goblin {
                placed[number] = Some(square);
            }
        }
    }
    placed.into_iter().flatten().collect()
}

fn home_square(arena: &Arena, depth: u8, across: i8) -> Option<Square> {
    let centre = arena.centre();
    let x = centre.x.checked_sub(depth)?;
    let y = u8::try_from(i16::from(centre.y) + i16::from(across)).ok()?;
    Some(Square::new(x, y))
}

fn mirrored_east_west(arena: &Arena, square: Square) -> Square {
    Square::new(
        arena.width().saturating_sub(1).saturating_sub(square.x),
        square.y,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::{BOWL_12_MASK, BOWL_21_MASK};
    use crate::characteristics::Archetype::{Bruiser, Sprinter, Thrower};
    use std::collections::HashSet;

    fn bowl_21() -> Arena {
        Arena::from_mask(&BOWL_21_MASK).unwrap()
    }

    fn line(depth: u8, across: &[i8], prefers: &[Archetype]) -> FormationLine {
        FormationLine {
            depth,
            across: across.to_vec(),
            prefers: prefers.to_vec(),
        }
    }

    fn scrimmage() -> Formation {
        Formation::Lines(vec![
            line(1, &[-1, 0, 1], &[Bruiser, Sprinter, Thrower]),
            line(3, &[-5, -4, 4, 5], &[Sprinter, Bruiser, Thrower]),
            line(8, &[-2, 2], &[Thrower, Bruiser, Sprinter]),
        ])
    }

    fn home_in_scrimmage() -> Vec<Square> {
        kickoff_squares(&bowl_21(), &scrimmage(), TeamId::HOME, &Roster::DEFAULT)
    }

    #[test]
    fn given_the_scrimmage_formation_when_home_kicks_off_then_three_bruisers_hold_the_line_by_the_centre()
     {
        assert_eq!(
            home_in_scrimmage()[..3],
            [Square::new(9, 9), Square::new(9, 10), Square::new(9, 11)]
        );
    }

    #[test]
    fn given_the_scrimmage_formation_when_home_kicks_off_then_the_sprinters_take_the_wings() {
        assert_eq!(
            home_in_scrimmage()[4..8],
            [
                Square::new(7, 5),
                Square::new(7, 6),
                Square::new(7, 14),
                Square::new(7, 15)
            ]
        );
    }

    #[test]
    fn given_the_scrimmage_formation_when_home_kicks_off_then_the_thrower_and_the_fourth_bruiser_stand_back()
     {
        let squares = home_in_scrimmage();
        assert_eq!(
            (squares[8], squares[3]),
            (Square::new(2, 8), Square::new(2, 12))
        );
    }

    #[test]
    fn given_the_scrimmage_formation_when_away_kicks_off_then_it_mirrors_home() {
        let mirrored: Vec<Square> = home_in_scrimmage()
            .into_iter()
            .map(|square| Square::new(20 - square.x, square.y))
            .collect();
        assert_eq!(
            kickoff_squares(&bowl_21(), &scrimmage(), TeamId::AWAY, &Roster::DEFAULT),
            mirrored
        );
    }

    #[test]
    fn given_a_roster_without_sprinters_when_lining_up_in_scrimmage_then_bruisers_take_the_wings() {
        let roster = Roster {
            bruisers: 8,
            sprinters: 0,
            throwers: 1,
        };
        let squares = kickoff_squares(&bowl_21(), &scrimmage(), TeamId::HOME, &roster);
        assert_eq!(
            squares[3..7],
            [
                Square::new(7, 5),
                Square::new(7, 6),
                Square::new(7, 14),
                Square::new(7, 15)
            ]
        );
    }

    #[test]
    fn given_the_scrimmage_formation_on_the_large_bowl_when_validated_then_accepted() {
        assert_eq!(scrimmage().validate(&bowl_21(), 9), Ok(()));
    }

    #[test]
    fn given_the_scrimmage_formation_on_the_regular_bowl_when_validated_then_its_back_line_is_off_the_bowl()
     {
        assert_eq!(
            scrimmage().validate(&Arena::bowl_15(), 9),
            Err(FormationError::OffTheBowl {
                depth: 8,
                across: -2
            })
        );
    }

    #[test]
    fn given_eight_squares_for_nine_goblins_when_validated_then_too_few_squares() {
        let short = Formation::Lines(vec![line(1, &[-4, -3, -2, -1, 1, 2, 3, 4], &[])]);
        assert_eq!(
            short.validate(&bowl_21(), 9),
            Err(FormationError::TooFewSquares {
                squares: 8,
                team_size: 9
            })
        );
    }

    #[test]
    fn given_a_line_on_the_centre_column_when_validated_then_refused() {
        let on_the_centre = Formation::Lines(vec![line(0, &[0], &[])]);
        assert_eq!(
            on_the_centre.validate(&bowl_21(), 1),
            Err(FormationError::OnTheCentreColumn { across: 0 })
        );
    }

    #[test]
    fn given_a_square_named_twice_when_validated_then_refused() {
        let doubled = Formation::Lines(vec![line(1, &[0], &[]), line(1, &[0], &[])]);
        assert_eq!(
            doubled.validate(&bowl_21(), 2),
            Err(FormationError::SquareNamedTwice {
                depth: 1,
                across: 0
            })
        );
    }

    fn home_default() -> Vec<Square> {
        kickoff_squares(
            &Arena::bowl_15(),
            &Formation::DenLines,
            TeamId::HOME,
            &Roster::DEFAULT,
        )
    }

    #[test]
    fn given_default_roster_when_home_kicks_off_then_one_square_per_goblin() {
        assert_eq!(home_default().len(), 9);
    }

    #[test]
    fn given_default_roster_when_home_kicks_off_then_bruisers_hold_column_six() {
        assert!(home_default()[..4].iter().all(|square| square.x == 6));
    }

    #[test]
    fn given_default_roster_when_home_kicks_off_then_bruisers_spread_on_rows_4_6_8_10() {
        let rows: Vec<u8> = home_default()[..4].iter().map(|square| square.y).collect();
        assert_eq!(rows, vec![4, 6, 8, 10]);
    }

    #[test]
    fn given_default_roster_when_home_kicks_off_then_sprinters_hold_column_four() {
        assert!(home_default()[4..8].iter().all(|square| square.x == 4));
    }

    #[test]
    fn given_default_roster_when_home_kicks_off_then_the_thrower_stands_back_in_the_middle() {
        assert_eq!(home_default()[8], Square::new(2, 7));
    }

    #[test]
    fn given_default_roster_when_away_kicks_off_then_it_mirrors_home() {
        let mirrored: Vec<Square> = home_default()
            .into_iter()
            .map(|square| Square::new(14 - square.x, square.y))
            .collect();
        assert_eq!(
            kickoff_squares(
                &Arena::bowl_15(),
                &Formation::DenLines,
                TeamId::AWAY,
                &Roster::DEFAULT
            ),
            mirrored
        );
    }

    #[test]
    fn given_default_roster_when_home_kicks_off_then_nobody_crosses_the_middle() {
        assert!(home_default().iter().all(|square| square.x < 7));
    }

    #[test]
    fn given_eight_bruisers_on_the_compact_bowl_when_kicking_off_then_no_square_is_shared() {
        let arena = Arena::from_mask(&BOWL_12_MASK).unwrap();
        let roster = Roster {
            bruisers: 8,
            sprinters: 0,
            throwers: 1,
        };
        let squares = kickoff_squares(&arena, &Formation::DenLines, TeamId::HOME, &roster);
        let distinct: HashSet<Square> = squares.iter().copied().collect();
        assert_eq!(distinct.len(), squares.len());
    }

    #[test]
    fn given_eight_bruisers_on_the_compact_bowl_when_kicking_off_then_every_square_is_in_bounds() {
        let arena = Arena::from_mask(&BOWL_12_MASK).unwrap();
        let roster = Roster {
            bruisers: 8,
            sprinters: 0,
            throwers: 1,
        };
        assert!(
            kickoff_squares(&arena, &Formation::DenLines, TeamId::HOME, &roster)
                .iter()
                .all(|square| arena.is_in_bounds(*square))
        );
    }
}
