use super::super::{TargetingDefinition, TargetingError};
use bevy::prelude::{Entity, Event, Message, Vec3};
use bevy_gas::{
    AbilityActivationContext, AbilityActivationTargetsError, AbilitySpecHandle, AbilityTargetData,
    GameplayExecutionQueueError, GameplayExecutionRequestId,
};
use std::error::Error;
use std::fmt;
use std::sync::Arc;

/// Stable identifier assigned to one queued targeting request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TargetingRequestId(pub(super) u64);

impl TargetingRequestId {
    /// Returns the numeric request identifier.
    pub fn get_value(self) -> u64 {
        self.0
    }
}

/// Spatial input captured when a targeting request is submitted.
#[derive(Debug, Clone, Copy)]
pub struct TargetingInput {
    origin: Vec3,
    direction: Vec3,
    explicit_target: Option<Entity>,
}

impl TargetingInput {
    /// Creates targeting input from a world-space origin and direction.
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self {
            origin,
            direction,
            explicit_target: None,
        }
    }

    /// Sets the entity consumed by `SelectExplicitEntity`.
    pub fn with_explicit_target(mut self, target: Entity) -> Self {
        self.explicit_target = Some(target);
        self
    }

    /// Returns the captured world-space origin.
    pub fn get_origin(self) -> Vec3 {
        self.origin
    }

    /// Returns the captured world-space direction.
    pub fn get_direction(self) -> Vec3 {
        self.direction
    }

    /// Returns the optional explicitly selected entity.
    pub fn get_explicit_target(self) -> Option<Entity> {
        self.explicit_target
    }
}

/// Describes what should happen after a targeting request succeeds.
#[derive(Clone)]
pub enum TargetingContinuation {
    /// Only emits a [`TargetingResultEvent`].
    EmitResult,
    /// Adds an ability activation carrying the acquired target data to the queue.
    ActivateAbility {
        handle: AbilitySpecHandle,
        context: Box<AbilityActivationContext>,
    },
}

impl TargetingContinuation {
    /// Creates an ability-activation continuation.
    pub fn activate_ability(handle: AbilitySpecHandle, context: AbilityActivationContext) -> Self {
        Self::ActivateAbility {
            handle,
            context: Box::new(context),
        }
    }
}

pub(super) struct TargetingRequest {
    pub(super) id: TargetingRequestId,
    pub(super) source: Entity,
    pub(super) input: TargetingInput,
    pub(super) definition: Arc<TargetingDefinition>,
    pub(super) continuation: TargetingContinuation,
}

/// Triggered after a queued targeting request succeeds or fails.
#[derive(Event, Clone)]
pub struct TargetingResultEvent {
    request_id: TargetingRequestId,
    source: Entity,
    result: Result<AbilityTargetData, TargetingError>,
}

/// Failure to submit the activation continuation of a successful targeting request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetingContinuationError {
    /// Acquired target data could not form an activation target selection.
    InvalidTargets(AbilityActivationTargetsError),
    /// The gameplay execution queue rejected the submission.
    ExecutionQueue(GameplayExecutionQueueError),
}

impl fmt::Display for TargetingContinuationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTargets(error) => error.fmt(formatter),
            Self::ExecutionQueue(error) => error.fmt(formatter),
        }
    }
}

impl Error for TargetingContinuationError {}

/// Buffered targeting result available immediately after the targeting phase.
///
/// The existing `TargetingResultEvent` is still triggered for observers. This message lets
/// ordered systems consume the result before GAS resolution without relying on observers.
#[derive(Message, Debug, Clone)]
pub struct TargetingRequestResult {
    /// Identifier assigned by the targeting queue.
    pub request_id: TargetingRequestId,
    /// Entity that submitted the targeting request.
    pub source: Entity,
    /// Acquired targets or the concrete targeting failure.
    pub result: Result<AbilityTargetData, TargetingError>,
    /// Continuation submission outcome, absent for `EmitResult` or failed acquisition.
    ///
    /// Success identifies the GAS request, whose execution result arrives separately.
    pub continuation: Option<Result<GameplayExecutionRequestId, TargetingContinuationError>>,
}

impl TargetingResultEvent {
    pub(super) fn new(
        request_id: TargetingRequestId,
        source: Entity,
        result: Result<AbilityTargetData, TargetingError>,
    ) -> Self {
        Self {
            request_id,
            source,
            result,
        }
    }

    /// Returns the completed request identifier.
    pub fn get_request_id(&self) -> TargetingRequestId {
        self.request_id
    }

    /// Returns the entity that submitted the targeting request.
    pub fn get_source(&self) -> Entity {
        self.source
    }

    /// Returns the acquired data or targeting failure.
    pub fn get_result(&self) -> &Result<AbilityTargetData, TargetingError> {
        &self.result
    }
}
