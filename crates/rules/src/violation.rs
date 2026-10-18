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
    /// The path has more steps than the goblin has left this turn.
    #[error("{goblin} has {steps_left} steps left this turn, the path has {steps}")]
    PathTooLong {
        /// The goblin.
        goblin: GoblinId,
        /// Steps it may still take this turn.
        steps_left: u8,
        /// Steps in the path.
        steps: usize,
    },
    /// Actions cost a step, and the path would leave the goblin none for its action.
    #[error("{goblin} has {steps_left} steps left this turn: a path of {steps} leaves none to act")]
    NoStepLeftToAct {
        /// The goblin.
        goblin: GoblinId,
        /// Steps it may still take this turn.
        steps_left: u8,
        /// Steps in the path.
        steps: usize,
    },
    /// The goblin already took its action this turn, or this kind of action where each kind comes once.
    #[error("{goblin} already used this action this turn")]
    ActionAlreadyUsed {
        /// The goblin.
        goblin: GoblinId,
    },
    /// The rules only have simple assaults and throws.
    #[error("these rules only have simple assaults and throws: {goblin} cannot use another style")]
    StyleNotInTheseRules {
        /// The goblin.
        goblin: GoblinId,
    },
    /// A projection throws a standing enemy down; this one already lies down.
    #[error("{target} already lies down and cannot be projected")]
    ProjectionNeedsAStandingTarget {
        /// The target.
        target: GoblinId,
    },
    /// A projection must land its target on a free square of the bowl around the attacker.
    #[error("{target} cannot be projected that way: the square is off the bowl or taken")]
    ProjectionLandingUnavailable {
        /// The target.
        target: GoblinId,
    },
    /// The goblin would hold the ball, and the rules keep carriers from assaulting.
    #[error("{goblin} holds the ball and cannot assault")]
    CarrierCannotAssault {
        /// The carrier.
        goblin: GoblinId,
    },
    /// The rules have no Stance to brace in.
    #[error("these rules have no Stance: {goblin} cannot brace")]
    NoStanceInTheseRules {
        /// The goblin.
        goblin: GoblinId,
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
    /// The thrower would not hold the ball at the end of its path.
    #[error("{goblin} does not hold the ball when it would throw")]
    ThrowWithoutBall {
        /// The would-be thrower.
        goblin: GoblinId,
    },
    /// The receiver is not a standing teammate of the thrower.
    #[error("{receiver} cannot receive: it must be a standing teammate of the thrower")]
    ReceiverUnavailable {
        /// The refused receiver.
        receiver: GoblinId,
    },
    /// The assault target plays for the same team.
    #[error("{target} is not an opponent")]
    TargetNotAnOpponent {
        /// The refused target.
        target: GoblinId,
    },
    /// The assault target is knocked out.
    #[error("{target} is not on the bowl")]
    TargetNotOnTheBowl {
        /// The refused target.
        target: GoblinId,
    },
    /// The assault target is not next to the square where the attacker ends its move.
    #[error("{target} is not next to where the attacker ends its move")]
    TargetNotAdjacent {
        /// The refused target.
        target: GoblinId,
    },
}
