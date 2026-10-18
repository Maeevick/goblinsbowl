//! The state of a match: goblins, ball, score and per-team turn counters.

use serde::{Deserialize, Serialize};

use crate::characteristics::{Archetype, Characteristics};
use crate::command::{Action, Activation};
use crate::roster::{GoblinId, Roster};
use crate::ruleset::Ruleset;
use crate::square::Square;
use crate::team::TeamId;

/// Where a goblin is and whether it stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Presence {
    /// On its feet on this square.
    Standing(Square),
    /// Lying on this square: no reach, no Stance, cannot hold the ball.
    Down(Square),
    /// Off the bowl until the next kick-off.
    KnockedOut,
}

impl Presence {
    /// The square the goblin occupies, standing or down.
    pub fn square(self) -> Option<Square> {
        match self {
            Presence::Standing(square) | Presence::Down(square) => Some(square),
            Presence::KnockedOut => None,
        }
    }

    /// Whether the goblin is on its feet on the bowl.
    pub fn is_standing(self) -> bool {
        matches!(self, Presence::Standing(_))
    }
}

/// A goblin taking part in the match.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Goblin {
    /// Its identity.
    pub id: GoblinId,
    /// Its archetype.
    pub archetype: Archetype,
    /// Its characteristics.
    pub characteristics: Characteristics,
    /// Health left; 0 means knocked out.
    pub health: u8,
    /// Where it is.
    pub presence: Presence,
    /// Whether it braced in Stance since its team's last turn started.
    pub in_stance: bool,
    /// Steps it may still take this turn.
    pub steps_left: u8,
    /// Whether it may still Assault, brace in Stance or Throw this turn.
    pub has_action_left: bool,
}

impl Goblin {
    /// The team it plays for.
    pub fn team(&self) -> TeamId {
        self.id.team
    }

    /// The square it occupies, standing or down.
    pub fn square(&self) -> Option<Square> {
        self.presence.square()
    }

    /// Whether it stands on the bowl.
    pub fn is_standing(&self) -> bool {
        self.presence.is_standing()
    }

    /// Whether it may still be ordered this turn: standing, with steps or its action left.
    pub fn can_act(&self) -> bool {
        self.is_standing() && (self.steps_left > 0 || self.has_action_left)
    }

    /// Gives it a whole turn: every step of its Move and its action.
    pub fn refresh_its_turn(&mut self) {
        self.steps_left = self.characteristics.movement;
        self.has_action_left = true;
    }

    /// Spends whatever is left of its turn: no step, no action.
    pub fn spend_its_turn(&mut self) {
        self.steps_left = 0;
        self.has_action_left = false;
    }

    /// Spends what `order` uses once it is through: the steps it walks and its action when goblins
    /// keep moving after their action, otherwise, or when the order neither moves nor acts, the
    /// rest of its turn. Where actions cost steps, a goblin left without one can no longer act.
    pub fn spend_order(&mut self, order: &Activation, ruleset: &Ruleset) {
        let steps = u8::try_from(order.path.len()).unwrap_or(u8::MAX);
        if ruleset.keeps_moving_after_action && (steps > 0 || order.action.is_some()) {
            self.steps_left = self.steps_left.saturating_sub(steps);
            if let Some(action) = order.action {
                self.spend_action(action, ruleset);
            }
            if ruleset.actions_cost_a_step && self.steps_left == 0 {
                self.has_action_left = false;
            }
        } else {
            self.spend_its_turn();
        }
    }

    fn spend_action(&mut self, action: Action, ruleset: &Ruleset) {
        self.has_action_left = false;
        if ruleset.actions_cost_a_step {
            self.steps_left = match action {
                Action::Stance => 0,
                Action::Assault { .. } | Action::Throw { .. } => self.steps_left.saturating_sub(1),
            };
        }
    }
}

/// Who braces for a goblin: itself, and how many of its adjacent teammates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StanceSupport {
    /// Whether the goblin itself stands braced in Stance.
    pub itself: bool,
    /// Standing teammates braced in Stance on squares next to it.
    pub neighbours: u8,
}

/// Where the ball is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ball {
    /// Held by a standing goblin.
    Carried {
        /// The carrier.
        by: GoblinId,
    },
    /// Lying on a square.
    Loose {
        /// Its square.
        at: Square,
    },
}

/// Whether the match is running and whose turn it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// A team is playing its turn.
    Playing {
        /// The team allowed to act.
        active_team: TeamId,
    },
    /// The final whistle went.
    Finished,
}

