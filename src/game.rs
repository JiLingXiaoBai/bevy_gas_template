//! The new project's gameplay composition entry point.
//!
//! Register game-specific plugins and systems in [`GamePlugin`]. Shared simulation,
//! configuration, and gameplay adapters remain available in their respective modules.

mod plugin;

pub use plugin::GamePlugin;
