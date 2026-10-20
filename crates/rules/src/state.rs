//! The state of a match: goblins, ball, score and per-team turn counters.

use serde::{Deserialize, Serialize};

use crate::characteristics::{Archetype, Characteristics};
use crate::command::{Action, ActionKind, Activation, ThrowStyle};
use crate::roster::{GoblinId, Roster};
use crate::ruleset::Ruleset;
use crate::square::{Reach, Square};
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
    /// The kinds of action it took this turn, under rules where each kind comes once per turn.
    #[serde(default, skip_serializing_if = "ActionsUsed::is_none")]
    pub actions_used: ActionsUsed,
    /// Where it stands with focusing: gathering itself this turn, or focused, +1 to its throws.
    #[serde(default, skip_serializing_if = "Focus::is_unfocused")]
    pub focus: Focus,
}

/// A goblin's focus on its throws.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Focus {
    /// No focus.
    #[default]
    Unfocused,
    /// It focuses this turn, standing still.
    Focusing,
    /// It focused on its team's last turn: +1 to its throws this turn.
    Focused,
}

impl Focus {
    /// Whether it carries no focus at all.
    pub fn is_unfocused(&self) -> bool {
        *self == Focus::Unfocused
    }

    /// Its focus once its team's next turn starts: focusing becomes focused, focused wears off.
    pub fn at_the_next_turn(self) -> Focus {
        match self {
            Focus::Focusing => Focus::Focused,
            Focus::Unfocused | Focus::Focused => Focus::Unfocused,
        }
    }

    /// What it adds to its throw rolls.
    pub fn throw_bonus(self) -> u8 {
        u8::from(self == Focus::Focused)
    }
}

/// The kinds of action a goblin took this turn, where each kind comes once per turn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionsUsed {
    /// It assaulted.
    #[serde(default)]
    pub assault: bool,
    /// It threw the ball or handed it off.
    #[serde(default)]
    pub throw: bool,
    /// It braced in Stance.
    #[serde(default)]
    pub stance: bool,
    /// It focused.
    #[serde(default)]
    pub focus: bool,
}

impl ActionsUsed {
    /// Whether it took no action yet.
    pub fn is_none(&self) -> bool {
        *self == ActionsUsed::default()
    }

    /// Whether `action`'s kind was already taken.
    pub fn includes(&self, action: Action) -> bool {
        match action {
            Action::Assault { .. } => self.assault,
            Action::Throw { .. } => self.throw,
            Action::Stance => self.stance,
            Action::Focus => self.focus,
        }
    }

    fn record(&mut self, action: Action) {
        match action {
            Action::Assault { .. } => self.assault = true,
            Action::Throw { .. } => self.throw = true,
            Action::Stance => self.stance = true,
            Action::Focus => self.focus = true,
        }
    }

    fn any_left(&self, ruleset: &Ruleset) -> bool {
        !self.assault
            || !self.throw
            || (ruleset.goblins_can_take_stance && !self.stance)
            || (focus_stands_alone(ruleset) && !self.focus)
    }
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

    /// Whether it may still take `action` this turn: its kind not taken yet where each kind comes
    /// once per turn, otherwise its one action unused.
    pub fn may_take(&self, action: Action, ruleset: &Ruleset) -> bool {
        self.has_action_left
            && !(ruleset.one_action_of_each_kind && self.actions_used.includes(action))
    }

    /// Whether it may still take `action` once it walked `steps_walked` squares: standing, the
    /// action in the rules and still its to take, and a step left to pay for it where actions cost
    /// steps; a focus takes its whole Move, so it must not have walked or spent any step.
    pub fn may_act_after(&self, action: Action, steps_walked: usize, ruleset: &Ruleset) -> bool {
        let in_the_rules = match action {
            Action::Stance => ruleset.goblins_can_take_stance,
            Action::Focus => {
                focus_stands_alone(ruleset)
                    && steps_walked == 0
                    && self.steps_left == self.characteristics.movement
            }
            Action::Assault { .. } | Action::Throw { .. } => true,
        };
        self.is_standing()
            && in_the_rules
            && self.may_take(action, ruleset)
            && (!ruleset.actions_cost_a_step || steps_walked < usize::from(self.steps_left))
    }

    /// What a focused throw adds to the throw roll once the goblin walked `steps_walked` squares:
    /// +1 per step it sacrifices, every step it has left after paying for the throw where actions
    /// cost a step.
    pub fn focused_throw_bonus(&self, steps_walked: usize, ruleset: &Ruleset) -> u8 {
        let walked = u8::try_from(steps_walked).unwrap_or(u8::MAX);
        self.steps_left
            .saturating_sub(walked)
            .saturating_sub(u8::from(ruleset.actions_cost_a_step))
    }

    /// Gives it a whole turn: every step of its Move and its action.
    pub fn refresh_its_turn(&mut self) {
        self.steps_left = self.characteristics.movement;
        self.has_action_left = true;
        self.actions_used = ActionsUsed::default();
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
        if ruleset.one_action_of_each_kind {
            self.actions_used.record(action);
            self.has_action_left = self.actions_used.any_left(ruleset);
        } else {
            self.has_action_left = false;
        }
        self.steps_left = match action {
            Action::Throw {
                style: ThrowStyle::Chaos | ThrowStyle::Focused,
                ..
            }
            | Action::Focus => 0,
            Action::Stance if ruleset.actions_cost_a_step => 0,
            Action::Assault { .. } | Action::Throw { .. } if ruleset.actions_cost_a_step => {
                self.steps_left.saturating_sub(1)
            }
            Action::Assault { .. } | Action::Throw { .. } | Action::Stance => self.steps_left,
        };
    }
}

