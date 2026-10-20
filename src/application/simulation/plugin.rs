use super::SimulationSettings;
use crate::gameplay::GameplaySchedulePlugin;
use bevy::prelude::*;
use bevy_gas::{AdditionalCostProvider, GameplayAbilitySystemPlugin, Random};
use std::marker::PhantomData;

/// Installs the shared GAS simulation, game phases, fixed timestep, and initial random seed.
///
/// Add this once after the application's clock plugins and before optional game adapters such
/// as targeting or ability commands. It installs GAS itself; do not also install a GAS plugin
/// group. This plugin creates no scene, loads no configuration, and requires no renderer.
/// Games install their own command, targeting, movement, and presentation plugins separately.
#[derive(Default)]
pub struct SimulationPlugin {
    settings: SimulationSettings,
}

impl SimulationPlugin {
    /// Creates a simulation plugin using already validated startup `settings`.
    pub fn new(settings: SimulationSettings) -> Self {
        Self { settings }
    }

    /// Selects the game's external-cost provider while preserving the simulation settings.
    ///
    /// Returns a plugin to install instead of this one. Initialize the provider's resources
    /// before running gameplay and use matching typed parameters for direct GAS calls.
    pub fn with_additional_costs<P: AdditionalCostProvider>(self) -> impl Plugin {
        ConfiguredSimulationPlugin::<P> {
            settings: self.settings,
            marker: PhantomData,
        }
    }
}

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(GameplayAbilitySystemPlugin);
        install_settings(app, self.settings);
    }
}

struct ConfiguredSimulationPlugin<P> {
    settings: SimulationSettings,
    marker: PhantomData<fn() -> P>,
}

impl<P: AdditionalCostProvider> Plugin for ConfiguredSimulationPlugin<P> {
    fn build(&self, app: &mut App) {
        app.add_plugins(GameplayAbilitySystemPlugin::with_additional_costs::<P>());
        install_settings(app, self.settings);
    }
}

fn install_settings(app: &mut App, settings: SimulationSettings) {
    if !app.is_plugin_added::<GameplaySchedulePlugin>() {
        app.add_plugins(GameplaySchedulePlugin);
    }
    app.insert_resource(settings)
        .insert_resource(Time::<Fixed>::from_duration(settings.timestep()))
        .insert_resource(Random::from_seed(settings.random_seed()));
}
