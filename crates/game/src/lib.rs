//! Goblins Bowl desktop game: the Bevy plugins behind the window, wired together by the binary.

pub mod badges;
pub mod beats;
pub mod bot_seat;
pub mod bowl_view;
pub mod camera_rig;
pub mod dev_capture;
pub mod dice_view;
pub mod final_whistle;
pub mod goblin_cards;
pub mod goblin_view;
pub mod hud;
pub mod local_match;
pub mod options;
pub mod orders;
pub mod pointer;
pub mod replay;
pub mod replay_bar;
pub mod server_link;
pub mod start_menu;
#[cfg(test)]
mod test_support;
pub mod texts;
