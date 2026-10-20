use super::GameCommandId;
use crate::gameplay::targeting::{
    TargetingDefinition, TargetingError, TargetingInput, TargetingRequestId,
};
use bevy::prelude::{Entity, Message};
use bevy_gas::{
    AbilityActivationTargets, AbilityActivationTargetsError, AbilitySpecHandle,
    GameplayExecutionError, GameplayExecutionQueueError, GameplayExecutionRequestId,
};
use std::error::Error;
use std::fmt;
use std::sync::Arc;

/// Root activation reason assigned by the ability command adapter.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum AbilityCommandOrigin {
    /// A player or other game input requested the activation.
    #[default]
    Input,
    /// Game logic directly requested the activation.
    Direct,
}

pub(super) enum CommandTargets {
    Direct(AbilityActivationTargets),
    Targeted {
        input: TargetingInput,
        definition: Arc<TargetingDefinition>,
    },
}

/// One buffered ability intent, independent of physical input and GAS queue bookkeeping.
///
/// Handles and explicit entities are captured at construction. Targeting input also captures
/// its origin and direction; targets are acquired from the fixed tick's synchronized world.
/// The adapter creates the root chain when submitting the command. This represents root
/// activation only; chained activations and source snapshots use the lower-level GAS APIs.
pub struct AbilityCommand {
    pub(super) source: Entity,
    pub(super) handle: AbilitySpecHandle,
    pub(super) targets: CommandTargets,
    pub(super) origin: AbilityCommandOrigin,
}

impl AbilityCommand {
    /// Creates a command from `source` using `handle` and already selected `targets`.
    ///
    /// The returned command defaults to input activation. No targeting rules are applied;
    /// GAS checks execution requirements when the adapter submits the activation.
    pub fn direct(
        source: Entity,
        handle: AbilitySpecHandle,
        targets: impl Into<AbilityActivationTargets>,
    ) -> Self {
        Self {
            source,
            handle,
            targets: CommandTargets::Direct(targets.into()),
            origin: AbilityCommandOrigin::Input,
        }
    }

    /// Creates an input command that acquires targets using `input` and `definition`.
    ///
    /// Returns a buffered intent; acquisition failures are reported as command results
    /// when the fixed-tick adapter consumes it. Build origins from logical positions.
    pub fn targeted(
        source: Entity,
        handle: AbilitySpecHandle,
        input: TargetingInput,
        definition: Arc<TargetingDefinition>,
    ) -> Self {
        Self {
            source,
            handle,
            targets: CommandTargets::Targeted { input, definition },
            origin: AbilityCommandOrigin::Input,
        }
    }

    /// Returns this command with the specified root activation reason.
    pub fn with_origin(mut self, origin: AbilityCommandOrigin) -> Self {
        self.origin = origin;
        self
    }
}

/// Failure between consuming a command and receiving its GAS execution outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbilityCommandSubmissionError {
    /// Target acquisition unexpectedly returned an empty target selection.
    InvalidTargets(AbilityActivationTargetsError),
    /// GAS could not accept the activation request.
    ExecutionQueue(GameplayExecutionQueueError),
    /// Targeting work was removed or its processor did not run in the expected phase.
    MissingTargetingResult,
    /// Accepted GAS work was removed or its resolver did not run in the expected phase.
    MissingExecutionResult,
}

impl fmt::Display for AbilityCommandSubmissionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTargets(error) => error.fmt(formatter),
            Self::ExecutionQueue(error) => error.fmt(formatter),
            Self::MissingTargetingResult => {
                formatter.write_str("targeting did not report the command in this fixed tick")
            }
            Self::MissingExecutionResult => {
                formatter.write_str("GAS did not report the command in this fixed tick")
            }
        }
    }
}

impl Error for AbilityCommandSubmissionError {}

/// Terminal outcome of one consumed ability command.
#[derive(Debug, Clone, PartialEq)]
pub enum AbilityCommandOutcome {
    /// GAS accepted the primary activation; this does not mean the ability has finished.
    Succeeded,
    /// Spatial acquisition failed before an activation was submitted.
    TargetingFailed(TargetingError),
    /// The command could not reach a reported GAS execution outcome.
    SubmissionFailed(AbilityCommandSubmissionError),
    /// GAS rejected the activation under a gameplay rule.
    Rejected(GameplayExecutionError),
    /// Invalid configuration or runtime state prevented GAS execution.
    Failed(GameplayExecutionError),
}

/// Buffered terminal result of one command consumed by [`super::AbilityCommandPlugin`].
///
/// Results are published once in command intake order during `AbilityCommandSet::Results`.
/// Read every frame or fixed tick using a message reader. As with other Bevy messages,
/// consumers that remain inactive beyond message retention can miss old results.
#[derive(Message, Debug, Clone, PartialEq)]
pub struct AbilityCommandResult {
    /// Identifier returned by the ability command queue.
    pub command_id: GameCommandId,
    /// Actor on whose ASC the ability was requested.
    pub source: Entity,
    /// Targeting request identifier, absent for already selected targets.
    pub targeting_request_id: Option<TargetingRequestId>,
    /// Accepted GAS request identifier, absent when acquisition or submission failed.
    pub execution_request_id: Option<GameplayExecutionRequestId>,
    /// Final result of this root activation request.
    pub outcome: AbilityCommandOutcome,
}
