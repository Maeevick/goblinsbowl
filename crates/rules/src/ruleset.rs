//! Every tunable number of a match, as loaded from `rules.toml`.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::arena::{Arena, ArenaError, BOWL_21_MASK};
use crate::characteristics::{Archetype, Characteristics};
use crate::formation::{Formation, FormationError, FormationLine};
use crate::roster::{CountLimit, Roster, RosterError, RosterLimits};

/// Characteristics of each archetype.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchetypeCharacteristics {
    /// The fighter.
    pub bruiser: Characteristics,
    /// The scorer.
    pub sprinter: Characteristics,
    /// The leader.
    pub thrower: Characteristics,
}

impl ArchetypeCharacteristics {
    /// Characteristics of `archetype`.
    pub fn of(&self, archetype: Archetype) -> Characteristics {
        match archetype {
            Archetype::Bruiser => self.bruiser,
            Archetype::Sprinter => self.sprinter,
            Archetype::Thrower => self.thrower,
        }
    }
}

/// Every tunable number of a match.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ruleset {
    /// Turns each team plays per half.
    pub turns_per_half: u8,
    /// Halves per match.
    pub halves: u8,
    /// Seconds a coach has for one turn.
    pub turn_seconds: u16,
    /// Goblins per team.
    pub team_size: u8,
    /// Value an agility test must reach after its maluses.
    pub ball_roll_target: u8,
    /// Orthogonal steps a throw covers without malus.
    pub throw_free_range: u8,
    /// Extra malus on an interception attempt.
    pub interception_surprise_malus: u8,
    /// Malus on the defense roll of a goblin assaulted while down.
    pub downed_defense_malus: u8,
    /// Squares the kick-off drops the ball into the receiving team's half, from the centre.
    #[serde(default)]
    pub kickoff_depth: u8,
    /// Most a Stance bonus adds to one roll; 5, a goblin and its four neighbours, means no cap.
    #[serde(default = "uncapped_stance_bonus")]
    pub stance_bonus_cap: u8,
    /// Whether a goblin keeps the steps it did not walk after its action: its Move and one action
    /// per turn, spent over as many orders as its coach likes; otherwise its first order ends its turn.
    #[serde(default)]
    pub keeps_moving_after_action: bool,
    /// Whether a turn ends on its own once no goblin of the team can act; otherwise it waits
    /// for its coach to end it or for the clock to run out.
    #[serde(default = "turns_end_on_their_own")]
    pub turn_ends_when_no_goblin_can_act: bool,
    /// Arena mask rows (`x` out of bounds, `O` in bounds, `H` home den, `A` away den).
    pub arena: Vec<String>,
    /// Where goblins stand at kick-off; match records of rules 0.1 leave out their den lines.
    #[serde(default, skip_serializing_if = "Formation::is_den_lines")]
    pub formation: Formation,
    /// Characteristics per archetype.
    pub archetypes: ArchetypeCharacteristics,
    /// Allowed count per archetype in a roster.
    pub roster_limits: RosterLimits,
    /// Roster used when a coach does not pick one.
    pub default_roster: Roster,
}

