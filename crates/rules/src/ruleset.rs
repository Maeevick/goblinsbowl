//! Every tunable number of a match, as loaded from `rules.toml`.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::arena::{Arena, ArenaError, ROUND_BOWL_21_MASK};
use crate::characteristics::{Archetype, Characteristics};
use crate::formation::{Formation, FormationError, FormationLine, kickoff_drop_square};
use crate::roster::{CountLimit, Roster, RosterError, RosterLimits};
use crate::square::Reach;
use crate::team::TeamId;

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
    /// Fewest squares the kick-off drops the ball into the receiving team's half, from the centre.
    #[serde(default)]
    pub kickoff_depth: u8,
    /// Extra squares of kick-off depth rolled on a die, from 0 to this many.
    #[serde(default)]
    pub kickoff_depth_spread: u8,
    /// Most rows the kick-off drops the ball to either side of the centre row, rolled on a die.
    #[serde(default)]
    pub kickoff_across_spread: u8,
    /// Times the kick-off ball bounces after it drops, unless a goblin catches it first.
    #[serde(default = "one_kickoff_bounce")]
    pub kickoff_bounces: u8,
    /// Most a Stance bonus adds to one roll; 5, a goblin and its four neighbours, means no cap.
    #[serde(default = "uncapped_stance_bonus")]
    pub stance_bonus_cap: u8,
    /// Most braced adjacent teammates a Stance bonus counts; 4, every neighbour, means no cap.
    #[serde(default = "every_braced_neighbour")]
    pub neighbour_stance_bonus_cap: u8,
    /// Whether a dodge is an agility test, each adjacent standing enemy a malus of one and two when
    /// in Stance; otherwise the Agility die faces the Defense die of the most dangerous enemy.
    #[serde(default)]
    pub dodges_test_agility: bool,
    /// Added to every agility dodge: at 1, one of the enemies around the square left makes no
    /// difference.
    #[serde(default)]
    pub dodge_bonus: u8,
    /// Whether a goblin can spend its action bracing in Stance, a Defense bonus until its team's
    /// next turn.
    #[serde(default = "stance_in_the_rules")]
    pub goblins_can_take_stance: bool,
    /// Whether a goblin of the team on turn missing a bouncing ball, after an assault knocked the
    /// carrier down, ends the turn; otherwise only a missed catch of the team's own throw does.
    #[serde(default = "missed_bounces_end_turns")]
    pub missed_bounces_end_the_turn: bool,
    /// Whether the goblin holding the ball may assault.
    #[serde(default = "carriers_assault")]
    pub carriers_can_assault: bool,
    /// Whether the goblin holding the ball may brace in Stance.
    #[serde(default = "carriers_brace")]
    pub carriers_can_brace: bool,
    /// Whether a throw to an orthogonally adjacent teammate is a hand-off: no throw roll, only the
    /// receiver's catch.
    #[serde(default)]
    pub adjacent_throws_are_hand_offs: bool,
    /// Whether every goblin gets its Health back at a kick-off, a knocked-out one rolling its Health
    /// die; otherwise Health lasts the whole match and a knocked-out goblin stays out of it.
    #[serde(default = "kick_offs_heal")]
    pub goblins_heal_at_kick_off: bool,
    /// Whether a goblin keeps the steps it did not walk after its action: its Move and one action
    /// per turn, spent over as many orders as its coach likes; otherwise its first order ends its turn.
    #[serde(default)]
    pub keeps_moving_after_action: bool,
    /// Whether a turn ends on its own once no goblin of the team can act; otherwise it waits
    /// for its coach to end it or for the clock to run out.
    #[serde(default = "turns_end_on_their_own")]
    pub turn_ends_when_no_goblin_can_act: bool,
    /// Whether an assault tied on the dice knocks the defender down without hurting it; otherwise
    /// nobody falls and the turn goes on.
    #[serde(default = "ties_knock_down")]
    pub assault_ties_knock_down: bool,
    /// Malus on the defense roll of the goblin holding the ball when it is assaulted.
    #[serde(default)]
    pub carrier_defense_malus: u8,
    /// Whether, at a kick-off where nobody heals, each knocked-out goblin rolls its Health die minus
    /// one and comes back with that much Health, a 0 keeping it out until the next kick-off.
    #[serde(default)]
    pub knocked_out_goblins_may_return: bool,
    /// Whether an assault or a throw costs one step of the goblin's Move and a Stance every step it
    /// has left, so a goblin with no step left cannot act.
    #[serde(default)]
    pub actions_cost_a_step: bool,
    /// Whether a goblin may take each kind of action once per turn, an assault, a throw and a
    /// Stance in any order; otherwise it takes one action in all.
    #[serde(default)]
    pub one_action_of_each_kind: bool,
    /// Whether an enemy braced in Stance weighs two on an agility dodge instead of one.
    #[serde(default = "braced_tacklers_weigh_double")]
    pub braced_tacklers_weigh_double: bool,
    /// Whether a standing goblin weighs on the eight squares around it rather than the four
    /// orthogonal ones: leaving them takes a dodge, rolls made there suffer its malus, and a ball
    /// flying over one of them, not only over its own square, may be intercepted.
    #[serde(default)]
    pub reach_covers_diagonals: bool,
    /// Whether a bouncing ball goes to one of the eight squares around, rolled on a d8 clockwise
    /// from north, instead of one of the four orthogonal ones on a d4.
    #[serde(default)]
    pub ball_bounces_all_around: bool,
    /// Whether an assault may be chirurgical (+1 to win, no Health lost), brutal (−1 to win, one
    /// more Health lost) or a projection (−1 to win, the target falling on a square picked around
    /// the attacker); otherwise every assault is simple.
    #[serde(default)]
    pub assaults_have_styles: bool,
    /// Whether a goblin that has not moved this turn may throw into chaos, with no throw roll, the
    /// ball bouncing three times from the receiver's square and the turn ending unless a teammate
    /// then holds it, or, holding the ball, focus, +1 to its throws on its team's next turn;
    /// otherwise every throw is simple and nobody focuses.
    #[serde(default)]
    pub throws_have_styles: bool,
    /// Whether a Thrower may throw focused, sacrificing every step it has left after the throw for
    /// +1 each to its throw roll; the focus of `throws_have_styles`, a whole turn standing still
    /// for +1 on the next one, is then gone.
    #[serde(default)]
    pub focused_throws_spend_steps_left: bool,
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
    /// A kick-off could drop the ball off the bowl.
    #[error("a kick-off {depth} squares deep and {across} rows across drops the ball off the bowl")]
    KickOffOffTheBowl {
        /// Squares into the receiving half.
        depth: u8,
        /// Rows from the centre row.
        across: i8,
    },
}

