//! Team compositions and goblin identities.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::characteristics::Archetype;
use crate::team::TeamId;

/// Identity of a goblin in a match, written `H1` to `H9` for home and `A1` to `A9` for away.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct GoblinId {
    /// Team the goblin plays for.
    pub team: TeamId,
    /// Shirt number within the team, starting at 1.
    pub number: u8,
}

/// Why a text is not a goblin id.
#[derive(Debug, Error, PartialEq, Eq)]
#[error("{0:?} is not a goblin id (expected H1..H9 or A1..A9)")]
pub struct GoblinIdParseError(pub String);

impl GoblinId {
    /// Builds the id of the goblin wearing `number` in `team`.
    pub const fn new(team: TeamId, number: u8) -> Self {
        Self { team, number }
    }
}

impl fmt::Display for GoblinId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}{}", self.team.letter(), self.number)
    }
}

impl FromStr for GoblinId {
    type Err = GoblinIdParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let rejection = || GoblinIdParseError(text.to_string());
        let mut characters = text.trim().chars();
        let team = match characters.next().map(|letter| letter.to_ascii_uppercase()) {
            Some('H') => TeamId::HOME,
            Some('A') => TeamId::AWAY,
            _ => return Err(rejection()),
        };
        let number: u8 = characters.as_str().parse().map_err(|_| rejection())?;
        if number == 0 {
            return Err(rejection());
        }
        Ok(GoblinId::new(team, number))
    }
}

impl TryFrom<String> for GoblinId {
    type Error = GoblinIdParseError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<GoblinId> for String {
    fn from(id: GoblinId) -> Self {
        id.to_string()
    }
}

/// How many goblins of each archetype a team fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Roster {
    /// Number of Bruisers.
    pub bruisers: u8,
    /// Number of Sprinters.
    pub sprinters: u8,
    /// Number of Throwers.
    pub throwers: u8,
}

/// Inclusive bounds on how many goblins of one archetype a roster may hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CountLimit {
    /// Smallest allowed count.
    pub min: u8,
    /// Largest allowed count.
    pub max: u8,
}

/// Bounds per archetype that every roster must respect.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RosterLimits {
    /// Bounds on Bruisers.
    pub bruiser: CountLimit,
    /// Bounds on Sprinters.
    pub sprinter: CountLimit,
    /// Bounds on Throwers.
    pub thrower: CountLimit,
}

/// Why a roster cannot take the field.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum RosterError {
    /// The roster does not hold exactly the team size.
    #[error("a team fields {expected} goblins, this roster has {found}")]
    WrongSize {
        /// Team size required by the ruleset.
        expected: u8,
        /// Goblins in the roster.
        found: u8,
    },
    /// An archetype count is outside its bounds.
    #[error("{archetype:?} count {found} is outside {min}..={max}")]
    ArchetypeOutOfBounds {
        /// The archetype whose count is wrong.
        archetype: Archetype,
        /// Smallest allowed count.
        min: u8,
        /// Largest allowed count.
        max: u8,
        /// Count in the roster.
        found: u8,
    },
}

impl Roster {
    /// The default 3 Bruisers, 4 Sprinters, 2 Throwers composition.
    pub const DEFAULT: Roster = Roster {
        bruisers: 3,
        sprinters: 4,
        throwers: 2,
    };

    /// The roster counting `archetypes`, one goblin each.
    pub fn of(archetypes: impl IntoIterator<Item = Archetype>) -> Roster {
        archetypes.into_iter().fold(
            Roster {
                bruisers: 0,
                sprinters: 0,
                throwers: 0,
            },
            |mut roster, archetype| {
                let count = match archetype {
                    Archetype::Bruiser => &mut roster.bruisers,
                    Archetype::Sprinter => &mut roster.sprinters,
                    Archetype::Thrower => &mut roster.throwers,
                };
                *count = count.saturating_add(1);
                roster
            },
        )
    }

    /// Total number of goblins.
    pub fn size(&self) -> u8 {
        self.bruisers
            .saturating_add(self.sprinters)
            .saturating_add(self.throwers)
    }

    /// How many goblins of `archetype` the roster holds.
    pub fn count(&self, archetype: Archetype) -> u8 {
        match archetype {
            Archetype::Bruiser => self.bruisers,
            Archetype::Sprinter => self.sprinters,
            Archetype::Thrower => self.throwers,
        }
    }

    /// Archetype of every goblin, in shirt-number order: Bruisers, then Sprinters, then Throwers.
    pub fn archetypes_by_number(&self) -> Vec<Archetype> {
        Archetype::ALL
            .into_iter()
            .flat_map(|archetype| {
                std::iter::repeat_n(archetype, usize::from(self.count(archetype)))
            })
            .collect()
    }

