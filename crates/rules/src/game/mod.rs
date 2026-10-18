//! The rules engine: applies commands to a match state, rolling dice through a [`DiceRoller`].

use thiserror::Error;

use crate::arena::Arena;
use crate::command::Command;
use crate::dice::DiceRoller;
use crate::event::{GameEvent, TurnEndReason};
use crate::formation::kickoff_squares;
use crate::roster::{GoblinId, Roster, RosterError};
use crate::ruleset::{Ruleset, RulesetError};
use crate::state::{Ball, GameState, Goblin, Phase, Presence};
use crate::team::TeamId;
use crate::validation::validate_activation;
use crate::violation::RuleViolation;

#[cfg(test)]
mod action_economy_tests;
mod activation;
mod fight;
#[cfg(test)]
mod fight_tests;
mod kickoff;
#[cfg(test)]
mod moving_after_action_tests;
#[cfg(test)]
mod property_tests;
#[cfg(test)]
pub(crate) mod scenario;
#[cfg(test)]
mod scoring_tests;
mod throwing;
mod turns;
#[cfg(test)]
mod turns_tests;

/// Why a match cannot start.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SetupError {
    /// The ruleset is not playable.
    #[error(transparent)]
    Ruleset(#[from] RulesetError),
    /// A roster breaks the ruleset.
    #[error("{team:?} roster: {error}")]
    Roster {
        /// Team whose roster is wrong.
        team: TeamId,
        /// What is wrong with it.
        error: RosterError,
    },
}

/// A match in progress: its ruleset, arena, state and dice.
#[derive(Clone, Debug)]
pub struct Game<D: DiceRoller> {
    ruleset: Ruleset,
    arena: Arena,
    state: GameState,
    dice: D,
}

