//! Hand-made match situations for rule tests.

use crate::characteristics::Archetype;
use crate::dice::ScriptedDice;
use crate::roster::{GoblinId, Roster};
use crate::ruleset::Ruleset;
use crate::square::Square;
use crate::state::{Ball, GameState, Goblin, Phase, Presence};
use crate::team::TeamId;

use super::Game;

/// Hand-made match situations for rule tests: an empty bowl, home to play turn 1 of half 1.
pub(crate) struct Scenario {
    ruleset: Ruleset,
    state: GameState,
}

impl Scenario {
    pub(crate) fn new() -> Self {
        let ruleset = Ruleset::default();
        let state = GameState {
            rosters: vec![Roster::DEFAULT, Roster::DEFAULT],
            goblins: Vec::new(),
            ball: Ball::Loose {
                at: Square::new(7, 7),
            },
            score: vec![0, 0],
            half: 1,
            turns_played: vec![1, 0],
            phase: Phase::Playing {
                active_team: TeamId::HOME,
            },
            opening_team: TeamId::HOME,
        };
        Self { ruleset, state }
    }

    pub(crate) fn rules(mut self, change: impl FnOnce(&mut Ruleset)) -> Self {
        change(&mut self.ruleset);
        self
    }

    pub(crate) fn goblin(mut self, id: &str, archetype: Archetype, x: u8, y: u8) -> Self {
        let id: GoblinId = id.parse().expect("scenario goblin id");
        let characteristics = self.ruleset.archetypes.of(archetype);
        self.state.goblins.push(Goblin {
            id,
            archetype,
            characteristics,
            health: characteristics.health,
            presence: Presence::Standing(Square::new(x, y)),
            in_stance: false,
            steps_left: characteristics.movement,
            has_action_left: true,
        });
        self
    }

    pub(crate) fn edit(mut self, id: &str, change: impl FnOnce(&mut Goblin)) -> Self {
        let id: GoblinId = id.parse().expect("scenario goblin id");
        change(self.state.goblin_mut(id).expect("scenario goblin exists"));
        self
    }

    pub(crate) fn ball_loose_at(mut self, x: u8, y: u8) -> Self {
        self.state.ball = Ball::Loose {
            at: Square::new(x, y),
        };
        self
    }

    pub(crate) fn ball_carried_by(mut self, id: &str) -> Self {
        self.state.ball = Ball::Carried {
            by: id.parse().expect("scenario goblin id"),
        };
        self
    }

    pub(crate) fn game(self, scripted_rolls: impl IntoIterator<Item = u8>) -> Game<ScriptedDice> {
        Game::resume(self.ruleset, self.state, ScriptedDice::new(scripted_rolls))
            .expect("scenario ruleset is valid")
    }

    pub(crate) fn turns(mut self, home: u8, away: u8) -> Self {
        self.state.turns_played = vec![home, away];
        self
    }

    pub(crate) fn active(mut self, team: TeamId) -> Self {
        self.state.phase = Phase::Playing { active_team: team };
        self
    }

    pub(crate) fn half(mut self, half: u8) -> Self {
        self.state.half = half;
        self
    }
}
