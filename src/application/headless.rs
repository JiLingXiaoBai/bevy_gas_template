use super::scenario::{CastInput, initialize_scenario, queue_laser, read_report};
use super::{GameResult, LaserReport};
use crate::gameplay::targeting::TargetingPlugin;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy_gas::{GameplayAbilitySystemPlugin, GameplayAbilitySystemSet};
use std::path::Path;

/// Executes the game-owned laser configuration without opening a window.
///
/// `directory` contains the Luban binary tables. This drives activation tick zero
/// and twelve subsequent fixed ticks, returning the observed attributes and active
/// ability count. Loading, compilation, and unexpected gameplay results return an error.
pub fn run_headless(directory: &Path) -> GameResult<LaserReport> {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, GameplayAbilitySystemPlugin, TargetingPlugin))
        .insert_resource(CastInput(true))
        .add_systems(
            FixedUpdate,
            queue_laser.in_set(GameplayAbilitySystemSet::RequestProducers),
        );
    initialize_scenario(app.world_mut(), directory)?;
    app.finish();
    app.cleanup();
    for _ in 0..=12 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    let report = app
        .world_mut()
        .run_system_once(read_report)
        .map_err(|error| format!("Could not read the laser result: {error:?}"))??;
    if report.target_health != 320.0 || report.caster_mana != 80.0 || report.active_abilities != 0 {
        return Err(format!("Configured laser smoke test failed: {report:?}; expected target Health=320, caster Mana=80, active abilities=0").into());
    }
    Ok(report)
}
