//! Bot seats of the local match: a built-in bot plays one command per pause, and only once the bowl
//! has played out and its headlines have been read, so people can follow.

use std::time::Duration;

use bevy::prelude::*;
use goblins_rules::dice::SeededDice;
use goblins_rules::host::SeatKind;
use goblins_rules::policies::Bot;

use crate::goblin_view::BowlAnimation;
use crate::local_match::{MatchOrder, MatchSystems, MatchView};
use crate::start_menu::StartMenu;

/// Plays every bot seat of the local match.
pub struct BotSeatPlugin {
    /// How the bots play.
    pub bot: Bot,
    /// Seed of the bots' decisions, for the random bot.
    pub seed: u64,
    /// Pause before each bot command.
    pub pause: Duration,
}

#[derive(Resource)]
struct BotCoach {
    bot: Bot,
    dice: SeededDice,
    pause: Timer,
}

impl Plugin for BotSeatPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(BotCoach {
            bot: self.bot,
            dice: SeededDice::from_seed(self.seed),
            pause: Timer::new(self.pause, TimerMode::Repeating),
        })
        .add_systems(Update, play_bot_seats.in_set(MatchSystems::Present));
    }
}

fn play_bot_seats(
    time: Res<Time>,
    view: Res<MatchView>,
    animation: Option<Res<BowlAnimation>>,
    menu: Option<Res<StartMenu>>,
    mut coach: ResMut<BotCoach>,
    mut orders: MessageWriter<MatchOrder>,
) {
    if menu.is_some_and(|menu| menu.open) {
        coach.pause.reset();
        return;
    }
    let snapshot = &view.snapshot;
    let bot_team = snapshot
        .state
        .active_team()
        .filter(|team| snapshot.seats.get(team.index()) == Some(&SeatKind::Bot));
    let bowl_busy = animation.is_some_and(|animation| animation.busy);
    let Some(team) = bot_team.filter(|_| !bowl_busy) else {
        coach.pause.reset();
        return;
    };
    if !coach.pause.tick(time.delta()).just_finished() {
        return;
    }
    let bot = coach.bot;
    let command = bot.command(
        &view.arena,
        &view.ruleset,
        &snapshot.state,
        team,
        &mut coach.dice,
    );
    orders.write(MatchOrder::Command { team, command });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::local_match::LocalMatchPlugin;
    use bevy::time::TimeUpdateStrategy;
    use goblins_rules::host::{MatchHost, MatchSetup};
    use goblins_rules::roster::Roster;
    use goblins_rules::ruleset::Ruleset;

    fn app_with(seats: [SeatKind; 2]) -> App {
        let (host, opening_events) = MatchHost::start(MatchSetup {
            ruleset: Ruleset::default(),
            rosters: [Roster::DEFAULT, Roster::DEFAULT],
            seed: 42,
            seats,
            turn_seconds: [60, 60],
            agent_ready_grace_seconds: 15,
        })
        .unwrap();
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            LocalMatchPlugin {
                host,
                opening_events,
                record_to: None,
            },
            BotSeatPlugin {
                bot: Bot::Cautious,
                seed: 7,
                pause: Duration::from_millis(500),
            },
        ))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            250,
        )));
        app.update();
        app
    }

    fn version(app: &App) -> u64 {
        app.world().resource::<MatchView>().snapshot.version
    }

    #[test]
    fn given_bot_seats_when_their_pause_passes_then_a_command_is_played() {
        let mut app = app_with([SeatKind::Bot, SeatKind::Bot]);
        let before = version(&app);
        for _ in 0..4 {
            app.update();
        }
        assert!(version(&app) > before);
    }

    #[test]
    fn given_bot_seats_when_less_than_their_pause_passes_then_nothing_is_played() {
        let mut app = app_with([SeatKind::Bot, SeatKind::Bot]);
        let before = version(&app);
        app.update();
        assert_eq!(version(&app), before);
    }

    #[test]
    fn given_the_bowl_still_playing_out_the_last_moves_when_the_pause_passes_then_the_bot_waits() {
        let mut app = app_with([SeatKind::Bot, SeatKind::Bot]);
        app.insert_resource(BowlAnimation {
            busy: true,
            headline: None,
        });
        let before = version(&app);
        for _ in 0..8 {
            app.update();
        }
        assert_eq!(version(&app), before);
    }

    #[test]
    fn given_human_seats_when_time_passes_then_no_bot_plays_for_them() {
        let mut app = app_with([SeatKind::Human, SeatKind::Human]);
        let before = version(&app);
        for _ in 0..8 {
            app.update();
        }
        assert_eq!(version(&app), before);
    }
}
