//! Reusable game-owned input bindings, spatial sampling, targeting, and positions.
//!
//! Sample application entry points remain re-exported for compatibility; new callers should
//! use the dedicated application module.

pub mod ability_input;
pub mod position;
pub mod randoms;
mod schedule;
pub(crate) mod spatial;
pub mod targeting;

pub use crate::application::{
    GameResult, LaserReport, default_config_directory, run_headless, run_windowed,
};
pub use schedule::{GameplaySchedulePlugin, GameplaySet};
