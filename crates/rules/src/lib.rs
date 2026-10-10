//! Pure, deterministic rules engine of Goblins Bowl.
//!
//! No IO, no clock, no async: callers pass dice, commands and elapsed time in,
//! and receive events and snapshots out, so every match can be replayed exactly.

pub mod arena;
pub mod square;
pub mod team;