    /// Checks the roster against the team size and the per-archetype limits.
    ///
    /// # Errors
    /// [`RosterError::WrongSize`] first, then the first archetype out of bounds.
    pub fn validate(&self, team_size: u8, limits: &RosterLimits) -> Result<(), RosterError> {
        if self.size() != team_size {
            return Err(RosterError::WrongSize {
                expected: team_size,
                found: self.size(),
            });
        }
        Archetype::ALL.into_iter().try_for_each(|archetype| {
            let limit = limits.for_archetype(archetype);
            let found = self.count(archetype);
            if (limit.min..=limit.max).contains(&found) {
                Ok(())
            } else {
                Err(RosterError::ArchetypeOutOfBounds {
                    archetype,
                    min: limit.min,
                    max: limit.max,
                    found,
                })
            }
        })
    }
}

impl RosterLimits {
    /// Bounds for `archetype`.
    pub fn for_archetype(&self, archetype: Archetype) -> CountLimit {
        match archetype {
            Archetype::Bruiser => self.bruiser,
            Archetype::Sprinter => self.sprinter,
            Archetype::Thrower => self.thrower,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIMITS: RosterLimits = RosterLimits {
        bruiser: CountLimit { min: 0, max: 8 },
        sprinter: CountLimit { min: 0, max: 8 },
        thrower: CountLimit { min: 1, max: 2 },
    };

    #[test]
    fn given_home_goblin_three_when_written_then_h3() {
        assert_eq!(GoblinId::new(TeamId::HOME, 3).to_string(), "H3");
    }

    #[test]
    fn given_text_a9_when_parsed_then_away_goblin_nine() {
        assert_eq!("A9".parse(), Ok(GoblinId::new(TeamId::AWAY, 9)));
    }

    #[test]
    fn given_lowercase_text_when_parsed_then_accepted() {
        assert_eq!("h2".parse(), Ok(GoblinId::new(TeamId::HOME, 2)));
    }

    #[test]
    fn given_square_like_text_when_parsed_then_rejected() {
        assert!("B4".parse::<GoblinId>().is_err());
    }

    #[test]
    fn given_number_zero_when_parsed_then_rejected() {
        assert!("H0".parse::<GoblinId>().is_err());
    }

    #[test]
    fn given_goblin_id_when_serialised_then_json_string() {
        assert_eq!(
            serde_json::to_string(&GoblinId::new(TeamId::AWAY, 1)).unwrap(),
            "\"A1\""
        );
    }

    #[test]
    fn given_two_bruisers_and_a_thrower_when_making_their_roster_then_two_none_and_one() {
        assert_eq!(
            Roster::of([Archetype::Bruiser, Archetype::Thrower, Archetype::Bruiser]),
            Roster {
                bruisers: 2,
                sprinters: 0,
                throwers: 1
            }
        );
    }

    #[test]
    fn given_default_roster_when_counting_then_nine_goblins() {
        assert_eq!(Roster::DEFAULT.size(), 9);
    }

    #[test]
    fn given_default_roster_when_counting_sprinters_then_four() {
        assert_eq!(Roster::DEFAULT.count(Archetype::Sprinter), 4);
    }

    #[test]
    fn given_roster_when_listing_by_number_then_bruisers_sprinters_throwers() {
        let roster = Roster {
            bruisers: 1,
            sprinters: 2,
            throwers: 1,
        };
        assert_eq!(
            roster.archetypes_by_number(),
            vec![
                Archetype::Bruiser,
                Archetype::Sprinter,
                Archetype::Sprinter,
                Archetype::Thrower
            ]
        );
    }

    #[test]
    fn given_default_roster_when_validated_then_accepted() {
        assert_eq!(Roster::DEFAULT.validate(9, &LIMITS), Ok(()));
    }

    #[test]
    fn given_eight_goblin_roster_when_validated_then_wrong_size() {
        let roster = Roster {
            bruisers: 4,
            sprinters: 3,
            throwers: 1,
        };
        assert_eq!(
            roster.validate(9, &LIMITS),
            Err(RosterError::WrongSize {
                expected: 9,
                found: 8
            })
        );
    }

    #[test]
    fn given_roster_without_thrower_when_validated_then_thrower_out_of_bounds() {
        let roster = Roster {
            bruisers: 5,
            sprinters: 4,
            throwers: 0,
        };
        assert_eq!(
            roster.validate(9, &LIMITS),
            Err(RosterError::ArchetypeOutOfBounds {
                archetype: Archetype::Thrower,
                min: 1,
                max: 2,
                found: 0
            })
        );
    }
}
