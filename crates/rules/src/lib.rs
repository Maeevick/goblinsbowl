//! Pure, deterministic rules engine of Goblins Bowl.
//!
//! No IO, no clock, no async: callers pass dice, commands and elapsed time in,
//! and receive events and snapshots out, so every match can be replayed exactly.

pub mod arena;
pub mod board;
pub mod characteristics;
pub mod command;
pub mod dice;
pub mod event;
pub mod flight;
pub mod formation;
pub mod game;
pub mod host;
pub mod odds;
pub mod paths;
pub mod preview;
pub mod roster;
pub mod ruleset;
pub mod square;
pub mod state;
pub mod team;
pub mod validation;
pub mod violation;
