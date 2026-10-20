use super::LaserReport;
use super::scenario::{initialize_scenario, queue_laser, read_report};
use bevy::ecs::message::MessageCursor;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy_gas_template::application::{GameResult, SimulationPlugin};
use bevy_gas_template::gameplay::commands::{
    AbilityCommandOutcome, AbilityCommandPlugin, AbilityCommandResult,
};
use std::path::Path;

/// Executes the game-owned laser configuration without opening a window.
///
/// `directory` contains the Luban binary tables. This sample smoke test drives activation tick
/// zero and twelve subsequent fixed ticks using the shared simulation composition, returning
/// the observed attributes and active ability count. Loading, command rejection, compilation,
/// and unexpected gameplay results return an error. Games build their own headless runtimes
/// using [`SimulationPlugin`] instead of inheriting this sample's fixed duration and assertions.
pub fn run_headless(directory: &Path) -> GameResult<LaserReport> {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        SimulationPlugin::default(),
        AbilityCommandPlugin,
    ));
    initialize_scenario(app.world_mut(), directory)?;
    app.finish();
    app.cleanup();
    let command_id = app
        .world_mut()
        .run_system_once(queue_laser)
        .map_err(|error| format!("Could not queue the laser command: {error:?}"))??;
    let mut cursor = MessageCursor::<AbilityCommandResult>::default();
    let mut accepted = false;
    for _ in 0..=12 {
        app.world_mut().run_schedule(FixedUpdate);
        for result in cursor.read(app.world().resource::<Messages<AbilityCommandResult>>()) {
            if result.command_id == command_id {
                if matches!(result.outcome, AbilityCommandOutcome::Succeeded) {
                    accepted = true;
                } else {
                    return Err(
                        format!("Configured laser command failed: {:?}", result.outcome).into(),
                    );
                }
            }
        }
    }
    if !accepted {
        return Err("Configured laser command did not produce a terminal result".into());
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

/// Reports that the interactive example requires the disabled `windowed` feature.
///
/// `directory` is accepted for the same launch interface but no configuration is loaded.
/// Returns an error explaining how to enable window support or run headlessly.
#[cfg(not(feature = "windowed"))]
pub fn run_windowed(_directory: &Path) -> GameResult<()> {
    Err("The interactive example requires the `windowed` feature. Enable it with `--features windowed`, or pass `--headless` to run without a window.".into())
}