fn uncapped_stance_bonus() -> u8 {
    5
}

fn every_braced_neighbour() -> u8 {
    4
}

fn turns_end_on_their_own() -> bool {
    true
}

fn stance_in_the_rules() -> bool {
    true
}

fn missed_bounces_end_turns() -> bool {
    true
}

fn carriers_assault() -> bool {
    true
}

fn carriers_brace() -> bool {
    true
}

fn kick_offs_heal() -> bool {
    true
}

fn one_kickoff_bounce() -> u8 {
    1
}

fn ties_knock_down() -> bool {
    true
}

fn braced_tacklers_weigh_double() -> bool {
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
            kickoff_depth: 4,
            kickoff_depth_spread: 3,
            kickoff_across_spread: 5,
            kickoff_bounces: 3,
            stance_bonus_cap: 1,
            neighbour_stance_bonus_cap: 0,
            dodges_test_agility: true,
            dodge_bonus: 0,
            goblins_can_take_stance: false,
            missed_bounces_end_the_turn: false,
            carriers_can_assault: false,
            carriers_can_brace: false,
            adjacent_throws_are_hand_offs: true,
            goblins_heal_at_kick_off: false,
            keeps_moving_after_action: true,
            turn_ends_when_no_goblin_can_act: false,
            assault_ties_knock_down: false,
            carrier_defense_malus: 1,
            knocked_out_goblins_may_return: true,
            actions_cost_a_step: true,
            one_action_of_each_kind: true,
            braced_tacklers_weigh_double: false,
            reach_covers_diagonals: true,
            ball_bounces_all_around: true,
            assaults_have_styles: true,
            throws_have_styles: true,
            focused_throws_spend_steps_left: true,
            arena: ROUND_BOWL_21_MASK
                .iter()
                .map(|row| (*row).to_string())
                .collect(),
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
                    agility: 3,
                    health: 7,
                    movement: 4,
                },
                sprinter: Characteristics {
                    attack: 3,
                    defense: 2,
                    agility: 6,
                    health: 5,
                    movement: 6,
                },
                thrower: Characteristics {
                    attack: 3,
                    defense: 3,
                    agility: 6,
                    health: 6,
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
    /// unplayable archetype, invalid default roster, a formation that cannot line a team up, then a
    /// kick-off that could drop the ball off the bowl.
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
        self.validate_kickoff(&arena)
    }

    fn validate_kickoff(&self, arena: &Arena) -> Result<(), RulesetError> {
        let deepest = self.kickoff_depth.saturating_add(self.kickoff_depth_spread);
        let widest = i8::try_from(self.kickoff_across_spread).unwrap_or(i8::MAX);
        for depth in self.kickoff_depth..=deepest {
            for across in -widest..=widest {
                let on_the_bowl = TeamId::DUEL
                    .into_iter()
                    .all(|team| kickoff_drop_square(arena, team, depth, across).is_some());
                if !on_the_bowl {
                    return Err(RulesetError::KickOffOffTheBowl { depth, across });
                }
            }
        }
        Ok(())
    }

    /// The arena described by the mask.
    ///
    /// # Errors
    /// The [`ArenaError`] of an invalid mask.
    pub fn build_arena(&self) -> Result<Arena, ArenaError> {
        Arena::from_mask(&self.arena)
    }

    /// The squares a standing goblin weighs on around it.
    pub fn reach(&self) -> Reach {
        if self.reach_covers_diagonals {
            Reach::AllAround
        } else {
            Reach::Orthogonal
        }
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
    fn given_default_ruleset_when_reading_the_bruiser_then_four_four_three_seven_four() {
        assert_eq!(
            Ruleset::default().archetypes.of(Archetype::Bruiser),
            Characteristics {
                attack: 4,
                defense: 4,
                agility: 3,
                health: 7,
                movement: 4
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
    fn given_a_kick_off_scattering_beyond_the_east_edge_when_validated_then_off_the_bowl() {
        let ruleset = Ruleset {
            kickoff_depth: 8,
            kickoff_depth_spread: 5,
            kickoff_across_spread: 0,
            ..Ruleset::default()
        };
        assert_eq!(
            ruleset.validate(),
            Err(RulesetError::KickOffOffTheBowl {
                depth: 11,
                across: 0
            })
        );
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
