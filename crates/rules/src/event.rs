//! Everything that happens in a match, reported in order so shells can animate, log and replay it.

use serde::{Deserialize, Serialize};

use crate::roster::GoblinId;
use crate::square::Square;
use crate::team::TeamId;

/// An agility test (Pick, Throw, Catch, Intercept, Dodge): the die minus its maluses against the target.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgilityRoll {
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

/// An opposed roll (Assault, Dodge): the actor's die plus its modifier against the opponent's die
/// plus its modifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContestRoll {
    /// Faces of the actor's die.
    pub actor_faces: u8,
    /// Face shown by the actor's die.
    pub actor_rolled: u8,
    /// Added to the actor's face: +1 for a chirurgical assault, −1 for a brutal one.
    #[serde(default, skip_serializing_if = "crate::odds::is_zero")]
    pub actor_modifier: i8,
    /// Faces of the opponent's defense die.
    pub opponent_faces: u8,
    /// Face shown by the opponent's die.
    pub opponent_rolled: u8,
    /// Added to the opponent's face: Stance walls, minus the malus of a downed defender.
    pub opponent_modifier: i8,
    /// Actor's total minus the opponent's total; the actor wins ties (margin 0).
    pub margin: i8,
}

impl ContestRoll {
    /// Whether the actor won the contest (ties go to the actor).
    pub fn actor_won(&self) -> bool {
        self.margin >= 0
    }
}

/// How a dodge was rolled, as the ruleset decides.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "test", rename_all = "snake_case")]
pub enum DodgeRoll {
    /// Agility die against the Defense die and Stance bonus of the enemy most likely to stop the goblin.
    Contest {
        /// That enemy.
        tackler: GoblinId,
        /// The opposed roll.
        contest: ContestRoll,
    },
    /// Agility test, each adjacent standing enemy a malus of one, two when it stands in Stance.
    Agility(AgilityRoll),
}

impl DodgeRoll {
    /// Whether the goblin slipped away.
    pub fn succeeded(&self) -> bool {
        match self {
            DodgeRoll::Contest { contest, .. } => contest.actor_won(),
            DodgeRoll::Agility(roll) => roll.success,
        }
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
    /// A knocked-out goblin rolled no Health back at the kick-off and stays out until the next one.
    StillKnockedOut {
        /// The goblin.
        goblin: GoblinId,
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
        roll: AgilityRoll,
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
    /// A goblin stepped from one square to the next.
    Moved {
        /// The goblin.
        goblin: GoblinId,
        /// Square it left.
        from: Square,
        /// Square it entered.
        to: Square,
    },
    /// A goblin tried to slip out of the reach of adjacent enemies.
    DodgeAttempted {
        /// The dodging goblin.
        goblin: GoblinId,
        /// Its roll.
        roll: DodgeRoll,
    },
    /// A goblin tried to pick up the loose ball it stepped onto.
    PickAttempted {
        /// The goblin.
        goblin: GoblinId,
        /// Its roll.
        roll: AgilityRoll,
    },
    /// A goblin threw the ball to a teammate.
    ThrowAttempted {
        /// The thrower.
        thrower: GoblinId,
        /// The intended receiver.
        receiver: GoblinId,
        /// The throw roll (adjacent enemies and distance beyond the free range as malus).
        roll: AgilityRoll,
        /// Orthogonal steps between the thrower and the receiver.
        distance: u8,
    },
    /// A goblin handed the ball to an adjacent teammate, without a throw roll.
    HandedOff {
        /// The goblin giving the ball.
        giver: GoblinId,
        /// The teammate who must catch it.
        receiver: GoblinId,
    },
    /// An opponent standing on the flight line tried to snatch the ball.
    InterceptAttempted {
        /// The would-be interceptor.
        goblin: GoblinId,
        /// Its catch roll, with the surprise malus.
        roll: AgilityRoll,
    },
    /// A goblin assaulted an adjacent enemy.
    AssaultResolved {
        /// The attacker.
        attacker: GoblinId,
        /// The defender.
        defender: GoblinId,
        /// Attack die against the defense die, Stance walls and the downed malus.
        contest: ContestRoll,
        /// Whether a Stance bonus, its own or a braced neighbour's, raised the defender's roll.
        defender_braced: bool,
    },
    /// An assault tied under rules where ties stand off: nobody fell and the turn goes on.
    StandOff {
        /// The attacker.
        attacker: GoblinId,
        /// The defender.
        defender: GoblinId,
    },
    /// A goblin braced in Stance until its team's next turn.
    StanceTaken {
        /// The goblin.
        goblin: GoblinId,
    },
    /// A goblin fell or was knocked over on a square.
    KnockedDown {
        /// The goblin.
        goblin: GoblinId,
        /// Square where it lies.
        at: Square,
    },
    /// A goblin lost health.
    HealthLost {
        /// The goblin.
        goblin: GoblinId,
        /// Points lost.
        amount: u8,
        /// Points left.
        remaining: u8,
    },
    /// A goblin reached 0 health and left the bowl until the next kick-off.
    KnockedOut {
        /// The goblin.
        goblin: GoblinId,
    },
    /// A team scored: one of its standing goblins held the ball in an enemy den.
    Scored {
        /// The scoring team.
        team: TeamId,
        /// The goblin holding the ball.
        scorer: GoblinId,
    },
    /// A team's turn is over.
    TurnEnded {
        /// The team whose turn ended.
        team: TeamId,
        /// Why it ended.
        reason: TurnEndReason,
    },
    /// Both teams used all their turns of the half.
    HalfEnded {
        /// The half that just ended.
        half: u8,
    },
    /// The final whistle went.
    GameEnded {
        /// Final points per team.
        score: Vec<u8>,
    },
}
