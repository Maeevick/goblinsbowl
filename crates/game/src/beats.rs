//! What the bowl shows of the match, beat by beat: steps, falls, ball flights, played in order.

use bevy::prelude::Message;
use goblins_rules::event::{AgilityRoll, ContestRoll, DodgeRoll, GameEvent};
use goblins_rules::roster::GoblinId;
use goblins_rules::square::Square;
use goblins_rules::team::TeamId;

/// A headline the bowl holds on, so it is read before anything that happened after it plays.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Headline {
    /// What it says.
    pub text: String,
    /// The team it is about, which colours it.
    pub team: Option<TeamId>,
}

/// One thing the bowl shows happening, played once the previous one is over.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Beat {
    /// A goblin steps into a square.
    Step {
        /// The goblin walking.
        goblin: GoblinId,
        /// The square it enters.
        to: Square,
    },
    /// A goblin falls to the ground.
    Fall {
        /// The goblin falling.
        goblin: GoblinId,
    },
    /// A downed goblin gets back on its feet.
    Rise {
        /// The goblin rising.
        goblin: GoblinId,
    },
    /// A knocked-out goblin leaves the bowl.
    Vanish {
        /// The goblin leaving.
        goblin: GoblinId,
    },
    /// Dice are rolled for a goblin.
    Roll {
        /// The goblin rolling.
        goblin: GoblinId,
        /// What was rolled.
        roll: ShownRoll,
    },
    /// Every goblin walks back to its kick-off square.
    LineUp,
    /// The ball comes to rest on a square.
    BallTo {
        /// Where it rests.
        square: Square,
    },
    /// The ball reaches a goblin's hands.
    BallToGoblin {
        /// The goblin reached.
        goblin: GoblinId,
    },
    /// A headline shown big while the bowl waits.
    Headline(Headline),
}

/// A roll worth showing, by kind of action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShownRoll {
    /// Slipping out of the enemies' reach.
    Dodge(DodgeRoll),
    /// Hitting an opponent.
    Assault(ContestRoll),
    /// Picking up the loose ball.
    Pick(AgilityRoll),
    /// Throwing the ball.
    Throw(AgilityRoll),
    /// Catching the ball.
    Catch(AgilityRoll),
    /// Snatching a ball thrown by the other team.
    Intercept(AgilityRoll),
}

impl ShownRoll {
    /// Whether the goblin rolling got what it wanted.
    pub fn succeeded(&self) -> bool {
        match self {
            ShownRoll::Dodge(dodge) => dodge.succeeded(),
            ShownRoll::Assault(contest) => contest.actor_won(),
            ShownRoll::Pick(roll)
            | ShownRoll::Throw(roll)
            | ShownRoll::Catch(roll)
            | ShownRoll::Intercept(roll) => roll.success,
        }
    }
}

/// A roll the bowl starts showing, in the order of the match.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct RollShown {
    /// The goblin rolling.
    pub goblin: GoblinId,
    /// What was rolled.
    pub roll: ShownRoll,
}

/// What the bowl shows of `events`, in order, holding on the headline `headline_of` finds for an
/// event before showing what came after it.
pub fn timeline_of(
    events: &[GameEvent],
    headline_of: impl Fn(&GameEvent) -> Option<Headline>,
) -> Vec<Beat> {
    events
        .iter()
        .flat_map(|event| {
            beats_from_events(std::slice::from_ref(event))
                .into_iter()
                .chain(headline_of(event).map(Beat::Headline))
        })
        .collect()
}

/// What the bowl shows of `events`, in order.
pub fn beats_from_events(events: &[GameEvent]) -> Vec<Beat> {
    let mut beats = Vec::new();
    for event in events {
        match *event {
            GameEvent::Moved { goblin, to, .. } => beats.push(Beat::Step { goblin, to }),
            GameEvent::KnockedDown { goblin, .. } => beats.push(Beat::Fall { goblin }),
            GameEvent::StoodUp { goblin } => beats.push(Beat::Rise { goblin }),
            GameEvent::KnockedOut { goblin } => beats.push(Beat::Vanish { goblin }),
            GameEvent::KickedOff { .. } => beats.push(Beat::LineUp),
            GameEvent::BallDropped { at } => beats.push(Beat::BallTo { square: at }),
            GameEvent::BallBounced { to, .. } => beats.push(Beat::BallTo { square: to }),
            GameEvent::DodgeAttempted { goblin, roll } => beats.push(Beat::Roll {
                goblin,
                roll: ShownRoll::Dodge(roll),
            }),
            GameEvent::AssaultResolved {
                attacker, contest, ..
            } => beats.push(Beat::Roll {
                goblin: attacker,
                roll: ShownRoll::Assault(contest),
            }),
            GameEvent::PickAttempted { goblin, roll } => {
                beats.push(Beat::Roll {
                    goblin,
                    roll: ShownRoll::Pick(roll),
                });
                if roll.success {
                    beats.push(Beat::BallToGoblin { goblin });
                }
            }
            GameEvent::ThrowAttempted { thrower, roll, .. } => beats.push(Beat::Roll {
                goblin: thrower,
                roll: ShownRoll::Throw(roll),
            }),
            GameEvent::InterceptAttempted { goblin, roll } => {
                beats.push(Beat::Roll {
                    goblin,
                    roll: ShownRoll::Intercept(roll),
                });
                if roll.success {
                    beats.push(Beat::BallToGoblin { goblin });
                }
            }
            GameEvent::CatchAttempted { goblin, roll } => {
                beats.push(Beat::BallToGoblin { goblin });
                beats.push(Beat::Roll {
                    goblin,
                    roll: ShownRoll::Catch(roll),
                });
            }
            _ => {}
        }
    }
    beats
}

