use super::{AbilityCommand, AbilityCommandOutcome, AbilityCommandResult, GameCommandQueue};
use bevy::prelude::*;

/// Reports whether the adapter has consumed all prepared and submitted work.
///
/// Call scene-boundary cancellation only after this is true. Queued commands
/// have not entered the adapter and may be cancelled separately without losing IDs.
pub fn ability_commands_are_settled(world: &World) -> bool {
    super::processing::is_settled(world)
}

/// Cancels commands still waiting in the FIFO, publishing one terminal result each.
///
/// Requires a complete tick boundary and a settled adapter. This does not cancel
/// submitted GAS work or active abilities. Identifiers remain reserved.
pub fn cancel_pending_ability_commands(world: &mut World) -> usize {
    let mut cancelled = Vec::new();
    if let Some(mut queue) = world.get_resource_mut::<GameCommandQueue<AbilityCommand>>() {
        while let Some((command_id, command)) = queue.pop() {
            cancelled.push(AbilityCommandResult {
                command_id,
                source: command.source,
                targeting_request_id: None,
                execution_request_id: None,
                activation_chain_id: None,
                outcome: AbilityCommandOutcome::Cancelled,
            });
        }
    }
    let count = cancelled.len();
    if count > 0 {
        world.write_message_batch(cancelled);
    }
    count
}
