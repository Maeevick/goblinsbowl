//! Everything that happens in a match, reported in order so shells can animate, log and replay it.

use serde::{Deserialize, Serialize};

use crate::roster::GoblinId;
use crate::square::Square;
use crate::team::TeamId;

/// A ball roll (Pick, Throw, Catch, Intercept): the die minus its maluses against the target.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BallRoll {
    /// Faces of the die (the goblin's agility).
    pub faces: u8,
    /// Face shown.
    pub rolled: u8,
    /// Total malus subtracted from the face.
    pub malus: u8,
    /// Value to reach after the malus.
    pub target: u8,
    /// Whether the roll succeeded.
    pub success: bool,
}

/// An opposed roll (Assault, Dodge): the actor's die against the opponent's die plus its modifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContestRoll {
    /// Faces of the actor's die.
    pub actor_faces: u8,
    /// Face shown by the actor's die.
    pub actor_rolled: u8,
    /// Faces of the opponent's defense die.
    pub opponent_faces: u8,
    /// Face shown by the opponent's die.
    pub opponent_rolled: u8,
    /// Added to the opponent's face: Stance walls, minus the malus of a downed defender.
    pub opponent_modifier: i8,
    /// Actor's face minus the opponent's total; the actor wins ties (margin 0).
    pub margin: i8,
}

impl ContestRoll {
    /// Whether the actor won the contest (ties go to the actor).
    pub fn actor_won(&self) -> bool {
        self.margin >= 0
    }
}

/// Why a turn ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnEndReason {
    /// The coach ended it.
    EndTurn,
    /// The turn clock ran out.
    TimeUp,
    /// A goblin of the active team failed an Assault or a Dodge.
    Knockdown,
    /// The active team failed a Pick, Throw or Catch, or its throw was intercepted.
    Fumble,
    /// A team scored.
    Score,
    /// Every goblin of the active team has acted or cannot act.
    NoGoblinCanAct,
}

/// Something that happened in the match.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum GameEvent {
    /// The coin toss picked the team that plays first.
    CoinTossed {
        /// Team that receives the opening kick-off.
        receiving_team: TeamId,
    },
    /// Teams lined up in formation and the ball is about to drop.
    KickedOff {
        /// Team that plays first after this kick-off.
        receiving_team: TeamId,
    },
    /// A knocked-out goblin came back for the kick-off.
    ReturnedFromKnockOut {
        /// The goblin.
        goblin: GoblinId,
        /// Health rolled on its health die.
        health: u8,
    },
    /// The ball dropped on a square.
    BallDropped {
        /// Square it dropped on.
        at: Square,
    },
    /// The ball bounced one square.
    BallBounced {
        /// Square it left.
        from: Square,
        /// Square it reached.
        to: Square,
    },
    /// A goblin tried to catch the ball.
    CatchAttempted {
        /// The catcher.
        goblin: GoblinId,
        /// Its roll.
        roll: BallRoll,
    },
    /// A team's turn began.
    TurnStarted {
        /// The team.
        team: TeamId,
        /// Current half.
        half: u8,
        /// Turn number of this team in the half.
        turn: u8,
    },
    /// A downed goblin got up at the start of its team's turn; it cannot act during this turn.
    StoodUp {
        /// The goblin.
        goblin: GoblinId,
    },
}
