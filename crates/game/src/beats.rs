//! What the bowl shows of the match, beat by beat: steps, falls, ball flights, played in order.

use goblins_rules::event::GameEvent;
use goblins_rules::roster::GoblinId;
use goblins_rules::square::Square;

/// One thing the bowl shows happening, played once the previous one is over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
}

/// What the bowl shows of `events`, in order.
pub fn beats_from_events(events: &[GameEvent]) -> Vec<Beat> {
    events
        .iter()
        .filter_map(|event| match *event {
            GameEvent::Moved { goblin, to, .. } => Some(Beat::Step { goblin, to }),
            GameEvent::KnockedDown { goblin, .. } => Some(Beat::Fall { goblin }),
            GameEvent::StoodUp { goblin } => Some(Beat::Rise { goblin }),
            GameEvent::KnockedOut { goblin } => Some(Beat::Vanish { goblin }),
            GameEvent::KickedOff { .. } => Some(Beat::LineUp),
            GameEvent::BallDropped { at } => Some(Beat::BallTo { square: at }),
            GameEvent::BallBounced { to, .. } => Some(Beat::BallTo { square: to }),
            GameEvent::PickAttempted { goblin, roll }
            | GameEvent::InterceptAttempted { goblin, roll }
                if roll.success =>
            {
                Some(Beat::BallToGoblin { goblin })
            }
            GameEvent::CatchAttempted { goblin, .. } => Some(Beat::BallToGoblin { goblin }),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use goblins_rules::team::TeamId;

    fn roll(success: bool) -> goblins_rules::event::BallRoll {
        goblins_rules::event::BallRoll {
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
    fn given_a_successful_pick_when_turning_events_into_beats_then_the_ball_reaches_the_picker() {
        let events = [GameEvent::PickAttempted {
            goblin: goblin("A6"),
            roll: roll(true),
        }];
        assert_eq!(
            beats_from_events(&events),
            vec![Beat::BallToGoblin {
                goblin: goblin("A6")
            }]
        );
    }

    #[test]
    fn given_a_failed_pick_when_turning_events_into_beats_then_the_ball_stays_put() {
        let events = [GameEvent::PickAttempted {
            goblin: goblin("A6"),
            roll: roll(false),
        }];
        assert_eq!(beats_from_events(&events), Vec::new());
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
    fn given_a_caught_throw_when_turning_events_into_beats_then_the_ball_reaches_the_receiver() {
        let events = [
            GameEvent::ThrowAttempted {
                thrower: goblin("H9"),
                receiver: goblin("H5"),
                roll: roll(true),
            },
            GameEvent::CatchAttempted {
                goblin: goblin("H5"),
                roll: roll(true),
            },
        ];
        assert_eq!(
            beats_from_events(&events),
            vec![Beat::BallToGoblin {
                goblin: goblin("H5")
            }]
        );
    }

    #[test]
    fn given_an_interception_when_turning_events_into_beats_then_the_ball_reaches_the_interceptor()
    {
        let events = [
            GameEvent::ThrowAttempted {
                thrower: goblin("H9"),
                receiver: goblin("H5"),
                roll: roll(true),
            },
            GameEvent::InterceptAttempted {
                goblin: goblin("A2"),
                roll: roll(true),
            },
        ];
        assert_eq!(
            beats_from_events(&events),
            vec![Beat::BallToGoblin {
                goblin: goblin("A2")
            }]
        );
    }
}