/// Why a ruleset cannot be used.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum RulesetError {
    /// The text is not a TOML document matching the ruleset layout.
    #[error("the rules file does not describe a ruleset: {0}")]
    Unreadable(String),
    /// The arena mask is invalid.
    #[error(transparent)]
    Arena(#[from] ArenaError),
    /// An archetype has a zero-faced die or no health.
    #[error("{0:?} has a zero-faced die or no health")]
    UnplayableArchetype(Archetype),
    /// The default roster breaks the team size or the limits.
    #[error("the default roster is invalid: {0}")]
    DefaultRoster(#[from] RosterError),
    /// A match without halves or turns cannot be played.
    #[error("a match needs at least one half and one turn per half")]
    NoTurns,
    /// The kick-off formation cannot line a team up.
    #[error(transparent)]
    Formation(#[from] FormationError),
}

fn uncapped_stance_bonus() -> u8 {
    5
}

fn turns_end_on_their_own() -> bool {
    true
}

impl Default for Ruleset {
    fn default() -> Self {
        Self {
            turns_per_half: 12,
            halves: 2,
            turn_seconds: 60,
            team_size: 9,
            ball_roll_target: 2,
            throw_free_range: 6,
            interception_surprise_malus: 1,
            downed_defense_malus: 2,
            kickoff_depth: 0,
            stance_bonus_cap: uncapped_stance_bonus(),
            keeps_moving_after_action: true,
            turn_ends_when_no_goblin_can_act: false,
            arena: BOWL_21_MASK.iter().map(|row| (*row).to_string()).collect(),
            formation: Formation::Lines(vec![
                FormationLine {
                    depth: 1,
                    across: vec![-1, 0, 1],
                    prefers: vec![Archetype::Bruiser, Archetype::Sprinter, Archetype::Thrower],
                },
                FormationLine {
                    depth: 3,
                    across: vec![-5, -4, 4, 5],
                    prefers: vec![Archetype::Sprinter, Archetype::Bruiser, Archetype::Thrower],
                },
                FormationLine {
                    depth: 8,
                    across: vec![-2, 2],
                    prefers: vec![Archetype::Thrower, Archetype::Bruiser, Archetype::Sprinter],
                },
            ]),
            archetypes: ArchetypeCharacteristics {
                bruiser: Characteristics {
                    attack: 4,
                    defense: 4,
                    agility: 2,
                    health: 6,
                    movement: 3,
                },
                sprinter: Characteristics {
                    attack: 3,
                    defense: 2,
                    agility: 4,
                    health: 3,
                    movement: 5,
                },
                thrower: Characteristics {
                    attack: 2,
                    defense: 3,
                    agility: 4,
                    health: 3,
                    movement: 4,
                },
            },
            roster_limits: RosterLimits {
                bruiser: CountLimit { min: 0, max: 8 },
                sprinter: CountLimit { min: 0, max: 8 },
                thrower: CountLimit { min: 1, max: 2 },
            },
            default_roster: Roster::DEFAULT,
        }
    }
}

impl Ruleset {
    /// Parses and validates a ruleset written in TOML.
    ///
    /// # Errors
    /// [`RulesetError::Unreadable`] when the text does not match the layout, otherwise the
    /// first validation failure (see [`Ruleset::validate`]).
    pub fn from_toml_str(text: &str) -> Result<Ruleset, RulesetError> {
        let ruleset: Ruleset =
            toml::from_str(text).map_err(|error| RulesetError::Unreadable(error.to_string()))?;
        ruleset.validate()?;
        Ok(ruleset)
    }

    /// Checks that the ruleset can be played.
    ///
    /// # Errors
    /// A [`RulesetError`] naming the first problem found: no turns, invalid arena,
    /// unplayable archetype, invalid default roster, then a formation that cannot line a team up.
    pub fn validate(&self) -> Result<(), RulesetError> {
        if self.halves == 0 || self.turns_per_half == 0 {
            return Err(RulesetError::NoTurns);
        }
        let arena = self.build_arena()?;
        if let Some(archetype) = Archetype::ALL
            .into_iter()
            .find(|archetype| !self.archetypes.of(*archetype).is_playable())
        {
            return Err(RulesetError::UnplayableArchetype(archetype));
        }
        self.default_roster
            .validate(self.team_size, &self.roster_limits)?;
        self.formation.validate(&arena, self.team_size)?;
        Ok(())
    }

    /// The arena described by the mask.
    ///
    /// # Errors
    /// The [`ArenaError`] of an invalid mask.
    pub fn build_arena(&self) -> Result<Arena, ArenaError> {
        Arena::from_mask(&self.arena)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_default_ruleset_when_validated_then_accepted() {
        assert_eq!(Ruleset::default().validate(), Ok(()));
    }

    #[test]
    fn given_default_ruleset_when_reading_the_bruiser_then_four_four_two_six_three() {
        assert_eq!(
            Ruleset::default().archetypes.of(Archetype::Bruiser),
            Characteristics {
                attack: 4,
                defense: 4,
                agility: 2,
                health: 6,
                movement: 3
            }
        );
    }

    #[test]
    fn given_default_ruleset_when_reading_the_match_length_then_two_halves_of_twelve_turns() {
        let ruleset = Ruleset::default();
        assert_eq!((ruleset.halves, ruleset.turns_per_half), (2, 12));
    }

    #[test]
    fn given_default_ruleset_when_reading_how_goblins_move_then_they_keep_moving_after_their_action()
     {
        assert!(Ruleset::default().keeps_moving_after_action);
    }

    #[test]
    fn given_default_ruleset_when_reading_how_turns_end_then_they_wait_for_the_coach() {
        assert!(!Ruleset::default().turn_ends_when_no_goblin_can_act);
    }

    #[test]
    fn given_default_ruleset_when_written_and_read_back_then_unchanged() {
        let text = toml::to_string(&Ruleset::default()).unwrap();
        assert_eq!(Ruleset::from_toml_str(&text), Ok(Ruleset::default()));
    }

    #[test]
    fn given_text_that_is_not_a_ruleset_when_read_then_unreadable() {
        assert!(matches!(
            Ruleset::from_toml_str("turns_per_half = \"many\""),
            Err(RulesetError::Unreadable(_))
        ));
    }

    #[test]
    fn given_zero_turns_when_validated_then_no_turns() {
        let ruleset = Ruleset {
            turns_per_half: 0,
            ..Ruleset::default()
        };
        assert_eq!(ruleset.validate(), Err(RulesetError::NoTurns));
    }

    #[test]
    fn given_ragged_arena_when_validated_then_arena_error() {
        let ruleset = Ruleset {
            arena: vec!["OOO".into(), "O".into()],
            ..Ruleset::default()
        };
        assert!(matches!(ruleset.validate(), Err(RulesetError::Arena(_))));
    }

    #[test]
    fn given_a_formation_too_short_for_the_team_when_validated_then_formation_error() {
        let ruleset = Ruleset {
            formation: Formation::Lines(Vec::new()),
            ..Ruleset::default()
        };
        assert!(matches!(
            ruleset.validate(),
            Err(RulesetError::Formation(
                FormationError::TooFewSquares { .. }
            ))
        ));
    }

    #[test]
    fn given_sprinter_without_agility_die_when_validated_then_unplayable() {
        let mut ruleset = Ruleset::default();
        ruleset.archetypes.sprinter.agility = 0;
        assert_eq!(
            ruleset.validate(),
            Err(RulesetError::UnplayableArchetype(Archetype::Sprinter))
        );
    }

    #[test]
    fn given_default_roster_of_ten_when_validated_then_default_roster_error() {
        let ruleset = Ruleset {
            default_roster: Roster {
                bruisers: 5,
                sprinters: 4,
                throwers: 1,
            },
            ..Ruleset::default()
        };
        assert!(matches!(
            ruleset.validate(),
            Err(RulesetError::DefaultRoster(_))
        ));
    }
}