#[cfg(test)]
mod tests {
    use super::*;
    use goblins_rules::event::TurnEndReason;

    fn roll(success: bool) -> goblins_rules::event::AgilityRoll {
        goblins_rules::event::AgilityRoll {
            faces: 4,
            rolled: if success { 3 } else { 1 },
            malus: 0,
            target: 2,
            success,
        }
    }

    fn goblin(text: &str) -> GoblinId {
        text.parse().unwrap()
    }

    fn fumble_headline(event: &GameEvent) -> Option<Headline> {
        matches!(
            event,
            GameEvent::TurnEnded {
                reason: TurnEndReason::Fumble,
                ..
            }
        )
        .then(|| Headline {
            text: "Fumble !".to_string(),
            team: Some(TeamId::AWAY),
        })
    }

    #[test]
    fn given_a_fumble_then_the_next_team_getting_up_when_laying_out_the_timeline_then_the_headline_holds_the_bowl_in_between()
     {
        let events = [
            GameEvent::PickAttempted {
                goblin: goblin("A4"),
                roll: roll(false),
            },
            GameEvent::TurnEnded {
                team: TeamId::AWAY,
                reason: TurnEndReason::Fumble,
            },
            GameEvent::StoodUp {
                goblin: goblin("H2"),
            },
        ];
        assert_eq!(
            timeline_of(&events, fumble_headline),
            vec![
                Beat::Roll {
                    goblin: goblin("A4"),
                    roll: ShownRoll::Pick(roll(false))
                },
                Beat::Headline(Headline {
                    text: "Fumble !".to_string(),
                    team: Some(TeamId::AWAY)
                }),
                Beat::Rise {
                    goblin: goblin("H2")
                },
            ]
        );
    }

    #[test]
    fn given_two_moves_when_turning_events_into_beats_then_two_steps_in_order() {
        let events = [
            GameEvent::Moved {
                goblin: goblin("H5"),
                from: Square::new(4, 4),
                to: Square::new(5, 4),
            },
            GameEvent::Moved {
                goblin: goblin("H5"),
                from: Square::new(5, 4),
                to: Square::new(6, 4),
            },
        ];
        assert_eq!(
            beats_from_events(&events),
            vec![
                Beat::Step {
                    goblin: goblin("H5"),
                    to: Square::new(5, 4)
                },
                Beat::Step {
                    goblin: goblin("H5"),
                    to: Square::new(6, 4)
                },
            ]
        );
    }

    #[test]
    fn given_a_knockdown_when_turning_events_into_beats_then_the_goblin_falls() {
        let events = [GameEvent::KnockedDown {
            goblin: goblin("H2"),
            at: Square::new(6, 6),
        }];
        assert_eq!(
            beats_from_events(&events),
            vec![Beat::Fall {
                goblin: goblin("H2")
            }]
        );
    }

    #[test]
    fn given_a_goblin_standing_up_when_turning_events_into_beats_then_it_rises() {
        let events = [GameEvent::StoodUp {
            goblin: goblin("H2"),
        }];
        assert_eq!(
            beats_from_events(&events),
            vec![Beat::Rise {
                goblin: goblin("H2")
            }]
        );
    }

    #[test]
    fn given_a_knock_out_when_turning_events_into_beats_then_the_goblin_vanishes() {
        let events = [GameEvent::KnockedOut {
            goblin: goblin("A5"),
        }];
        assert_eq!(
            beats_from_events(&events),
            vec![Beat::Vanish {
                goblin: goblin("A5")
            }]
        );
    }

    #[test]
    fn given_a_successful_pick_when_turning_events_into_beats_then_the_roll_shows_before_the_ball_reaches_the_picker()
     {
        let events = [GameEvent::PickAttempted {
            goblin: goblin("A6"),
            roll: roll(true),
        }];
        assert_eq!(
            beats_from_events(&events),
            vec![
                Beat::Roll {
                    goblin: goblin("A6"),
                    roll: ShownRoll::Pick(roll(true))
                },
                Beat::BallToGoblin {
                    goblin: goblin("A6")
                }
            ]
        );
    }

    #[test]
    fn given_a_failed_pick_when_turning_events_into_beats_then_only_the_roll_shows() {
        let events = [GameEvent::PickAttempted {
            goblin: goblin("A6"),
            roll: roll(false),
        }];
        assert_eq!(
            beats_from_events(&events),
            vec![Beat::Roll {
                goblin: goblin("A6"),
                roll: ShownRoll::Pick(roll(false))
            }]
        );
    }

