//! Why the engine refuses a command; a refused command leaves the match untouched.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::roster::GoblinId;
use crate::square::Square;
use crate::team::TeamId;

/// A command that breaks the rules.
#[derive(Clone, Debug, Error, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "violation", rename_all = "snake_case")]
pub enum RuleViolation {
    /// The final whistle already went.
    #[error("the match is over")]
    MatchFinished,
    /// The other team is playing.
    #[error("it is not your turn: team {} is playing", active_team.letter())]
    NotYourTurn {
        /// Team allowed to act.
        active_team: TeamId,
    },
    /// No goblin has this id.
    #[error("there is no goblin {goblin}")]
    UnknownGoblin {
        /// The unknown id.
        goblin: GoblinId,
    },
    /// The goblin plays for the other team.
    #[error("{goblin} plays for the other team")]
    NotYourGoblin {
        /// The goblin.
        goblin: GoblinId,
    },
    /// The goblin is down or knocked out.
    #[error("{goblin} is not standing on the bowl")]
    NotStanding {
        /// The goblin.
        goblin: GoblinId,
    },
    /// The goblin already used its activation this turn.
    #[error("{goblin} already acted this turn")]
    AlreadyActed {
        /// The goblin.
        goblin: GoblinId,
    },
    /// The path has more steps than the goblin's movement.
    #[error("{goblin} can move {movement} squares, the path has {steps}")]
    PathTooLong {
        /// The goblin.
        goblin: GoblinId,
        /// Its movement.
        movement: u8,
        /// Steps in the path.
        steps: usize,
    },
    /// A step is not orthogonally next to the previous square.
    #[error("step {index} to [{}, {}] is not next to the previous square", square.x, square.y)]
    PathNotContiguous {
        /// Zero-based index of the step in the path.
        index: usize,
        /// The square of that step.
        square: Square,
    },
    /// A step leaves the bowl.
    #[error("[{}, {}] is outside the bowl", square.x, square.y)]
    PathLeavesTheBowl {
        /// The out-of-bounds square.
        square: Square,
    },
    /// A step enters a square already visited by this path.
    #[error("the path comes back to [{}, {}]", square.x, square.y)]
    PathRevisitsSquare {
        /// The revisited square.
        square: Square,
    },
    /// A step enters a square holding a goblin, standing or down.
    #[error("[{}, {}] is occupied", square.x, square.y)]
    PathBlocked {
        /// The occupied square.
        square: Square,
    },
}
