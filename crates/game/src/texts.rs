//! Every word the game shows, in French (default) or English.

use clap::ValueEnum;
use goblins_rules::characteristics::Archetype;
use goblins_rules::command::{Action, Activation};
use goblins_rules::event::{GameEvent, TurnEndReason};
use goblins_rules::odds::{ContestDice, Probability};
use goblins_rules::preview::{ActionOdds, ActivationOdds, RiskyStep};
use goblins_rules::roster::GoblinId;
use goblins_rules::team::TeamId;

use crate::beats::ShownRoll;
use crate::final_whistle::SheetEntry;

const FRENCH_QUIPS: [&str; 16] = [
    "Un dokkaebi a perdu son gourdin magique. Il fouille sous les gradins.",
    "Les leprechauns recomptent leur or. Encore. Et encore.",
    "Un nisse réclame sa bouillie au beurre avant de reprendre.",
    "Des korrigans dansent en rond autour de la tanière. Personne n'ose les arrêter.",
    "La balle a été vue au pied d'un arc-en-ciel. Fausse alerte.",
    "L'arbitre gobelin relit le règlement. À l'envers.",
    "Une flamme bleue flotte au-dessus du bowl. Bon signe… ou pas.",
    "Pause boisson : jus de champignon pour tout le monde.",
    "Un gobelin vend des chaussures porte-bonheur en tribune. Que des pieds gauches.",
    "Le public scande « Gar-de ! Gar-de ! » sans trop savoir pourquoi.",
    "Les dokkaebi parient leur gelée de sarrasin sur le prochain tour.",
    "Un leprechaun jure que la balle était en or massif ce matin.",
    "Quelqu'un a noué les lacets du Lanceur ensemble. Enquête en cours.",
    "Le chronomètre s'est fait mordre. Il marche encore, en grognant.",
    "Les korrigans proposent un pas de danse pour se réchauffer.",
    "Un nisse balaie la tanière. C'est la tradition, paraît-il.",
];

const ENGLISH_QUIPS: [&str; 16] = [
    "A dokkaebi lost its magic club. It is searching under the stands.",
    "The leprechauns are counting their gold. Again. And again.",
    "A nisse wants its buttered porridge before play resumes.",
    "Korrigans dance in circles around the den. Nobody dares to stop them.",
    "The ball was spotted at the end of a rainbow. False alarm.",
    "The goblin referee rereads the rulebook. Upside down.",
    "A blue flame floats over the bowl. A good sign… or not.",
    "Drinks break: mushroom juice for everyone.",
    "A goblin sells lucky boots in the stands. Left feet only.",
    "The crowd chants “Stance! Stance!” without quite knowing why.",
    "The dokkaebi bet their buckwheat jelly on the next turn.",
    "A leprechaun swears the ball was solid gold this morning.",
    "Someone tied the Thrower's laces together. Investigation under way.",
    "The clock got bitten. It still works, grumbling.",
    "The korrigans suggest a dance step to warm up.",
    "A nisse sweeps the den. Tradition, apparently.",
];

/// Language of the screens.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum Language {
    /// Français.
    #[default]
    French,
    /// English.
    English,
}

/// Sentences of the game in one language.
#[derive(bevy::prelude::Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Texts {
    /// The language used.
    pub language: Language,
}

impl Texts {
    fn french(&self) -> bool {
        self.language == Language::French
    }

