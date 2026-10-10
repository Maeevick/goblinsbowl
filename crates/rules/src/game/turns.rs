//! Turn sequencing: starting and ending turns.

use crate::dice::DiceRoller;
use crate::event::{GameEvent, TurnEndReason};
use crate::roster::GoblinId;
use crate::state::{Phase, Presence};
use crate::team::TeamId;

use super::Game;

impl<D: DiceRoller> Game<D> {
    pub(super) fn start_turn(&mut self, team: TeamId, events: &mut Vec<GameEvent>) {
        let turn = self.state.turns_played[team.index()].saturating_add(1);
        self.state.turns_played[team.index()] = turn;
        self.state.phase = Phase::Playing { active_team: team };
        events.push(GameEvent::TurnStarted {
            team,
            half: self.state.half,
            turn,
        });
        let mut stood_up = Vec::new();
        for goblin in self
            .state
            .goblins
            .iter_mut()
            .filter(|goblin| goblin.team() == team)
        {
            goblin.has_acted = false;
            goblin.in_stance = false;
            if let Presence::Down(square) = goblin.presence {
                goblin.presence = Presence::Standing(square);
                goblin.has_acted = true;
                stood_up.push(goblin.id);
            }
        }
        events.extend(
            stood_up
                .into_iter()
                .map(|goblin| GameEvent::StoodUp { goblin }),
        );
    }

    pub(super) fn finish_turn(&mut self, reason: TurnEndReason, events: &mut Vec<GameEvent>) {
        let Some(team) = self.state.active_team() else {
            return;
        };
        events.push(GameEvent::TurnEnded { team, reason });
        self.start_turn(team.duel_opponent(), events);
    }

    pub(super) fn has_goblin_able_to_act(&self, team: TeamId) -> bool {
        self.state.goblins_of(team).any(|goblin| goblin.can_act())
    }

    pub(super) fn scoring_carrier(&self) -> Option<(TeamId, GoblinId)> {
        let carrier = self.state.carrier()?;
        let goblin = self.state.goblin(carrier)?;
        let Presence::Standing(square) = goblin.presence else {
            return None;
        };
        self.arena
            .is_scoring_square_for(goblin.team(), square)
            .then_some((goblin.team(), carrier))
    }

    pub(super) fn score(&mut self, team: TeamId, scorer: GoblinId, events: &mut Vec<GameEvent>) {
        if let Some(points) = self.state.score.get_mut(team.index()) {
            *points = points.saturating_add(1);
        }
        events.push(GameEvent::Scored { team, scorer });
        if let Some(active_team) = self.state.active_team() {
            events.push(GameEvent::TurnEnded {
                team: active_team,
                reason: TurnEndReason::Score,
            });
        }
        let receiving_team = team.duel_opponent();
        self.kick_off(receiving_team, events);
        self.start_turn(receiving_team, events);
    }
}
