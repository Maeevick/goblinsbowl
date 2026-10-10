//! Kick-off formations: where each goblin stands when play starts or restarts.

use crate::arena::Arena;
use crate::characteristics::Archetype;
use crate::roster::Roster;
use crate::square::Square;
use crate::team::TeamId;

/// Kick-off square of every goblin of `team`, in shirt-number order.
///
/// Bruisers hold the front line, Sprinters the line two steps behind, Throwers two steps further,
/// each group spread evenly across the rows of the team's den; a group larger than the den spills
/// onto the next column back. Home stands in the west half, away mirrors it in the east half.
pub fn kickoff_squares(arena: &Arena, team: TeamId, roster: &Roster) -> Vec<Square> {
    let den_rows: Vec<u8> = arena.den_squares(team).map(|square| square.y).collect();
    let front_column = (arena.width().saturating_sub(1) / 2).saturating_sub(1);
    let home_side_squares: Vec<Square> = Archetype::ALL
        .into_iter()
        .zip([0_u8, 2, 4])
        .flat_map(|(archetype, columns_behind_front)| {
            spread_on_lines(
                front_column.saturating_sub(columns_behind_front),
                roster.count(archetype),
                &den_rows,
            )
        })
        .collect();
    if team == TeamId::HOME {
        home_side_squares
    } else {
        home_side_squares
            .into_iter()
            .map(|square| mirrored_east_west(arena, square))
            .collect()
    }
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

fn mirrored_east_west(arena: &Arena, square: Square) -> Square {
    Square::new(
        arena.width().saturating_sub(1).saturating_sub(square.x),
        square.y,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::BOWL_12_MASK;
    use std::collections::HashSet;

    fn home_default() -> Vec<Square> {
        kickoff_squares(&Arena::bowl_15(), TeamId::HOME, &Roster::DEFAULT)
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
            kickoff_squares(&Arena::bowl_15(), TeamId::AWAY, &Roster::DEFAULT),
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
        let squares = kickoff_squares(&arena, TeamId::HOME, &roster);
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
            kickoff_squares(&arena, TeamId::HOME, &roster)
                .iter()
                .all(|square| arena.is_in_bounds(*square))
        );
    }
}
