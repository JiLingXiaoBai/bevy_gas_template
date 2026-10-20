//! Shared simulation composition and application startup utilities.
//!
//! [`SimulationPlugin`] installs the same simulation settings and GAS runtime for
//! windowed games, headless applications, and tests. This module loads no configuration
//! and contains no sample scenes; optional demonstrations live entirely in `examples/`.

mod common;
mod simulation;

pub use common::{GameResult, default_config_directory};
pub use simulation::{SimulationPlugin, SimulationSettings, SimulationSettingsError};
