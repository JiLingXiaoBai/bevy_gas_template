//! Shared simulation composition for interactive applications, headless runtimes, and tests.

mod plugin;
mod settings;

pub use plugin::SimulationPlugin;
pub use settings::{SimulationSettings, SimulationSettingsError};
