//! Commands coaches send to the engine.

use serde::{Deserialize, Serialize};

use crate::roster::GoblinId;
use crate::square::Square;

/// What a goblin does once it has moved.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    /// Assault an adjacent enemy, standing or down.
    Assault {
        /// The enemy to hit.
        target: GoblinId,
        /// How the assault is thrown; simple when left out.
        #[serde(default, skip_serializing_if = "AssaultStyle::is_simple")]
        style: AssaultStyle,
    },
    /// Brace until the team's next turn: +1 defense to itself and to each adjacent teammate.
    Stance,
    /// Throw the ball to a standing teammate, at any distance.
    Throw {
        /// The teammate who should catch it.
        receiver: GoblinId,
        /// How the ball is thrown; simple when left out.
        #[serde(default, skip_serializing_if = "ThrowStyle::is_simple")]
        style: ThrowStyle,
    },
}

impl Action {
    /// A simple assault on `target`.
    pub fn assault(target: GoblinId) -> Action {
        Action::Assault {
            target,
            style: AssaultStyle::Simple,
        }
    }

    /// A simple throw to `receiver`.
    pub fn throw(receiver: GoblinId) -> Action {
        Action::Throw {
            receiver,
            style: ThrowStyle::Simple,
        }
    }
}

/// How an assault is thrown: its modifier decides who wins, and the Health a won assault takes
/// counts the dice without it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssaultStyle {
    /// Attack against Defense, as it comes.
    #[default]
    Simple,
    /// +1 to the attack roll, and the target loses no Health.
    Chirurgical,
    /// −1 to the attack roll, and the target loses one more Health.
    Brutal,
}

impl AssaultStyle {
    /// Whether it is the plain assault.
    pub fn is_simple(&self) -> bool {
        *self == AssaultStyle::Simple
    }

    /// What the style adds to the attack roll.
    pub fn attack_modifier(self) -> i8 {
        match self {
            AssaultStyle::Simple => 0,
            AssaultStyle::Chirurgical => 1,
            AssaultStyle::Brutal => -1,
        }
    }

    /// Health a won assault takes from its target, from the margin the dice show without the
    /// style's modifier: that margin, none when chirurgical, one more when brutal.
    pub fn damage(self, margin_as_rolled: i8) -> u8 {
        let margin = u8::try_from(margin_as_rolled.max(0)).unwrap_or_default();
        match self {
            AssaultStyle::Simple => margin,
            AssaultStyle::Chirurgical => 0,
            AssaultStyle::Brutal => margin.saturating_add(1),
        }
    }
}

/// How the ball is thrown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThrowStyle {
    /// A throw roll, then the catch.
    #[default]
    Simple,
}

impl ThrowStyle {
    /// Whether it is the plain throw.
    pub fn is_simple(&self) -> bool {
        *self == ThrowStyle::Simple
    }
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
