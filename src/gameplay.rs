//! The playable fireball sample and its deterministic, windowless verification.

mod application;
mod scenario;

pub use application::{default_config_directory, run_headless, run_windowed};
pub use scenario::{FireballReport, GameResult};