/// Whether focus is an action of its own, a whole turn standing still for +1 to the next turn's
/// throws, rather than part of a Thrower's throw.
fn focus_stands_alone(ruleset: &Ruleset) -> bool {
    ruleset.throws_have_styles && !ruleset.focused_throws_spend_steps_left
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

    /// Standing goblins opposing `team` within `reach` of `square`.
    pub fn standing_opponents_in_reach(
        &self,
        square: Square,
        team: TeamId,
        reach: Reach,
    ) -> impl Iterator<Item = &Goblin> + '_ {
        self.goblins.iter().filter(move |goblin| {
            goblin.team() != team
                && goblin.is_standing()
                && goblin
                    .square()
                    .is_some_and(|other| other.is_within(square, reach))
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

    /// The kinds of action goblin `id` may still take this turn from where it stands: the ball
    /// barring a carrier's assault or Stance where the rules say so, and a focus left to the
    /// carrier alone.
    pub fn action_kinds_left(&self, id: GoblinId, ruleset: &Ruleset) -> Vec<ActionKind> {
        let Some(goblin) = self.goblin(id) else {
            return Vec::new();
        };
        let carries_the_ball = self.carrier() == Some(id);
        ActionKind::ALL
            .into_iter()
            .filter(|kind| goblin.may_act_after(kind.sample(id), 0, ruleset))
            .filter(|kind| match kind {
                ActionKind::Assault => ruleset.carriers_can_assault || !carries_the_ball,
                ActionKind::Stance => ruleset.carriers_can_brace || !carries_the_ball,
                ActionKind::Focus => carries_the_ball,
                ActionKind::Throw => true,
            })
            .collect()
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
            focus: Default::default(),
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
            actions_used: ActionsUsed::default(),
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

    fn rules_where_actions_cost_a_step() -> Ruleset {
        Ruleset {
            goblins_can_take_stance: true,
            actions_cost_a_step: true,
            ..Ruleset::default()
        }
    }

    #[test]
    fn given_actions_that_cost_a_step_when_a_walk_would_use_every_step_left_then_no_stance_after_it()
     {
        let runner = standing("H1", 3, 4);
        assert!(!runner.may_act_after(Action::Stance, 3, &rules_where_actions_cost_a_step()));
    }

    #[test]
    fn given_actions_that_cost_a_step_when_a_walk_leaves_a_step_then_a_stance_after_it() {
        let runner = standing("H1", 3, 4);
        assert!(runner.may_act_after(Action::Stance, 2, &rules_where_actions_cost_a_step()));
    }

    #[test]
    fn given_rules_without_stance_when_asking_for_a_stance_then_no() {
        let runner = standing("H1", 3, 4);
        let without_stance = Ruleset {
            goblins_can_take_stance: false,
            ..Ruleset::default()
        };
        assert!(!runner.may_act_after(Action::Stance, 0, &without_stance));
    }

    fn rules_of_carriers_that_only_throw_or_focus() -> Ruleset {
        Ruleset {
            goblins_can_take_stance: true,
            carriers_can_assault: false,
            carriers_can_brace: false,
            throws_have_styles: true,
            focused_throws_spend_steps_left: false,
            ..Ruleset::default()
        }
    }

    #[test]
    fn given_the_shipped_rules_when_listing_the_carrier_s_actions_then_a_throw_alone() {
        let mut state = state_with(vec![standing("H1", 3, 4), standing("H2", 3, 6)]);
        state.ball = Ball::Carried { by: id("H1") };
        assert_eq!(
            state.action_kinds_left(id("H1"), &Ruleset::default()),
            [ActionKind::Throw]
        );
    }

    #[test]
    fn given_carriers_that_only_throw_or_focus_when_listing_the_carrier_s_actions_then_a_throw_and_a_focus()
     {
        let mut state = state_with(vec![standing("H1", 3, 4), standing("H2", 3, 6)]);
        state.ball = Ball::Carried { by: id("H1") };
        assert_eq!(
            state.action_kinds_left(id("H1"), &rules_of_carriers_that_only_throw_or_focus()),
            [ActionKind::Throw, ActionKind::Focus]
        );
    }

    #[test]
    fn given_carriers_that_only_throw_or_focus_when_listing_a_teammate_s_actions_then_an_assault_a_throw_and_a_stance()
     {
        let mut state = state_with(vec![standing("H1", 3, 4), standing("H2", 3, 6)]);
        state.ball = Ball::Carried { by: id("H1") };
        assert_eq!(
            state.action_kinds_left(id("H2"), &rules_of_carriers_that_only_throw_or_focus()),
            [ActionKind::Assault, ActionKind::Throw, ActionKind::Stance]
        );
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
                .standing_opponents_in_reach(Square::new(5, 5), TeamId::HOME, Reach::Orthogonal)
                .count(),
            2
        );
    }

    #[test]
    fn given_an_opponent_on_a_diagonal_when_the_reach_covers_all_around_then_listed() {
        let state = state_with(vec![standing("A1", 6, 6), standing("H2", 5, 6)]);
        assert_eq!(
            state
                .standing_opponents_in_reach(Square::new(5, 5), TeamId::HOME, Reach::AllAround)
                .count(),
            1
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
