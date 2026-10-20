use super::spatial_index::{remove_discarded_target_position, remove_discarded_targetable};
use super::{
    TargetingRequestQueue, TargetingRequestResult, TargetingSpatialIndex,
    process_targeting_request_queue_system, sync_targeting_spatial_index_system,
    targeting_request_queue_has_work,
};
use crate::gameplay::{GameplaySchedulePlugin, GameplaySet};
use bevy::prelude::*;

/// Installs game-owned targeting resources, lifecycle observers, and fixed-tick systems.
///
/// Installs [`GameplaySchedulePlugin`] when absent. Install alongside GAS's runtime plugin
/// or plugin group. Logical movement belongs in [`GameplaySet::Movement`]; requests
/// submitted by GAS request producers are acquired after spatial synchronization and their
/// continuations reach GAS resolution in the same tick. Presentation is optional and
/// installed separately with the position presentation plugin.
pub struct TargetingPlugin;

impl Plugin for TargetingPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<GameplaySchedulePlugin>() {
            app.add_plugins(GameplaySchedulePlugin);
        }
        app.init_resource::<TargetingRequestQueue>()
            .add_message::<TargetingRequestResult>()
            .init_resource::<TargetingSpatialIndex>()
            .add_observer(remove_discarded_targetable)
            .add_observer(remove_discarded_target_position)
            .add_systems(
                FixedUpdate,
                sync_targeting_spatial_index_system.in_set(GameplaySet::SpatialSync),
            )
            .add_systems(
                FixedUpdate,
                process_targeting_request_queue_system
                    .run_if(targeting_request_queue_has_work)
                    .in_set(GameplaySet::Targeting),
            );
    }
}
