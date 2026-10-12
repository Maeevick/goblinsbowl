//! Goblin archetypes and their characteristics.

use serde::{Deserialize, Serialize};

/// The three kinds of goblins a team can field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Archetype {
    /// The fighter: high attack, defense and health, low agility and movement.
    Bruiser,
    /// The scorer: fast and agile, fragile.
    Sprinter,
    /// The leader: agile ball handler, weak in a fight.
    Thrower,
}

impl Archetype {
    /// All archetypes, in the order goblins are numbered within a team.
    pub const ALL: [Archetype; 3] = [Archetype::Bruiser, Archetype::Sprinter, Archetype::Thrower];
}

/// What a goblin is made of. Attack, defense and agility (and health when returning from a knock-out)
/// are rolled as a die with that many faces; movement counts squares per turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Characteristics {
    /// Faces of the die rolled to assault.
    pub attack: u8,
    /// Faces of the die rolled to resist an assault or to tackle a dodging enemy.
    pub defense: u8,
    /// Faces of the die rolled to dodge, pick, throw, catch and intercept.
    pub agility: u8,
    /// Health pool; the goblin is knocked out when it reaches 0.
    pub health: u8,
    /// Orthogonal steps allowed per turn.
    pub movement: u8,
}

impl Characteristics {
    /// Whether every die characteristic and the health pool have at least one face or point.
    pub fn is_playable(&self) -> bool {
        [self.attack, self.defense, self.agility, self.health]
            .into_iter()
            .all(|value| value > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bruiser() -> Characteristics {
        Characteristics {
            attack: 4,
            defense: 4,
            agility: 2,
            health: 6,
            movement: 3,
        }
    }

    #[test]
    fn given_bruiser_characteristics_when_checked_then_playable() {
        assert!(bruiser().is_playable());
    }

    #[test]
    fn given_zero_faced_attack_when_checked_then_not_playable() {
        let broken = Characteristics {
            attack: 0,
            ..bruiser()
        };
        assert!(!broken.is_playable());
    }

    #[test]
    fn given_zero_health_when_checked_then_not_playable() {
        let broken = Characteristics {
            health: 0,
            ..bruiser()
        };
        assert!(!broken.is_playable());
    }

    #[test]
    fn given_zero_movement_when_checked_then_still_playable() {
        let rooted = Characteristics {
            movement: 0,
            ..bruiser()
        };
        assert!(rooted.is_playable());
    }

    #[test]
    fn given_archetype_when_serialised_then_snake_case_name() {
        assert_eq!(
            serde_json::to_string(&Archetype::Sprinter).unwrap(),
            "\"sprinter\""
        );
    }
}
