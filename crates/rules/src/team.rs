//! Team identities.

use serde::{Deserialize, Serialize};

/// Index of a team in a match: 0 is home, 1 is away (more teams later for the brawl mode).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TeamId(pub u8);

impl TeamId {
    /// The home team, defending the west den in the 2-team bowl.
    pub const HOME: TeamId = TeamId(0);
    /// The away team, defending the east den in the 2-team bowl.
    pub const AWAY: TeamId = TeamId(1);
    /// Both teams of a 2-team match, home first.
    pub const DUEL: [TeamId; 2] = [TeamId::HOME, TeamId::AWAY];

    /// Position of the team in per-team lists such as the score.
    pub fn index(self) -> usize {
        usize::from(self.0)
    }

    /// The opposing team in a 2-team match.
    pub fn duel_opponent(self) -> TeamId {
        if self == TeamId::HOME {
            TeamId::AWAY
        } else {
            TeamId::HOME
        }
    }

    /// The letter used for this team in masks, boards and goblin ids: `H` for home, `A` for away.
    pub fn letter(self) -> char {
        if self == TeamId::HOME { 'H' } else { 'A' }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_home_when_asking_duel_opponent_then_away() {
        assert_eq!(TeamId::HOME.duel_opponent(), TeamId::AWAY);
    }

    #[test]
    fn given_away_when_asking_duel_opponent_then_home() {
        assert_eq!(TeamId::AWAY.duel_opponent(), TeamId::HOME);
    }

    #[test]
    fn given_away_when_asking_its_index_then_one() {
        assert_eq!(TeamId::AWAY.index(), 1);
    }

    #[test]
    fn given_home_and_away_when_asking_letters_then_h_and_a() {
        assert_eq!([TeamId::HOME.letter(), TeamId::AWAY.letter()], ['H', 'A']);
    }
}
