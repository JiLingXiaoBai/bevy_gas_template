//! Independently installable ordering for game-owned fixed-tick phases.

mod phases;
mod plugin;

pub use phases::GameplaySet;
pub use plugin::GameplaySchedulePlugin;
