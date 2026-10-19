use super::GameplaySet;
use bevy::prelude::*;
use bevy_gas::GameplayAbilitySystemSet;

/// Orders game-owned fixed-tick phases relative to GAS without installing gameplay systems.
///
/// Install alongside GAS when movement or other game systems use [`GameplaySet`].
/// This plugin adds no resources, targeting systems, or presentation systems. The targeting
/// plugin installs it automatically when absent; when explicitly installing both plugins,
/// add this plugin first to satisfy Bevy's unique-plugin requirement.
pub struct GameplaySchedulePlugin;

impl Plugin for GameplaySchedulePlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            FixedUpdate,
            GameplaySet::Movement
                .after(GameplayAbilitySystemSet::EffectTicks)
                .before(GameplayAbilitySystemSet::AbilityTasks),
        )
        .configure_sets(
            FixedUpdate,
            (GameplaySet::SpatialSync, GameplaySet::Targeting)
                .chain()
                .after(GameplayAbilitySystemSet::RequestProducers)
                .before(GameplayAbilitySystemSet::PreGameplayConvergence),
        );
    }
}