    /// Name of a team.
    pub fn team_name(&self, team: TeamId) -> &'static str {
        match (team == TeamId::HOME, self.french()) {
            (true, true) => "Dokkaebi de la Flamme bleue",
            (true, false) => "Dokkaebi of the Blue Flame",
            (false, _) => "Leprechauns",
        }
    }

    /// Name of an archetype.
    pub fn archetype_name(&self, archetype: Archetype) -> &'static str {
        match (archetype, self.french()) {
            (Archetype::Bruiser, true) => "Cogneur",
            (Archetype::Sprinter, true) => "Sprinteur",
            (Archetype::Thrower, true) => "Lanceur",
            (Archetype::Bruiser, false) => "Bruiser",
            (Archetype::Sprinter, false) => "Sprinter",
            (Archetype::Thrower, false) => "Thrower",
        }
    }

    /// Question asked before a human team's turn starts.
    pub fn ready_question(&self, team: TeamId) -> String {
        if self.french() {
            format!("Au tour des {} ! Prêts ?", self.team_name(team))
        } else {
            format!("{}, your turn! Ready?", self.team_name(team))
        }
    }

    /// A goblin-folklore aside shown while the bowl changes hands, varied from turn to turn.
    pub fn handover_quip(&self, team: TeamId, half: u8, turn: u8) -> &'static str {
        let quips = if self.french() {
            &FRENCH_QUIPS
        } else {
            &ENGLISH_QUIPS
        };
        let index = usize::from(half) * 7 + usize::from(turn) * 3 + team.index() * 5;
        quips[index % quips.len()]
    }

    /// Label of the ready button.
    pub fn ready_button(&self) -> &'static str {
        if self.french() { "Prêts !" } else { "Ready!" }
    }

    /// Label of the end-of-turn button.
    pub fn end_turn_button(&self) -> &'static str {
        if self.french() {
            "Fin du tour"
        } else {
            "End turn"
        }
    }

    /// Label of the end-of-turn button asking for a second click while goblins can still act.
    pub fn end_turn_confirmation(&self) -> &'static str {
        if self.french() { "Vraiment ?" } else { "Sure?" }
    }

    /// Notice shown once no goblin of the team in control can play, until its coach ends the turn.
    pub fn nothing_left_to_play(&self) -> &'static str {
        if self.french() {
            "Plus aucun gobelin ne peut jouer : clique sur « Fin du tour »"
        } else {
            "No goblin can play any more: press “End turn”"
        }
    }

    /// Label of the stance button.
    pub fn stance_button(&self) -> &'static str {
        if self.french() { "Garde" } else { "Stance" }
    }

    /// Label of the cancel button.
    pub fn cancel_button(&self) -> &'static str {
        if self.french() { "Annuler" } else { "Cancel" }
    }

    /// Hint shown once a goblin is selected, for example "H3 (Cogneur) : choisis une case ou une cible".
    pub fn selected_goblin(&self, goblin: GoblinId, archetype: Archetype) -> String {
        let archetype = self.archetype_name(archetype);
        if self.french() {
            format!("{goblin} ({archetype}) : choisis une case ou une cible")
        } else {
            format!("{goblin} ({archetype}): pick a square or a target")
        }
    }

    /// What an aimed order would do and its chance, for example "Assaut sur A3 : 62,5 %".
    pub fn describe_draft(&self, activation: &Activation, odds: &ActivationOdds) -> String {
        let chance = if odds.overall_success == Probability::CERTAIN {
            if self.french() {
                "sans risque"
            } else {
                "no risk"
            }
            .to_string()
        } else {
            self.chance(odds.overall_success)
        };
        match (odds.scores_on_the_way, activation.action, self.french()) {
            (true, _, true) => format!("Marquer ! {chance}"),
            (true, _, false) => format!("Score! {chance}"),
            (false, Some(Action::Assault { target }), true) => {
                format!("Assaut sur {target} : {chance}")
            }
            (false, Some(Action::Assault { target }), false) => {
                format!("Assault {target}: {chance}")
            }
            (false, Some(Action::Throw { receiver }), true) => {
                format!("Passe à {receiver} : {chance}")
            }
            (false, Some(Action::Throw { receiver }), false) => {
                format!("Throw to {receiver}: {chance}")
            }
            (false, Some(Action::Stance), true) => format!("Garde : {chance}"),
            (false, Some(Action::Stance), false) => format!("Stance: {chance}"),
            (false, None, true) => format!("Aller ici : {chance}"),
            (false, None, false) => format!("Move here: {chance}"),
        }
    }

    /// The dice an aimed order rolls, for example "Esquive de A1 : d4 contre d4 · Assaut : d4 contre d4+1".
    pub fn dice_detail(&self, odds: &ActivationOdds) -> String {
        let versus = if self.french() { "contre" } else { "vs" };
        let dodges = odds.risky_steps.iter().filter_map(|step| match step {
            RiskyStep::Dodge { tackler, dice, .. } => Some(if self.french() {
                format!("Esquive de {tackler} : {}", describe_dice(dice, versus))
            } else {
                format!("Dodge past {tackler}: {}", describe_dice(dice, versus))
            }),
            RiskyStep::Pick { .. } => None,
        });
        let assault = match &odds.action {
            Some(ActionOdds::Assault {
                dice: Some(dice), ..
            }) => Some(if self.french() {
                format!("Assaut : {}", describe_dice(dice, versus))
            } else {
                format!("Assault: {}", describe_dice(dice, versus))
            }),
            _ => None,
        };
        dodges.chain(assault).collect::<Vec<_>>().join(" · ")
    }

    /// Exclamation shown over a goblin when its dice are rolled, for example "Esquivé !".
    pub fn roll_headline(&self, roll: &ShownRoll) -> &'static str {
        match (roll, roll.succeeded(), self.french()) {
            (ShownRoll::Dodge(_), true, true) => "Esquivé !",
            (ShownRoll::Dodge(_), true, false) => "Dodged!",
            (ShownRoll::Dodge(_), false, true) => "Plaqué !",
            (ShownRoll::Dodge(_), false, false) => "Tackled!",
            (ShownRoll::Assault(_), true, true) => "Touché !",
            (ShownRoll::Assault(_), true, false) => "Hit!",
            (ShownRoll::Assault(_), false, true) => "Contré !",
            (ShownRoll::Assault(_), false, false) => "Countered!",
            (ShownRoll::Pick(_), true, true) => "Ramassée !",
            (ShownRoll::Pick(_), true, false) => "Picked up!",
            (ShownRoll::Pick(_), false, true) => "Ratée !",
            (ShownRoll::Pick(_), false, false) => "Fumbled!",
            (ShownRoll::Throw(_), true, true) => "Lancée !",
            (ShownRoll::Throw(_), true, false) => "Thrown!",
            (ShownRoll::Throw(_), false, true) => "Passe ratée !",
            (ShownRoll::Throw(_), false, false) => "Bad throw!",
            (ShownRoll::Catch(_), true, true) => "Attrapée !",
            (ShownRoll::Catch(_), true, false) => "Caught!",
            (ShownRoll::Catch(_), false, true) => "Lâchée !",
            (ShownRoll::Catch(_), false, false) => "Dropped!",
            (ShownRoll::Intercept(_), true, true) => "Interceptée !",
            (ShownRoll::Intercept(_), true, false) => "Intercepted!",
            (ShownRoll::Intercept(_), false, true) => "Manquée !",
            (ShownRoll::Intercept(_), false, false) => "Missed!",
        }
    }

    /// The dice behind a roll, for example "4−1 sur d4, il faut 2" or "3 contre 2+1".
    pub fn roll_detail(&self, roll: &ShownRoll) -> String {
        match roll {
            ShownRoll::Dodge(contest) | ShownRoll::Assault(contest) => {
                describe_contest(contest, if self.french() { "contre" } else { "vs" })
            }
            ShownRoll::Pick(ball)
            | ShownRoll::Throw(ball)
            | ShownRoll::Catch(ball)
            | ShownRoll::Intercept(ball) => {
                if self.french() {
                    format!(
                        "{}, il faut {}",
                        describe_agility_roll(ball, "sur"),
                        ball.target
                    )
                } else {
                    format!(
                        "{}, needs {}",
                        describe_agility_roll(ball, "on"),
                        ball.target
                    )
                }
            }
        }
    }

    /// Hint shown under an aimed order.
    pub fn confirm_hint(&self) -> &'static str {
        if self.french() {
            "Clique encore pour confirmer"
        } else {
            "Click again to confirm"
        }
    }

    /// Line describing whose turn it is, for example "Mi-temps 1 · tour 3/10".
    pub fn turn_line(&self, half: u8, turn: u8, turns_per_half: u8) -> String {
        if self.french() {
            format!("Mi-temps {half} · tour {turn}/{turns_per_half}")
        } else {
            format!("Half {half} · turn {turn}/{turns_per_half}")
        }
    }

    /// A chance as a percentage with one decimal: "62,5 %" in French, "62.5 %" in English.
    pub fn chance(&self, probability: Probability) -> String {
        let text = format!("{:.1} %", probability.as_percentage());
        if self.french() {
            text.replace('.', ",")
        } else {
            text
        }
    }

    /// Remaining time as minutes and seconds, rounded up: "0:42".
    pub fn clock(&self, remaining_ms: u64) -> String {
        let seconds = remaining_ms.div_ceil(1_000);
        format!("{}:{:02}", seconds / 60, seconds % 60)
    }

    /// One readable sentence for an event, or None for events not worth a log line.
    pub fn describe_event(&self, event: &GameEvent) -> Option<String> {
        let french = self.french();
        let line = match event {
            GameEvent::CoinTossed { receiving_team } => pick(
                french,
                format!(
                    "Pile ou face : les {} commencent",
                    self.team_name(*receiving_team)
                ),
                format!("Coin toss: the {} start", self.team_name(*receiving_team)),
            ),
            GameEvent::KickedOff { receiving_team } => pick(
                french,
                format!(
                    "Coup d'envoi ! Les {} à l'attaque",
                    self.team_name(*receiving_team)
                ),
                format!("Kick-off! The {} attack", self.team_name(*receiving_team)),
            ),
            GameEvent::ReturnedFromKnockOut { goblin, health } => pick(
                french,
                format!("{goblin} revient avec {health} PV"),
                format!("{goblin} is back with {health} health"),
            ),
            GameEvent::CatchAttempted { goblin, roll } => pick(
                french,
                if roll.success {
                    format!(
                        "{goblin} attrape la balle : {}",
                        describe_agility_roll(roll, "sur")
                    )
                } else {
                    format!(
                        "{goblin} laisse échapper la balle : {}",
                        describe_agility_roll(roll, "sur")
                    )
                },
                if roll.success {
                    format!(
                        "{goblin} catches the ball: {}",
                        describe_agility_roll(roll, "on")
                    )
                } else {
                    format!(
                        "{goblin} drops the ball: {}",
                        describe_agility_roll(roll, "on")
                    )
                },
            ),
            GameEvent::TurnStarted { team, turn, .. } => pick(
                french,
                format!("Tour {turn} des {}", self.team_name(*team)),
                format!("Turn {turn} of the {}", self.team_name(*team)),
            ),
            GameEvent::StoodUp { goblin } => pick(
                french,
                format!("{goblin} se relève"),
                format!("{goblin} gets up"),
            ),
            GameEvent::DodgeAttempted {
                goblin,
                tackler,
                contest,
            } => pick(
                french,
                if contest.actor_won() {
                    format!(
                        "{goblin} esquive {tackler} : {}",
                        describe_contest(contest, "contre")
                    )
                } else {
                    format!(
                        "{goblin} trébuche face à {tackler} : {}",
                        describe_contest(contest, "contre")
                    )
                },
                if contest.actor_won() {
                    format!(
                        "{goblin} slips past {tackler}: {}",
                        describe_contest(contest, "vs")
                    )
                } else {
                    format!(
                        "{goblin} trips on {tackler}: {}",
                        describe_contest(contest, "vs")
                    )
                },
            ),
            GameEvent::PickAttempted { goblin, roll } => pick(
                french,
                if roll.success {
                    format!(
                        "{goblin} ramasse la balle : {}",
                        describe_agility_roll(roll, "sur")
                    )
                } else {
                    format!(
                        "{goblin} rate la balle : {}, il fallait {}",
                        describe_agility_roll(roll, "sur"),
                        roll.target
                    )
                },
                if roll.success {
                    format!(
                        "{goblin} picks up the ball: {}",
                        describe_agility_roll(roll, "on")
                    )
                } else {
                    format!(
                        "{goblin} misses the ball: {}, needed {}",
                        describe_agility_roll(roll, "on"),
                        roll.target
                    )
                },
            ),
            GameEvent::ThrowAttempted {
                thrower,
                receiver,
                roll,
                distance,
            } => pick(
                french,
                if roll.success {
                    format!(
                        "{thrower} lance vers {receiver} à {distance} cases : {}",
                        describe_agility_roll(roll, "sur")
                    )
                } else {
                    format!(
                        "{thrower} rate son lancer vers {receiver} : {}",
                        describe_agility_roll(roll, "sur")
                    )
                },
                if roll.success {
                    format!(
                        "{thrower} throws to {receiver}, {distance} squares away: {}",
                        describe_agility_roll(roll, "on")
                    )
                } else {
                    format!(
                        "{thrower} fumbles the throw to {receiver}: {}",
                        describe_agility_roll(roll, "on")
                    )
                },
            ),
            GameEvent::InterceptAttempted { goblin, roll } => pick(
                french,
                if roll.success {
                    format!(
                        "{goblin} intercepte ! {}",
                        describe_agility_roll(roll, "sur")
                    )
                } else {
                    format!(
                        "{goblin} rate l'interception : {}",
                        describe_agility_roll(roll, "sur")
                    )
                },
                if roll.success {
                    format!("{goblin} intercepts! {}", describe_agility_roll(roll, "on"))
                } else {
                    format!(
                        "{goblin} misses the interception: {}",
                        describe_agility_roll(roll, "on")
                    )
                },
            ),
            GameEvent::AssaultResolved {
                attacker,
                defender,
                contest,
                ..
            } => {
                let winner = if contest.actor_won() {
                    attacker
                } else {
                    defender
                };
                pick(
                    french,
                    format!(
                        "{attacker} attaque {defender} : {}, {winner} gagne",
                        describe_contest(contest, "contre")
                    ),
                    format!(
                        "{attacker} assaults {defender}: {}, {winner} wins",
                        describe_contest(contest, "vs")
                    ),
                )
            }
            GameEvent::StanceTaken { goblin } => pick(
                french,
                format!("{goblin} se met en garde"),
                format!("{goblin} braces in stance"),
            ),
            GameEvent::KnockedDown { goblin, .. } => {
                pick(french, format!("{goblin} tombe"), format!("{goblin} falls"))
            }
            GameEvent::HealthLost {
                goblin,
                amount,
                remaining,
            } => pick(
                french,
                format!("{goblin} perd {amount} PV ({remaining} restants)"),
                format!("{goblin} loses {amount} health ({remaining} left)"),
            ),
            GameEvent::KnockedOut { goblin } => pick(
                french,
                format!("{goblin} est KO !"),
                format!("{goblin} is knocked out!"),
            ),
            GameEvent::Scored { team, scorer } => pick(
                french,
                format!(
                    "Point pour les {} ! Marqué par {scorer}",
                    self.team_name(*team)
                ),
                format!(
                    "Point for the {}! Scored by {scorer}",
                    self.team_name(*team)
                ),
            ),
            GameEvent::TurnEnded { team, reason } => {
                let (french_cause, english_cause) = match reason {
                    TurnEndReason::Knockdown => ("Chute ! ", "Knockdown! "),
                    TurnEndReason::Fumble => ("Fumble ! ", "Fumble! "),
                    TurnEndReason::TimeUp => ("Temps écoulé ! ", "Time is up! "),
                    TurnEndReason::NoGoblinCanAct => {
                        ("Plus personne ne peut jouer. ", "Nobody left to play. ")
                    }
                    TurnEndReason::EndTurn => ("", ""),
                    TurnEndReason::Score => return None,
                };
                pick(
                    french,
                    format!("{french_cause}Fin du tour des {}", self.team_name(*team)),
                    format!("{english_cause}End of the {} turn", self.team_name(*team)),
                )
            }
            GameEvent::HalfEnded { half } => pick(
                french,
                format!("Fin de la mi-temps {half}"),
                format!("End of half {half}"),
            ),
            GameEvent::GameEnded { score } => self.final_whistle(
                score.first().copied().unwrap_or_default(),
                score.get(1).copied().unwrap_or_default(),
            ),
            GameEvent::BallDropped { .. }
            | GameEvent::BallBounced { .. }
            | GameEvent::Moved { .. } => {
                return None;
            }
        };
        Some(line)
    }

    /// A headline for the moments that change a match, shown big once the bowl has played them out:
    /// a turn lost, a point, a knock-out, an interception, half-time.
    pub fn headline(&self, event: &GameEvent) -> Option<String> {
        let french = self.french();
        let headline = match event {
            GameEvent::TurnEnded { team, reason } => {
                let team = self.team_name(*team);
                match reason {
                    TurnEndReason::Knockdown => pick(
                        french,
                        format!("Chute ! Le tour des {team} s'arrête"),
                        format!("Knockdown! The {team} turn is over"),
                    ),
                    TurnEndReason::Fumble => pick(
                        french,
                        format!("Fumble ! Le tour des {team} s'arrête"),
                        format!("Fumble! The {team} turn is over"),
                    ),
                    TurnEndReason::TimeUp => pick(
                        french,
                        format!("Temps écoulé pour les {team} !"),
                        format!("Time is up for the {team}!"),
                    ),
                    TurnEndReason::EndTurn
                    | TurnEndReason::Score
                    | TurnEndReason::NoGoblinCanAct => return None,
                }
            }
            GameEvent::Scored { team, scorer } => pick(
                french,
                format!("POINT ! {scorer} marque pour les {}", self.team_name(*team)),
                format!("POINT! {scorer} scores for the {}", self.team_name(*team)),
            ),
            GameEvent::KnockedOut { goblin } => pick(
                french,
                format!("{goblin} est KO !"),
                format!("{goblin} is knocked out!"),
            ),
            GameEvent::InterceptAttempted { goblin, roll } if roll.success => pick(
                french,
                format!("Interception de {goblin} !"),
                format!("{goblin} intercepts!"),
            ),
            GameEvent::HalfEnded { half } => pick(
                french,
                format!("Fin de la mi-temps {half} !"),
                format!("End of half {half}!"),
            ),
            _ => return None,
        };
        Some(headline)
    }

    /// What the turn line reads once the match is over.
    pub fn full_time(&self) -> &'static str {
        if self.french() {
            "Fin du match"
        } else {
            "Full time"
        }
    }

    /// Title of the end-of-match screen.
    pub fn final_title(&self) -> &'static str {
        if self.french() {
            "Coup de sifflet final !"
        } else {
            "Final whistle!"
        }
    }

    /// The result of the match: who won, and the score.
    pub fn final_result(&self, home_score: u8, away_score: u8) -> String {
        let winner = match home_score.cmp(&away_score) {
            std::cmp::Ordering::Greater => Some(TeamId::HOME),
            std::cmp::Ordering::Less => Some(TeamId::AWAY),
            std::cmp::Ordering::Equal => None,
        };
        match (winner, self.french()) {
            (Some(team), true) => format!(
                "Victoire des {}, {home_score} à {away_score}",
                self.team_name(team)
            ),
            (Some(team), false) => format!(
                "The {} win, {home_score} to {away_score}",
                self.team_name(team)
            ),
            (None, true) => format!("Match nul, {home_score} à {away_score}"),
            (None, false) => format!("Draw, {home_score} to {away_score}"),
        }
    }

    /// What a line of the final scoresheet counts, with the meaning of its two numbers.
    pub fn sheet_label(&self, entry: SheetEntry) -> &'static str {
        match (entry, self.french()) {
            (SheetEntry::Assaults, true) => "Assauts (gagnés / perdus)",
            (SheetEntry::Assaults, false) => "Assaults (won / lost)",
            (SheetEntry::BracedDefences, true) => "Défenses en Garde (tenues / tombées)",
            (SheetEntry::BracedDefences, false) => "Defences in Stance (held / fell)",
            (SheetEntry::OpenDefences, true) => "Défenses sans Garde (tenues / tombées)",
            (SheetEntry::OpenDefences, false) => "Defences without Stance (held / fell)",
            (SheetEntry::Dodges, true) => "Esquives (réussies / ratées)",
            (SheetEntry::Dodges, false) => "Dodges (made / failed)",
            (SheetEntry::Passes, true) => "Passes (réussies / ratées)",
            (SheetEntry::Passes, false) => "Passes (completed / failed)",
            (SheetEntry::Catches, true) => "Réceptions (réussies / ratées)",
            (SheetEntry::Catches, false) => "Catches (held / dropped)",
            (SheetEntry::Pickups, true) => "Ramassages (réussis / ratés)",
            (SheetEntry::Pickups, false) => "Pick-ups (made / failed)",
            (SheetEntry::Interceptions, true) => "Interceptions (réussies / ratées)",
            (SheetEntry::Interceptions, false) => "Interceptions (made / missed)",
            (SheetEntry::StepsWithTheBall, true) => "Cases courues avec la balle",
            (SheetEntry::StepsWithTheBall, false) => "Squares run with the ball",
            (SheetEntry::LongestPass, true) => "Plus longue passe réussie (cases)",
            (SheetEntry::LongestPass, false) => "Longest completed pass (squares)",
            (SheetEntry::PassDistance, true) => "Distance des passes réussies (cases)",
            (SheetEntry::PassDistance, false) => "Completed pass distance (squares)",
            (SheetEntry::KnockOuts, true) => "Gobelins KO",
            (SheetEntry::KnockOuts, false) => "Goblins knocked out",
            (SheetEntry::TurnEndings, true) => "Fins de tour (choix · temps · chute · fumble)",
            (SheetEntry::TurnEndings, false) => {
                "Turn endings (choice · clock · knockdown · fumble)"
            }
        }
    }

    /// Label of the button starting a new match.
    pub fn play_again_button(&self) -> &'static str {
        if self.french() {
            "Rejouer"
        } else {
            "Play again"
        }
    }

    /// Label of the button closing the game.
    pub fn quit_button(&self) -> &'static str {
        if self.french() { "Quitter" } else { "Quit" }
    }

    /// Final line once the match is over.
    pub fn final_whistle(&self, home_score: u8, away_score: u8) -> String {
        if self.french() {
            format!("Coup de sifflet final : {home_score} à {away_score}")
        } else {
            format!("Final whistle: {home_score} to {away_score}")
        }
    }
}

