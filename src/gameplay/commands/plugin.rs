use super::processing::{
    AbilityCommandState, collect_ability_command_results, prepare_ability_commands,
    submit_ability_commands,
};
use super::{AbilityCommand, AbilityCommandResult, GameCommandQueue};
use crate::gameplay::GameplaySet;
use crate::gameplay::targeting::TargetingPlugin;
use bevy::prelude::*;
use bevy_gas::GameplayAbilitySystemSet;

/// Public fixed-tick integration points for the optional ability command adapter.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AbilityCommandSet {
    /// Consumes commands after request producers and before spatial synchronization.
    Prepare,
    /// Submits activations after targeting and before GAS convergence and resolution.
    Submit,
    /// Publishes terminal command results after GAS attribute recalculation.
    Results,
}

/// Adapts buffered ability commands to the existing targeting and GAS queues.
///
/// Install alongside GAS. This installs `TargetingPlugin` when absent; when explicitly
/// installing both, add targeting first. Commands produced before `Prepare` execute and
/// report in the same fixed tick. Later commands remain buffered for the next tick.
/// The generic `GameCommandQueue<C>` does not require this plugin.
pub struct AbilityCommandPlugin;

impl Plugin for AbilityCommandPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<TargetingPlugin>() {
            app.add_plugins(TargetingPlugin);
        }
        app.init_resource::<GameCommandQueue<AbilityCommand>>()
            .init_resource::<AbilityCommandState>()
            .add_message::<AbilityCommandResult>()
            .configure_sets(
                FixedUpdate,
                AbilityCommandSet::Prepare
                    .after(GameplayAbilitySystemSet::RequestProducers)
                    .before(GameplaySet::SpatialSync),
            )
            .configure_sets(
                FixedUpdate,
                AbilityCommandSet::Submit
                    .after(GameplaySet::Targeting)
                    .before(GameplayAbilitySystemSet::PreGameplayConvergence),
            )
            .configure_sets(
                FixedUpdate,
                AbilityCommandSet::Results.after(GameplayAbilitySystemSet::RecalculateAttributes),
            )
            .add_systems(
                FixedUpdate,
                prepare_ability_commands.in_set(AbilityCommandSet::Prepare),
            )
            .add_systems(
                FixedUpdate,
                submit_ability_commands.in_set(AbilityCommandSet::Submit),
            )
            .add_systems(
                FixedUpdate,
                collect_ability_command_results.in_set(AbilityCommandSet::Results),
            );
    }
}