impl<D: DiceRoller> Game<D> {
    /// Starts a match: lines teams up, tosses a coin (d2: 1 home, 2 away) for the receiving team,
    /// drops the ball on the centre square, bounces it once and starts the receiver's first turn.
    ///
    /// # Errors
    /// [`SetupError`] when the ruleset or a roster is invalid.
    pub fn start(
        ruleset: Ruleset,
        rosters: [Roster; 2],
        dice: D,
    ) -> Result<(Self, Vec<GameEvent>), SetupError> {
        ruleset.validate()?;
        let arena = ruleset.build_arena().map_err(RulesetError::from)?;
        for (team, roster) in TeamId::DUEL.into_iter().zip(&rosters) {
            roster
                .validate(ruleset.team_size, &ruleset.roster_limits)
                .map_err(|error| SetupError::Roster { team, error })?;
        }
        let goblins = TeamId::DUEL
            .into_iter()
            .zip(&rosters)
            .flat_map(|(team, roster)| {
                let lineup = kickoff_squares(&arena, &ruleset.formation, team, roster);
                roster
                    .archetypes_by_number()
                    .into_iter()
                    .zip(lineup)
                    .zip(1_u8..)
                    .map(move |((archetype, square), number)| {
                        let characteristics = ruleset.archetypes.of(archetype);
                        Goblin {
                            id: GoblinId::new(team, number),
                            archetype,
                            characteristics,
                            health: characteristics.health,
                            presence: Presence::Standing(square),
                            in_stance: false,
                            steps_left: characteristics.movement,
                            has_action_left: true,
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        let centre = arena.centre();
        let mut game = Self {
            state: GameState {
                rosters: rosters.to_vec(),
                goblins,
                ball: Ball::Loose { at: centre },
                score: vec![0; TeamId::DUEL.len()],
                half: 1,
                turns_played: vec![0; TeamId::DUEL.len()],
                phase: Phase::Playing {
                    active_team: TeamId::HOME,
                },
                opening_team: TeamId::HOME,
            },
            ruleset,
            arena,
            dice,
        };
        let mut events = Vec::new();
        let receiving_team = if game.dice.roll(2) == 1 {
            TeamId::HOME
        } else {
            TeamId::AWAY
        };
        events.push(GameEvent::CoinTossed { receiving_team });
        game.state.opening_team = receiving_team;
        game.kick_off(receiving_team, &mut events);
        game.start_turn(receiving_team, &mut events);
        Ok((game, events))
    }

    /// Continues a match from a saved or hand-made state.
    ///
    /// # Errors
    /// [`SetupError::Ruleset`] when the ruleset is invalid.
    pub fn resume(ruleset: Ruleset, state: GameState, dice: D) -> Result<Self, SetupError> {
        ruleset.validate()?;
        let arena = ruleset.build_arena().map_err(RulesetError::from)?;
        Ok(Self {
            ruleset,
            arena,
            state,
            dice,
        })
    }

    /// Applies a command sent on behalf of `team` and reports what happened, in order.
    ///
    /// # Errors
    /// A [`RuleViolation`] when the command breaks the rules; the match is then left untouched.
    pub fn apply(
        &mut self,
        team: TeamId,
        command: Command,
    ) -> Result<Vec<GameEvent>, RuleViolation> {
        let active_team = self
            .state
            .active_team()
            .ok_or(RuleViolation::MatchFinished)?;
        if team != active_team {
            return Err(RuleViolation::NotYourTurn { active_team });
        }
        let mut events = Vec::new();
        match command {
            Command::Act(activation) => {
                validate_activation(&self.arena, &self.ruleset, &self.state, team, &activation)?;
                self.resolve_activation(activation, &mut events);
            }
            Command::EndTurn => self.finish_turn(TurnEndReason::EndTurn, &mut events),
            Command::TimeUp => self.finish_turn(TurnEndReason::TimeUp, &mut events),
        }
        Ok(events)
    }

    /// The current state.
    pub fn state(&self) -> &GameState {
        &self.state
    }

    /// The ruleset in force.
    pub fn ruleset(&self) -> &Ruleset {
        &self.ruleset
    }

    /// The arena.
    pub fn arena(&self) -> &Arena {
        &self.arena
    }

    /// The dice, for inspection (seed, scripted rolls left).
    pub fn dice(&self) -> &D {
        &self.dice
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dice::ScriptedDice;
    use crate::square::Square;

    const COIN_HOME: u8 = 1;
    const COIN_AWAY: u8 = 2;
    const BOUNCE_EAST: u8 = 2;
    const BOUNCE_NORTH: u8 = 1;

    fn centre_kick_off() -> Ruleset {
        Ruleset {
            kickoff_depth: 0,
            kickoff_depth_spread: 0,
            kickoff_across_spread: 0,
            kickoff_bounces: 1,
            ..Ruleset::default()
        }
    }

    fn started(rolls: impl IntoIterator<Item = u8>) -> (Game<ScriptedDice>, Vec<GameEvent>) {
        Game::start(
            centre_kick_off(),
            [Roster::DEFAULT, Roster::DEFAULT],
            ScriptedDice::new(rolls),
        )
        .unwrap()
    }

    #[test]
    fn given_a_kick_off_depth_of_three_when_the_away_team_receives_then_the_ball_drops_three_squares_into_its_half()
     {
        let ruleset = Ruleset {
            kickoff_depth: 3,
            ..centre_kick_off()
        };
        let (_, events) = Game::start(
            ruleset,
            [Roster::DEFAULT, Roster::DEFAULT],
            ScriptedDice::new([COIN_AWAY, BOUNCE_EAST]),
        )
        .unwrap();
        assert!(events.contains(&GameEvent::BallDropped {
            at: Square::new(13, 10)
        }));
    }

    #[test]
    fn given_a_kick_off_scattering_four_to_seven_deep_and_five_across_when_away_receives_then_the_ball_drops_where_the_dice_say()
     {
        let ruleset = Ruleset {
            kickoff_depth: 4,
            kickoff_depth_spread: 3,
            kickoff_across_spread: 5,
            ..centre_kick_off()
        };
        let depth_five = 2;
        let five_rows_north = 1;
        let (_, events) = Game::start(
            ruleset,
            [Roster::DEFAULT, Roster::DEFAULT],
            ScriptedDice::new([COIN_AWAY, depth_five, five_rows_north, BOUNCE_NORTH]),
        )
        .unwrap();
        assert!(events.contains(&GameEvent::BallDropped {
            at: Square::new(15, 5)
        }));
    }

    #[test]
    fn given_three_kick_off_bounces_when_nobody_catches_then_the_ball_bounces_three_times() {
        let ruleset = Ruleset {
            kickoff_bounces: 3,
            ..centre_kick_off()
        };
        let (game, events) = Game::start(
            ruleset,
            [Roster::DEFAULT, Roster::DEFAULT],
            ScriptedDice::new([COIN_HOME, BOUNCE_NORTH, BOUNCE_NORTH, BOUNCE_NORTH]),
        )
        .unwrap();
        let bounces = events
            .iter()
            .filter(|event| matches!(event, GameEvent::BallBounced { .. }))
            .count();
        assert_eq!(
            (bounces, game.state().ball),
            (
                3,
                Ball::Loose {
                    at: Square::new(10, 7)
                }
            )
        );
    }

    #[test]
    fn given_default_rosters_when_the_match_starts_then_eighteen_goblins_stand_on_the_bowl() {
        let (game, _) = started([COIN_HOME, BOUNCE_NORTH]);
        assert_eq!(
            game.state()
                .goblins
                .iter()
                .filter(|goblin| goblin.is_standing())
                .count(),
            18
        );
    }

    #[test]
    fn given_coin_shows_one_when_the_match_starts_then_home_plays_first() {
        let (game, _) = started([COIN_HOME, BOUNCE_NORTH]);
        assert_eq!(game.state().active_team(), Some(TeamId::HOME));
    }

    #[test]
    fn given_coin_shows_two_when_the_match_starts_then_away_plays_first() {
        let (game, _) = started([COIN_AWAY, BOUNCE_NORTH]);
        assert_eq!(game.state().active_team(), Some(TeamId::AWAY));
    }

    #[test]
    fn given_bounce_north_from_the_centre_when_the_match_starts_then_the_ball_lies_between_the_front_lines()
     {
        let (game, _) = started([COIN_HOME, BOUNCE_NORTH]);
        assert_eq!(
            game.state().ball,
            Ball::Loose {
                at: Square::new(10, 9)
            }
        );
    }

    #[test]
    fn given_the_match_starts_when_reading_events_then_toss_kick_off_drop_bounce_and_first_turn() {
        let (_, events) = started([COIN_AWAY, BOUNCE_NORTH]);
        assert_eq!(
            events,
            vec![
                GameEvent::CoinTossed {
                    receiving_team: TeamId::AWAY
                },
                GameEvent::KickedOff {
                    receiving_team: TeamId::AWAY
                },
                GameEvent::BallDropped {
                    at: Square::new(10, 10)
                },
                GameEvent::BallBounced {
                    from: Square::new(10, 10),
                    to: Square::new(10, 9)
                },
                GameEvent::TurnStarted {
                    team: TeamId::AWAY,
                    half: 1,
                    turn: 1
                },
            ]
        );
    }

    #[test]
    fn given_the_match_starts_when_reading_turn_counters_then_only_the_receiver_started_a_turn() {
        let (game, _) = started([COIN_AWAY, BOUNCE_NORTH]);
        assert_eq!(game.state().turns_played, vec![0, 1]);
    }

    #[test]
    fn given_a_ten_goblin_home_roster_when_starting_then_roster_error() {
        let too_many = Roster {
            bruisers: 5,
            sprinters: 4,
            throwers: 1,
        };
        let outcome = Game::start(
            Ruleset::default(),
            [too_many, Roster::DEFAULT],
            ScriptedDice::new([]),
        );
        assert!(matches!(
            outcome,
            Err(SetupError::Roster {
                team: TeamId::HOME,
                ..
            })
        ));
    }

    #[test]
    fn given_the_match_starts_when_reading_home_goblins_then_numbered_one_to_nine() {
        let (game, _) = started([COIN_HOME, BOUNCE_NORTH]);
        let numbers: Vec<u8> = game
            .state()
            .goblins_of(TeamId::HOME)
            .map(|goblin| goblin.id.number)
            .collect();
        assert_eq!(numbers, (1..=9).collect::<Vec<_>>());
    }
}
