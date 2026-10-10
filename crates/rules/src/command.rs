//! Commands coaches send to the engine.

use serde::{Deserialize, Serialize};

use crate::roster::GoblinId;
use crate::square::Square;

/// What a goblin does once it has moved; doing anything ends its activation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    /// Throw the ball to a standing teammate, at any distance.
    Throw {
        /// The teammate who should catch it.
        receiver: GoblinId,
    },
}

/// An order for one goblin: the squares it steps into, in order (not counting its starting square),
/// then an optional action.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Activation {
    /// The goblin to activate.
    pub goblin: GoblinId,
    /// Squares stepped into, each orthogonally next to the previous one.
    pub path: Vec<Square>,
    /// What the goblin does at the end of its path.
    #[serde(default)]
    pub action: Option<Action>,
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
