use bevy::prelude::*;

/// Game-owned fixed-tick phases surrounding GAS request production and resolution.
///
/// Install [`super::GameplaySchedulePlugin`] to order these phases relative to GAS.
/// The targeting plugin installs that schedule plugin automatically.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameplaySet {
    /// Updates authoritative positions before ability tasks and captured request inputs.
    Movement,
    /// Synchronizes spatial proxies after game request producers have run.
    SpatialSync,
    /// Acquires targets and submits resulting requests before GAS convergence and resolution.
    Targeting,
}
