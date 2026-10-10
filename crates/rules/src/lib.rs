//! Pure, deterministic rules engine of Goblins Bowl.
//!
//! No IO, no clock, no async: callers pass dice, commands and elapsed time in,
//! and receive events and snapshots out, so every match can be replayed exactly.

pub mod arena;
pub mod characteristics;
pub mod command;
pub mod dice;
pub mod event;
pub mod formation;
pub mod game;
pub mod roster;
pub mod ruleset;
pub mod square;
pub mod state;
pub mod team;
pub mod violation;
