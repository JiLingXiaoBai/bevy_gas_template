//! Sample application composition, separate from reusable gameplay and configuration.
//!
//! The headless entry runs with the base Bevy dependencies. The default `windowed` feature
//! enables the interactive presentation; calling its entry without that feature returns an
//! actionable error. Applications choose their own composition when reusing `gameplay`.

mod common;
mod headless;
mod scenario;
#[cfg(feature = "windowed")]
mod windowed;

#[cfg(not(feature = "windowed"))]
pub use common::run_windowed;
pub use common::{GameResult, default_config_directory};
pub use headless::run_headless;
pub use scenario::LaserReport;
#[cfg(feature = "windowed")]
pub use windowed::run_windowed;