/// Everything needed to know where a match stands, without its dice.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameState {
    /// Rosters of the teams, indexed by team.
    pub rosters: Vec<Roster>,
    /// All goblins, home first, each team in shirt-number order.
    pub goblins: Vec<Goblin>,
    /// Where the ball is.
    pub ball: Ball,
    /// Points per team, indexed by team.
    pub score: Vec<u8>,
    /// Current half, starting at 1.
    pub half: u8,
    /// Turns each team started in the current half, indexed by team.
    pub turns_played: Vec<u8>,
    /// Running or finished.
    pub phase: Phase,
    /// The team that played first in the first half; the other one opens the second half.
    pub opening_team: TeamId,
}

impl GameState {
    /// The goblin with this id.
    pub fn goblin(&self, id: GoblinId) -> Option<&Goblin> {
        self.goblins.iter().find(|goblin| goblin.id == id)
    }

    /// The goblin standing or lying on `square`.
    pub fn goblin_at(&self, square: Square) -> Option<&Goblin> {
        self.goblins
            .iter()
            .find(|goblin| goblin.square() == Some(square))
    }

    /// Goblins of `team`, in shirt-number order.
    pub fn goblins_of(&self, team: TeamId) -> impl Iterator<Item = &Goblin> + '_ {
        self.goblins
            .iter()
            .filter(move |goblin| goblin.team() == team)
    }

    /// The team allowed to act, if the match is running.
    pub fn active_team(&self) -> Option<TeamId> {
        match self.phase {
            Phase::Playing { active_team } => Some(active_team),
            Phase::Finished => None,
        }
    }

    /// The goblin holding the ball, if any.
    pub fn carrier(&self) -> Option<GoblinId> {
        match self.ball {
            Ball::Carried { by } => Some(by),
            Ball::Loose { .. } => None,
        }
    }

    /// The square of the ball, loose or carried.
    pub fn ball_square(&self) -> Option<Square> {
        match self.ball {
            Ball::Carried { by } => self.goblin(by).and_then(Goblin::square),
            Ball::Loose { at } => Some(at),
        }
    }

    /// Standing goblins opposing `team` on squares orthogonally next to `square`.
    pub fn standing_opponents_next_to(
        &self,
        square: Square,
        team: TeamId,
    ) -> impl Iterator<Item = &Goblin> + '_ {
        self.goblins.iter().filter(move |goblin| {
            goblin.team() != team
                && goblin.is_standing()
                && goblin
                    .square()
                    .is_some_and(|other| other.is_orthogonally_adjacent(square))
        })
    }

    /// Who braces for goblin `id`: itself if it stands in Stance, and its braced adjacent teammates.
    pub fn stance_support(&self, id: GoblinId) -> StanceSupport {
        let Some(goblin) = self.goblin(id) else {
            return StanceSupport::default();
        };
        let Some(square) = goblin.square() else {
            return StanceSupport::default();
        };
        let braced_neighbours = self
            .goblins_of(goblin.team())
            .filter(|teammate| {
                teammate.id != id
                    && teammate.is_standing()
                    && teammate.in_stance
                    && teammate
                        .square()
                        .is_some_and(|other| other.is_orthogonally_adjacent(square))
            })
            .count();
        StanceSupport {
            itself: goblin.is_standing() && goblin.in_stance,
            neighbours: u8::try_from(braced_neighbours).unwrap_or(u8::MAX),
        }
    }

    pub(crate) fn goblin_mut(&mut self, id: GoblinId) -> Option<&mut Goblin> {
        self.goblins.iter_mut().find(|goblin| goblin.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn goblin(id: &str, presence: Presence) -> Goblin {
        Goblin {
            id: id.parse().unwrap(),
            archetype: Archetype::Bruiser,
            characteristics: Characteristics {
                attack: 4,
                defense: 4,
                agility: 2,
                health: 6,
                movement: 3,
            },
            health: 6,
            presence,
            in_stance: false,
            steps_left: 3,
            has_action_left: true,
        }
    }

    fn standing(id: &str, x: u8, y: u8) -> Goblin {
        goblin(id, Presence::Standing(Square::new(x, y)))
    }

    fn braced(id: &str, x: u8, y: u8) -> Goblin {
        Goblin {
            in_stance: true,
            ..standing(id, x, y)
        }
    }

    fn state_with(goblins: Vec<Goblin>) -> GameState {
        GameState {
            rosters: vec![Roster::DEFAULT, Roster::DEFAULT],
            goblins,
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
        }
    }

    fn id(text: &str) -> GoblinId {
        text.parse().unwrap()
    }

    #[test]
    fn given_standing_presence_when_asking_its_square_then_that_square() {
        assert_eq!(
            Presence::Standing(Square::new(3, 4)).square(),
            Some(Square::new(3, 4))
        );
    }

    #[test]
    fn given_knocked_out_presence_when_asking_its_square_then_none() {
        assert_eq!(Presence::KnockedOut.square(), None);
    }

    #[test]
    fn given_downed_goblin_when_asking_if_it_can_act_then_no() {
        assert!(!goblin("H1", Presence::Down(Square::new(3, 4))).can_act());
    }

    #[test]
    fn given_goblin_that_spent_its_turn_when_asking_if_it_can_act_then_no() {
        let mut tired = standing("H1", 3, 4);
        tired.spend_its_turn();
        assert!(!tired.can_act());
    }

    #[test]
    fn given_goblin_with_steps_but_no_action_left_when_asking_if_it_can_act_then_yes() {
        let runner = Goblin {
            has_action_left: false,
            ..standing("H1", 3, 4)
        };
        assert!(runner.can_act());
    }

    #[test]
    fn given_goblin_with_its_action_but_no_step_left_when_asking_if_it_can_act_then_yes() {
        let hitter = Goblin {
            steps_left: 0,
            ..standing("H1", 3, 4)
        };
        assert!(hitter.can_act());
    }

    #[test]
    fn given_goblin_that_spent_its_turn_when_its_turn_is_refreshed_then_its_whole_move_and_action_are_back()
     {
        let mut rested = standing("H1", 3, 4);
        rested.spend_its_turn();
        rested.refresh_its_turn();
        assert_eq!((rested.steps_left, rested.has_action_left), (3, true));
    }

    fn support(itself: bool, neighbours: u8) -> StanceSupport {
        StanceSupport { itself, neighbours }
    }

    #[test]
    fn given_goblin_in_stance_alone_when_asking_its_support_then_only_itself() {
        assert_eq!(
            state_with(vec![braced("H1", 5, 5)]).stance_support(id("H1")),
            support(true, 0)
        );
    }

    #[test]
    fn given_goblin_between_two_braced_teammates_when_asking_its_support_then_two_neighbours() {
        let state = state_with(vec![
            standing("H1", 5, 5),
            braced("H2", 5, 4),
            braced("H3", 5, 6),
        ]);
        assert_eq!(state.stance_support(id("H1")), support(false, 2));
    }

    #[test]
    fn given_braced_goblin_next_to_a_braced_teammate_when_asking_its_support_then_itself_and_one_neighbour()
     {
        let state = state_with(vec![braced("H1", 5, 5), braced("H2", 6, 5)]);
        assert_eq!(state.stance_support(id("H1")), support(true, 1));
    }

    #[test]
    fn given_braced_teammate_on_a_diagonal_when_asking_the_support_then_none() {
        let state = state_with(vec![standing("H1", 5, 5), braced("H2", 6, 6)]);
        assert_eq!(state.stance_support(id("H1")), support(false, 0));
    }

    #[test]
    fn given_braced_opponent_next_to_the_goblin_when_asking_its_support_then_none() {
        let state = state_with(vec![standing("H1", 5, 5), braced("A1", 6, 5)]);
        assert_eq!(state.stance_support(id("H1")), support(false, 0));
    }

    #[test]
    fn given_downed_goblin_next_to_a_braced_teammate_when_asking_its_support_then_one_neighbour() {
        let state = state_with(vec![
            goblin("H1", Presence::Down(Square::new(5, 5))),
            braced("H2", 5, 4),
        ]);
        assert_eq!(state.stance_support(id("H1")), support(false, 1));
    }

    #[test]
    fn given_ball_carried_when_asking_where_the_ball_is_then_the_carrier_square() {
        let mut state = state_with(vec![standing("A4", 9, 3)]);
        state.ball = Ball::Carried { by: id("A4") };
        assert_eq!(state.ball_square(), Some(Square::new(9, 3)));
    }

    #[test]
    fn given_two_standing_and_one_downed_opponents_around_when_listing_then_two() {
        let state = state_with(vec![
            standing("A1", 5, 4),
            standing("A2", 4, 5),
            goblin("A3", Presence::Down(Square::new(6, 5))),
            standing("H2", 5, 6),
        ]);
        assert_eq!(
            state
                .standing_opponents_next_to(Square::new(5, 5), TeamId::HOME)
                .count(),
            2
        );
    }

    #[test]
    fn given_goblin_lying_on_a_square_when_looking_up_the_square_then_found() {
        let state = state_with(vec![goblin("A3", Presence::Down(Square::new(6, 5)))]);
        assert_eq!(
            state.goblin_at(Square::new(6, 5)).map(|goblin| goblin.id),
            Some(id("A3"))
        );
    }
}