fn pick(french: bool, in_french: String, in_english: String) -> String {
    if french { in_french } else { in_english }
}

fn describe_agility_roll(roll: &goblins_rules::event::AgilityRoll, on: &str) -> String {
    if roll.malus == 0 {
        format!("{} {on} d{}", roll.rolled, roll.faces)
    } else {
        format!("{}−{} {on} d{}", roll.rolled, roll.malus, roll.faces)
    }
}

fn describe_dice(dice: &ContestDice, versus: &str) -> String {
    let modifier = match dice.opponent_modifier {
        0 => String::new(),
        positive if positive > 0 => format!("+{positive}"),
        negative => format!("−{}", negative.unsigned_abs()),
    };
    format!(
        "d{} {versus} d{}{modifier}",
        dice.actor_faces, dice.opponent_faces
    )
}

fn describe_contest(contest: &goblins_rules::event::ContestRoll, versus: &str) -> String {
    let modifier = match contest.opponent_modifier {
        0 => String::new(),
        positive if positive > 0 => format!("+{positive}"),
        negative => format!("−{}", negative.unsigned_abs()),
    };
    format!(
        "{} {versus} {}{modifier}",
        contest.actor_rolled, contest.opponent_rolled
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use goblins_rules::event::{AgilityRoll, ContestRoll};
    use goblins_rules::roster::GoblinId;

    const FRENCH: Texts = Texts {
        language: Language::French,
    };
    const ENGLISH: Texts = Texts {
        language: Language::English,
    };

    fn id(text: &str) -> GoblinId {
        text.parse().unwrap()
    }

    fn odds(overall_success: Probability, scores_on_the_way: bool) -> ActivationOdds {
        ActivationOdds {
            risky_steps: Vec::new(),
            path_success: Probability::CERTAIN,
            scores_on_the_way,
            action: None,
            overall_success,
        }
    }

    fn order(goblin: &str, steps: usize, action: Option<Action>) -> Activation {
        Activation {
            goblin: id(goblin),
            path: (0..steps)
                .map(|step| goblins_rules::square::Square::new(4 + step as u8, 7))
                .collect(),
            action,
        }
    }

    #[test]
    fn given_french_when_a_goblin_is_selected_then_it_is_asked_where_to_go() {
        assert_eq!(
            FRENCH.selected_goblin(id("H3"), Archetype::Bruiser),
            "H3 (Cogneur) : choisis une case ou une cible"
        );
    }

    #[test]
    fn given_french_when_describing_an_aimed_assault_then_target_and_chance() {
        assert_eq!(
            FRENCH.describe_draft(
                &order("H1", 0, Some(Action::Assault { target: id("A3") })),
                &odds(Probability::new(5, 8), false)
            ),
            "Assaut sur A3 : 62,5 %"
        );
    }

    #[test]
    fn given_english_when_describing_an_aimed_throw_then_receiver_and_chance() {
        assert_eq!(
            ENGLISH.describe_draft(
                &order("H9", 0, Some(Action::Throw { receiver: id("H5") })),
                &odds(Probability::new(3, 4), false)
            ),
            "Throw to H5: 75.0 %"
        );
    }

    #[test]
    fn given_french_when_describing_a_walk_without_dice_then_no_risk() {
        assert_eq!(
            FRENCH.describe_draft(&order("H5", 2, None), &odds(Probability::CERTAIN, false)),
            "Aller ici : sans risque"
        );
    }

    #[test]
    fn given_french_when_describing_a_walk_into_the_den_then_a_score() {
        assert_eq!(
            FRENCH.describe_draft(&order("H5", 3, None), &odds(Probability::new(3, 4), true)),
            "Marquer ! 75,0 %"
        );
    }

    fn dice(actor_faces: u8, opponent_faces: u8, opponent_modifier: i8) -> ContestDice {
        ContestDice {
            actor_faces,
            opponent_faces,
            opponent_modifier,
        }
    }

    #[test]
    fn given_french_when_detailing_an_assault_on_a_braced_defender_then_both_dice_and_the_bonus() {
        let mut preview = odds(Probability::new(3, 8), false);
        preview.action = Some(ActionOdds::Assault {
            target: id("A1"),
            dice: Some(dice(4, 4, 1)),
            success: Probability::new(3, 8),
        });
        assert_eq!(FRENCH.dice_detail(&preview), "Assaut : d4 contre d4+1");
    }

    #[test]
    fn given_english_when_detailing_a_dodge_then_an_assault_on_a_downed_goblin_then_both_in_order()
    {
        let mut preview = odds(Probability::new(1, 2), false);
        preview.risky_steps = vec![RiskyStep::Dodge {
            from: goblins_rules::square::Square::new(5, 7),
            tackler: id("A1"),
            dice: dice(4, 4, 0),
            success: Probability::new(5, 8),
        }];
        preview.action = Some(ActionOdds::Assault {
            target: id("A2"),
            dice: Some(dice(3, 2, -2)),
            success: Probability::CERTAIN,
        });
        assert_eq!(
            ENGLISH.dice_detail(&preview),
            "Dodge past A1: d4 vs d4 · Assault: d3 vs d2−2"
        );
    }

    #[test]
    fn given_a_walk_without_dice_when_detailing_it_then_nothing() {
        assert_eq!(FRENCH.dice_detail(&odds(Probability::CERTAIN, false)), "");
    }

    fn contest_roll(actor_rolled: u8, opponent_rolled: u8, opponent_modifier: i8) -> ContestRoll {
        ContestRoll {
            actor_faces: 4,
            actor_rolled,
            opponent_faces: 4,
            opponent_rolled,
            opponent_modifier,
            margin: i8::try_from(actor_rolled).unwrap()
                - i8::try_from(opponent_rolled).unwrap()
                - opponent_modifier,
        }
    }

    #[test]
    fn given_french_when_a_dodge_fails_then_the_goblin_is_tackled() {
        assert_eq!(
            FRENCH.roll_headline(&ShownRoll::Dodge(contest_roll(1, 3, 0))),
            "Plaqué !"
        );
    }

    #[test]
    fn given_english_when_an_assault_wins_then_a_hit() {
        assert_eq!(
            ENGLISH.roll_headline(&ShownRoll::Assault(contest_roll(4, 2, 1))),
            "Hit!"
        );
    }

    #[test]
    fn given_french_when_detailing_a_pick_with_a_malus_then_the_face_the_malus_and_the_target() {
        let roll = AgilityRoll {
            faces: 4,
            rolled: 4,
            malus: 1,
            target: 2,
            success: true,
        };
        assert_eq!(
            FRENCH.roll_detail(&ShownRoll::Pick(roll)),
            "4−1 sur d4, il faut 2"
        );
    }

    #[test]
    fn given_english_when_detailing_a_contest_then_both_faces_and_the_bonus() {
        assert_eq!(
            ENGLISH.roll_detail(&ShownRoll::Assault(contest_roll(3, 2, 1))),
            "3 vs 2+1"
        );
    }

    #[test]
    fn given_english_when_an_order_is_aimed_then_a_second_click_is_asked() {
        assert_eq!(ENGLISH.confirm_hint(), "Click again to confirm");
    }

    #[test]
    fn given_french_when_naming_the_home_team_then_dokkaebi_de_la_flamme_bleue() {
        assert_eq!(
            FRENCH.team_name(TeamId::HOME),
            "Dokkaebi de la Flamme bleue"
        );
    }

    #[test]
    fn given_english_when_naming_the_away_team_then_leprechauns() {
        assert_eq!(ENGLISH.team_name(TeamId::AWAY), "Leprechauns");
    }

    #[test]
    fn given_french_when_naming_a_sprinter_then_sprinteur() {
        assert_eq!(FRENCH.archetype_name(Archetype::Sprinter), "Sprinteur");
    }

    #[test]
    fn given_french_when_asking_if_the_leprechauns_are_ready_then_a_friendly_question() {
        assert_eq!(
            FRENCH.ready_question(TeamId::AWAY),
            "Au tour des Leprechauns ! Prêts ?"
        );
    }

    #[test]
    fn given_french_when_showing_five_eighths_then_a_decimal_comma() {
        assert_eq!(FRENCH.chance(Probability::new(5, 8)), "62,5 %");
    }

    #[test]
    fn given_english_when_showing_five_eighths_then_a_decimal_point() {
        assert_eq!(ENGLISH.chance(Probability::new(5, 8)), "62.5 %");
    }

    #[test]
    fn given_41_point_2_seconds_left_when_shown_then_the_clock_says_0_42() {
        assert_eq!(FRENCH.clock(41_200), "0:42");
    }

    #[test]
    fn given_a_turn_when_described_in_french_then_half_and_turn_out_of_ten() {
        assert_eq!(FRENCH.turn_line(1, 3, 10), "Mi-temps 1 · tour 3/10");
    }

    #[test]
    fn given_a_won_assault_when_described_in_french_then_both_dice_and_the_winner() {
        let event = GameEvent::AssaultResolved {
            attacker: id("H3"),
            defender: id("A8"),
            contest: ContestRoll {
                actor_faces: 4,
                actor_rolled: 4,
                opponent_faces: 2,
                opponent_rolled: 1,
                opponent_modifier: 1,
                margin: 2,
            },
            defender_braced: true,
        };
        assert_eq!(
            FRENCH.describe_event(&event),
            Some("H3 attaque A8 : 4 contre 1+1, H3 gagne".to_string())
        );
    }

    #[test]
    fn given_a_failed_pick_when_described_in_french_then_the_roll_and_the_target() {
        let event = GameEvent::PickAttempted {
            goblin: id("A5"),
            roll: AgilityRoll {
                faces: 4,
                rolled: 2,
                malus: 1,
                target: 2,
                success: false,
            },
        };
        assert_eq!(
            FRENCH.describe_event(&event),
            Some("A5 rate la balle : 2−1 sur d4, il fallait 2".to_string())
        );
    }

    #[test]
    fn given_a_fumble_ending_the_turn_when_described_in_french_then_it_says_fumble() {
        let event = GameEvent::TurnEnded {
            team: TeamId::HOME,
            reason: TurnEndReason::Fumble,
        };
        assert_eq!(
            FRENCH.describe_event(&event),
            Some("Fumble ! Fin du tour des Dokkaebi de la Flamme bleue".to_string())
        );
    }

    #[test]
    fn given_a_turn_ended_by_its_coach_when_described_in_french_then_the_end_of_that_turn() {
        let event = GameEvent::TurnEnded {
            team: TeamId::AWAY,
            reason: TurnEndReason::EndTurn,
        };
        assert_eq!(
            FRENCH.describe_event(&event),
            Some("Fin du tour des Leprechauns".to_string())
        );
    }

    #[test]
    fn given_a_throw_when_described_in_french_then_the_receiver_and_how_far() {
        let event = GameEvent::ThrowAttempted {
            thrower: id("H9"),
            receiver: id("H5"),
            roll: AgilityRoll {
                faces: 4,
                rolled: 3,
                malus: 1,
                target: 2,
                success: true,
            },
            distance: 7,
        };
        assert_eq!(
            FRENCH.describe_event(&event),
            Some("H9 lance vers H5 à 7 cases : 3−1 sur d4".to_string())
        );
    }

    #[test]
    fn given_a_knockdown_ending_the_turn_when_headlined_in_french_then_the_turn_stops() {
        let event = GameEvent::TurnEnded {
            team: TeamId::HOME,
            reason: TurnEndReason::Knockdown,
        };
        assert_eq!(
            FRENCH.headline(&event),
            Some("Chute ! Le tour des Dokkaebi de la Flamme bleue s'arrête".to_string())
        );
    }

    #[test]
    fn given_a_point_when_headlined_in_english_then_the_scorer_and_its_team() {
        let event = GameEvent::Scored {
            team: TeamId::AWAY,
            scorer: id("A5"),
        };
        assert_eq!(
            ENGLISH.headline(&event),
            Some("POINT! A5 scores for the Leprechauns".to_string())
        );
    }

    #[test]
    fn given_a_missed_interception_when_headlined_then_no_headline() {
        let event = GameEvent::InterceptAttempted {
            goblin: id("A2"),
            roll: AgilityRoll {
                faces: 4,
                rolled: 1,
                malus: 1,
                target: 2,
                success: false,
            },
        };
        assert_eq!(FRENCH.headline(&event), None);
    }

    #[test]
    fn given_a_turn_ended_by_its_coach_when_headlined_then_no_headline() {
        let event = GameEvent::TurnEnded {
            team: TeamId::HOME,
            reason: TurnEndReason::EndTurn,
        };
        assert_eq!(FRENCH.headline(&event), None);
    }

    #[test]
    fn given_an_away_win_when_giving_the_result_in_french_then_the_winner_and_the_score() {
        assert_eq!(FRENCH.final_result(1, 2), "Victoire des Leprechauns, 1 à 2");
    }

    #[test]
    fn given_a_level_score_when_giving_the_result_in_english_then_a_draw() {
        assert_eq!(ENGLISH.final_result(0, 0), "Draw, 0 to 0");
    }

    #[test]
    fn given_french_when_labelling_braced_defences_then_held_and_fell() {
        assert_eq!(
            FRENCH.sheet_label(SheetEntry::BracedDefences),
            "Défenses en Garde (tenues / tombées)"
        );
    }

    #[test]
    fn given_a_goblin_moving_when_described_then_nothing_worth_logging() {
        let event = GameEvent::Moved {
            goblin: id("H1"),
            from: [1, 1].into(),
            to: [1, 2].into(),
        };
        assert_eq!(FRENCH.describe_event(&event), None);
    }

    #[test]
    fn given_a_two_one_final_score_when_described_in_french_then_the_score() {
        assert_eq!(FRENCH.final_whistle(2, 1), "Coup de sifflet final : 2 à 1");
    }

    #[test]
    fn given_english_when_describing_a_catch_then_the_roll_reads_on_the_die() {
        let event = GameEvent::CatchAttempted {
            goblin: id("H2"),
            roll: AgilityRoll {
                faces: 4,
                rolled: 3,
                malus: 0,
                target: 2,
                success: true,
            },
        };
        assert_eq!(
            ENGLISH.describe_event(&event),
            Some("H2 catches the ball: 3 on d4".to_string())
        );
    }

    #[test]
    fn given_french_when_no_goblin_can_play_then_the_coach_is_told_to_end_the_turn() {
        assert_eq!(
            FRENCH.nothing_left_to_play(),
            "Plus aucun gobelin ne peut jouer : clique sur « Fin du tour »"
        );
    }

    #[test]
    fn given_two_consecutive_turns_when_picking_quips_then_they_differ() {
        assert_ne!(
            FRENCH.handover_quip(TeamId::AWAY, 1, 3),
            FRENCH.handover_quip(TeamId::AWAY, 1, 4)
        );
    }

    #[test]
    fn given_both_teams_on_the_same_turn_when_picking_quips_then_they_differ() {
        assert_ne!(
            ENGLISH.handover_quip(TeamId::HOME, 2, 5),
            ENGLISH.handover_quip(TeamId::AWAY, 2, 5)
        );
    }

    #[test]
    fn given_the_same_turn_in_both_languages_when_picking_quips_then_the_same_joke() {
        let quip_number = |quip: &str, quips: &[&str]| quips.iter().position(|line| *line == quip);
        assert_eq!(
            quip_number(FRENCH.handover_quip(TeamId::HOME, 1, 1), &FRENCH_QUIPS),
            quip_number(ENGLISH.handover_quip(TeamId::HOME, 1, 1), &ENGLISH_QUIPS)
        );
    }

    #[test]
    fn given_french_when_asking_to_confirm_the_end_of_turn_then_sur() {
        assert_eq!(FRENCH.end_turn_confirmation(), "Vraiment ?");
    }
}
