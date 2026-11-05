//! Buffered game commands and an optional adapter to targeting and GAS execution.
//!
//! The generic queue owns only FIFO ordering and queue-local identifiers. Games can use it
//! for movement or construction without installing targeting or GAS. The ability adapter
//! prepares targeting, submits activations in command order, and reports one terminal result
//! per consumed command in the same fixed tick.

mod ability;
mod cancellation;
mod plugin;
mod processing;
mod queue;

pub use ability::{
    AbilityCommand, AbilityCommandOrigin, AbilityCommandOutcome, AbilityCommandResult,
    AbilityCommandSubmissionError,
};
pub use cancellation::{ability_commands_are_settled, cancel_pending_ability_commands};
pub use plugin::{AbilityCommandPlugin, AbilityCommandSet};
pub use queue::{GameCommandId, GameCommandQueue, GameCommandQueueError};
