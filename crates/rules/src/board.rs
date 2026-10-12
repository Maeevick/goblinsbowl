//! A coordinate-labelled text picture of the bowl, for tests, logs and agents.
//!
//! Each square takes three characters: `   ` out of bounds, ` . ` field, ` : ` den,
//! ` o ` loose ball, `H1 ` standing goblin, `h1 ` downed goblin, `H1*` carrier, `H1#` braced goblin.

use crate::arena::Arena;
use crate::square::Square;
use crate::state::{Ball, GameState, Goblin, Presence};

/// The board as text lines: a header of column numbers, then one line per row, prefixed by its number.
pub fn render_board(arena: &Arena, state: &GameState) -> Vec<String> {
    let header = std::iter::once(
        (0..arena.width())
            .map(|x| format!("{x:>2} "))
            .fold(String::from("   "), |line, cell| line + &cell)
            .trim_end()
            .to_string(),
    );
    let rows = (0..arena.height()).map(|y| {
        let cells: String = (0..arena.width())
            .map(|x| square_cell(arena, state, Square::new(x, y)))
            .collect();
        format!("{y:>2} {cells}").trim_end().to_string()
    });
    header.chain(rows).collect()
}

fn square_cell(arena: &Arena, state: &GameState, square: Square) -> String {
    if !arena.is_in_bounds(square) {
        return "   ".to_string();
    }
    if let Some(goblin) = state.goblin_at(square) {
        return goblin_cell(state, goblin);
    }
    if state.ball == (Ball::Loose { at: square }) {
        return " o ".to_string();
    }
    if arena.den_owner(square).is_some() {
        " : ".to_string()
    } else {
        " . ".to_string()
    }
}

fn goblin_cell(state: &GameState, goblin: &Goblin) -> String {
    let id = goblin.id.to_string();
    let label = if matches!(goblin.presence, Presence::Down(_)) {
        id.to_lowercase()
    } else {
        id
    };
    let marker = if state.carrier() == Some(goblin.id) {
        '*'
    } else if goblin.in_stance {
        '#'
    } else {
        ' '
    };
    format!("{label:<2}{marker}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dice::ScriptedDice;
    use crate::game::Game;
    use crate::roster::Roster;
    use crate::ruleset::Ruleset;

    fn kick_off_board() -> Vec<String> {
        let (game, _) = Game::start(
            Ruleset::default(),
            [Roster::DEFAULT, Roster::DEFAULT],
            ScriptedDice::new([1, 1]),
        )
        .unwrap();
        render_board(game.arena(), game.state())
    }

    #[test]
    fn given_the_opening_kick_off_when_rendered_then_it_matches_the_snapshot() {
        insta::assert_snapshot!(kick_off_board().join("\n"));
    }

    #[test]
    fn given_the_shipped_bowl_when_rendered_then_a_header_and_twenty_one_rows() {
        assert_eq!(kick_off_board().len(), 22);
    }

    #[test]
    fn given_a_goblin_lying_down_when_rendered_then_its_id_is_lowercase() {
        let (mut game, _) = Game::start(
            Ruleset::default(),
            [Roster::DEFAULT, Roster::DEFAULT],
            ScriptedDice::new([1, 1]),
        )
        .unwrap();
        let mut state = game.state().clone();
        if let Some(goblin) = state.goblins.first_mut() {
            goblin.presence = Presence::Down(Square::new(6, 4));
        }
        game = Game::resume(Ruleset::default(), state, ScriptedDice::new([])).unwrap();
        let row_four = render_board(game.arena(), game.state())[5].clone();
        assert!(row_four.contains("h1 "));
    }
}
