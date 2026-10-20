//! Configured laser demonstration, kept separate from the reusable application library.

mod headless;
mod scenario;
#[cfg(feature = "windowed")]
mod windowed;

pub use headless::run_headless;
#[cfg(not(feature = "windowed"))]
pub use headless::run_windowed;
pub use scenario::LaserReport;
#[cfg(feature = "windowed")]
pub use windowed::run_windowed;

#[cfg(test)]
mod smoke_test;
