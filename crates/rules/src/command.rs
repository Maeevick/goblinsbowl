//! Commands coaches send to the engine.

use serde::{Deserialize, Serialize};

use crate::roster::GoblinId;
use crate::square::Square;

/// An order for one goblin: the squares it steps into, in order, not counting its starting square.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Activation {
    /// The goblin to activate.
    pub goblin: GoblinId,
    /// Squares stepped into, each orthogonally next to the previous one.
    pub path: Vec<Square>,
}

/// What a coach can ask the engine to do during its turn.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum Command {
    /// Activate one goblin.
    Act(Activation),
    /// Hand the turn over to the other team.
    EndTurn,
    /// The turn clock ran out (sent by the shell that owns the clock).
    TimeUp,
}
