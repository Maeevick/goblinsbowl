//! Hand-made match situations for rule tests.

use crate::arena::BOWL_15_MASK;
use crate::characteristics::{Archetype, Characteristics};
use crate::dice::ScriptedDice;
use crate::formation::Formation;
use crate::roster::{GoblinId, Roster};
use crate::ruleset::{ArchetypeCharacteristics, Ruleset};
use crate::square::Square;
use crate::state::{ActionsUsed, Ball, GameState, Goblin, Phase, Presence};
use crate::team::TeamId;

use super::Game;

/// Hand-made match situations for rule tests: an empty 15 × 15 bowl under rules 0.1, whatever
/// `rules.toml` ships, home to play turn 1 of half 1.
pub(crate) struct Scenario {
    ruleset: Ruleset,
    state: GameState,
}

fn rules_0_1() -> Ruleset {
    Ruleset {
        turns_per_half: 10,
        arena: BOWL_15_MASK.iter().map(|row| (*row).to_string()).collect(),
        formation: Formation::DenLines,
        kickoff_depth: 0,
        kickoff_depth_spread: 0,
        kickoff_across_spread: 0,
        kickoff_bounces: 1,
        stance_bonus_cap: 5,
        neighbour_stance_bonus_cap: 4,
        dodges_test_agility: false,
        dodge_bonus: 0,
        goblins_can_take_stance: true,
        missed_bounces_end_the_turn: true,
        carriers_can_assault: true,
        carriers_can_brace: true,
        adjacent_throws_are_hand_offs: false,
        hand_off_catch_bonus: 0,
        goblins_heal_at_kick_off: true,
        keeps_moving_after_action: false,
        turn_ends_when_no_goblin_can_act: true,
        assault_ties_knock_down: true,
        carrier_defense_malus: 0,
        knocked_out_goblins_may_return: false,
        actions_cost_a_step: false,
        one_action_of_each_kind: false,
        braced_tacklers_weigh_double: true,
        reach_covers_diagonals: false,
        assaults_reach_diagonals: false,
        ball_bounces_all_around: false,
        assaults_have_styles: false,
        brutal_and_projection_need_a_bruiser: false,
        throws_have_styles: false,
        only_throwers_throw_into_chaos: false,
        focused_throws_spend_steps_left: false,
        focused_throw_bonus: None,
        failed_focused_throws_down_the_thrower: false,
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
        default_roster: Roster {
            bruisers: 4,
            sprinters: 4,
            throwers: 1,
        },
        ..Ruleset::default()
    }
}

impl Scenario {
    pub(crate) fn new() -> Self {
        let ruleset = rules_0_1();
        let state = GameState {
            rosters: vec![ruleset.default_roster; 2],
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
            focus: Default::default(),
            id,
            archetype,
            characteristics,
            health: characteristics.health,
            presence: Presence::Standing(Square::new(x, y)),
            in_stance: false,
            steps_left: characteristics.movement,
            has_action_left: true,
            actions_used: ActionsUsed::default(),
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
