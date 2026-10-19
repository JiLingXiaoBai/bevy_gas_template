//! Validated game-owned targeting pipelines that submit target data to GAS.
//!
//! This module performs point-based target acquisition through a dynamic AABB tree.
//! Target selection is independent of attack contact detection; no physical collision
//! or collider-volume intersection is inferred from its results.

mod acquisition;
mod error;
mod plugin;
mod spatial_index;
mod targeting_definition;
mod targeting_queue;

pub use acquisition::{TargetingCandidateQuery, TargetingSystemParams, acquire_targets};
pub use error::{TargetingDefinitionError, TargetingError};
pub use plugin::TargetingPlugin;
pub use spatial_index::{TargetingSpatialIndex, sync_targeting_spatial_index_system};
pub use targeting_definition::{
    Targetable, TargetingDefinition, TargetingOperation, TargetingSortOrder,
};
pub use targeting_queue::{
    TargetingContinuation, TargetingInput, TargetingRequestId, TargetingRequestQueue,
    TargetingResultEvent, process_targeting_request_queue_system, targeting_request_queue_has_work,
};