    #[test]
    fn given_a_bounce_when_turning_events_into_beats_then_the_ball_rests_where_it_bounced() {
        let events = [GameEvent::BallBounced {
            from: Square::new(7, 6),
            to: Square::new(7, 5),
        }];
        assert_eq!(
            beats_from_events(&events),
            vec![Beat::BallTo {
                square: Square::new(7, 5)
            }]
        );
    }

    #[test]
    fn given_a_kick_off_when_turning_events_into_beats_then_the_teams_line_up_before_the_ball_drops()
     {
        let events = [
            GameEvent::KickedOff {
                receiving_team: TeamId::HOME,
            },
            GameEvent::BallDropped {
                at: Square::new(7, 7),
            },
        ];
        assert_eq!(
            beats_from_events(&events),
            vec![
                Beat::LineUp,
                Beat::BallTo {
                    square: Square::new(7, 7)
                }
            ]
        );
    }

    #[test]
    fn given_a_caught_throw_when_turning_events_into_beats_then_the_ball_flies_before_the_catch_shows()
     {
        let events = [
            GameEvent::ThrowAttempted {
                thrower: goblin("H9"),
                receiver: goblin("H5"),
                roll: roll(true),
                distance: 5,
            },
            GameEvent::CatchAttempted {
                goblin: goblin("H5"),
                roll: roll(true),
            },
        ];
        assert_eq!(
            beats_from_events(&events),
            vec![
                Beat::Roll {
                    goblin: goblin("H9"),
                    roll: ShownRoll::Throw(roll(true))
                },
                Beat::BallToGoblin {
                    goblin: goblin("H5")
                },
                Beat::Roll {
                    goblin: goblin("H5"),
                    roll: ShownRoll::Catch(roll(true))
                },
            ]
        );
    }

    #[test]
    fn given_an_interception_when_turning_events_into_beats_then_the_roll_shows_before_the_ball_reaches_the_interceptor()
     {
        let events = [
            GameEvent::ThrowAttempted {
                thrower: goblin("H9"),
                receiver: goblin("H5"),
                roll: roll(true),
                distance: 5,
            },
            GameEvent::InterceptAttempted {
                goblin: goblin("A2"),
                roll: roll(true),
            },
        ];
        assert_eq!(
            beats_from_events(&events),
            vec![
                Beat::Roll {
                    goblin: goblin("H9"),
                    roll: ShownRoll::Throw(roll(true))
                },
                Beat::Roll {
                    goblin: goblin("A2"),
                    roll: ShownRoll::Intercept(roll(true))
                },
                Beat::BallToGoblin {
                    goblin: goblin("A2")
                },
            ]
        );
    }

    fn contest(margin: i8) -> ContestRoll {
        ContestRoll {
            actor_modifier: 0,
            actor_faces: 4,
            actor_rolled: 2,
            opponent_faces: 4,
            opponent_rolled: u8::try_from(2 - margin).unwrap(),
            opponent_modifier: 0,
            margin,
        }
    }

    #[test]
    fn given_a_dodge_when_turning_events_into_beats_then_its_roll_shows_on_the_dodger() {
        let events = [GameEvent::DodgeAttempted {
            goblin: goblin("H5"),
            roll: contest_dodge(1),
        }];
        assert_eq!(
            beats_from_events(&events),
            vec![Beat::Roll {
                goblin: goblin("H5"),
                roll: ShownRoll::Dodge(contest_dodge(1))
            }]
        );
    }

    #[test]
    fn given_an_assault_when_turning_events_into_beats_then_its_roll_shows_on_the_attacker() {
        let events = [GameEvent::AssaultResolved {
            style: Default::default(),
            attacker: goblin("H1"),
            defender: goblin("A1"),
            contest: contest(-2),
            defender_braced: false,
        }];
        assert_eq!(
            beats_from_events(&events),
            vec![Beat::Roll {
                goblin: goblin("H1"),
                roll: ShownRoll::Assault(contest(-2))
            }]
        );
    }

    #[test]
    fn given_a_lost_contest_when_asking_if_the_roll_succeeded_then_no() {
        assert!(!ShownRoll::Assault(contest(-1)).succeeded());
    }

    #[test]
    fn given_a_tied_contest_when_asking_if_the_roll_succeeded_then_yes_for_the_actor() {
        assert!(ShownRoll::Dodge(contest_dodge(0)).succeeded());
    }

    #[test]
    fn given_an_agility_dodge_short_of_its_target_when_asking_if_the_roll_succeeded_then_no() {
        let dodge = DodgeRoll::Agility(AgilityRoll {
            faces: 4,
            rolled: 3,
            malus: 2,
            target: 2,
            success: false,
        });
        assert!(!ShownRoll::Dodge(dodge).succeeded());
    }

    fn contest_dodge(margin: i8) -> DodgeRoll {
        DodgeRoll::Contest {
            tackler: goblin("A1"),
            contest: contest(margin),
        }
    }

    #[test]
    fn given_a_failed_catch_when_asking_if_the_roll_succeeded_then_no() {
        assert!(!ShownRoll::Catch(roll(false)).succeeded());
    }
}
