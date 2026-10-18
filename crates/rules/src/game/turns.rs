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
            goblin.refresh_its_turn();
            goblin.in_stance = false;
            goblin.focus = goblin.focus.at_the_next_turn();
            if let Presence::Down(square) = goblin.presence {
                goblin.presence = Presence::Standing(square);
                goblin.spend_its_turn();
                stood_up.push(goblin.id);
            }
        }
        events.extend(
            stood_up
                .into_iter()
                .map(|goblin| GameEvent::StoodUp { goblin }),
        );
        if self.ruleset.turn_ends_when_no_goblin_can_act && !self.has_goblin_able_to_act(team) {
            self.finish_turn(TurnEndReason::NoGoblinCanAct, events);
        }
    }

    pub(super) fn finish_turn(&mut self, reason: TurnEndReason, events: &mut Vec<GameEvent>) {
        let Some(team) = self.state.active_team() else {
            return;
        };
        events.push(GameEvent::TurnEnded { team, reason });
        match self.next_team_to_play(team.duel_opponent(), team) {
            Some(next) => self.start_turn(next, events),
            None => self.end_half(events),
        }
    }

    fn next_team_to_play(&self, preferred: TeamId, fallback: TeamId) -> Option<TeamId> {
        [preferred, fallback].into_iter().find(|team| {
            self.state
                .turns_played
                .get(team.index())
                .is_some_and(|played| *played < self.ruleset.turns_per_half)
        })
    }

    fn end_half(&mut self, events: &mut Vec<GameEvent>) {
        events.push(GameEvent::HalfEnded {
            half: self.state.half,
        });
        if self.state.half < self.ruleset.halves {
            self.state.half += 1;
            self.state.turns_played = vec![0; self.state.turns_played.len()];
            let receiving_team = self.state.opening_team.duel_opponent();
            self.kick_off(receiving_team, events);
            self.start_turn(receiving_team, events);
        } else {
            self.state.phase = Phase::Finished;
            events.push(GameEvent::GameEnded {
                score: self.state.score.clone(),
            });
        }
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
        match self.next_team_to_play(team.duel_opponent(), team) {
            Some(receiving_team) => {
                self.kick_off(receiving_team, events);
                self.start_turn(receiving_team, events);
            }
            None => self.end_half(events),
        }
    }
}
