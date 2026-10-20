use bevy::prelude::Resource;
use std::collections::VecDeque;
use std::error::Error;
use std::fmt;

/// Identifier of one command accepted by a particular queue instance.
///
/// Identifiers increase in FIFO order and are not reused after `clear`. They are local to
/// a queue resource, not persistent actor identities or network identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GameCommandId(u64);

impl GameCommandId {
    /// Returns the queue-local numeric identifier.
    pub const fn get_value(self) -> u64 {
        self.0
    }
}

/// Reason a game command could not enter its queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameCommandQueueError {
    /// The queue exhausted its monotonically increasing identifiers.
    IdExhausted,
}

impl fmt::Display for GameCommandQueueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IdExhausted => formatter.write_str("game command identifiers are exhausted"),
        }
    }
}

impl Error for GameCommandQueueError {}

/// Persistent FIFO of game-owned command values.
///
/// Producers may run on frames without fixed ticks. Entries remain until a consumer calls
/// `pop` or `clear`; the queue never executes commands itself. Order producers explicitly
/// when their relative order affects gameplay. Different command types have independent queues.
#[derive(Resource)]
pub struct GameCommandQueue<C: Send + Sync + 'static> {
    commands: VecDeque<(GameCommandId, C)>,
    next_id: u64,
}

impl<C: Send + Sync + 'static> Default for GameCommandQueue<C> {
    fn default() -> Self {
        Self {
            commands: VecDeque::new(),
            next_id: 1,
        }
    }
}

impl<C: Send + Sync + 'static> GameCommandQueue<C> {
    /// Appends `command` and returns its identifier.
    ///
    /// Returns `IdExhausted` without changing the queue when no further ID is available.
    pub fn push(&mut self, command: C) -> Result<GameCommandId, GameCommandQueueError> {
        let next_id = self
            .next_id
            .checked_add(1)
            .ok_or(GameCommandQueueError::IdExhausted)?;
        let id = GameCommandId(self.next_id);
        self.commands.push_back((id, command));
        self.next_id = next_id;
        Ok(id)
    }

    /// Removes and returns the oldest command with its ID, or `None` if empty.
    ///
    /// When the ability plugin owns this queue, popping manually transfers responsibility
    /// for executing that command and reporting its outcome to the caller.
    pub fn pop(&mut self) -> Option<(GameCommandId, C)> {
        self.commands.pop_front()
    }

    /// Returns the number of buffered commands.
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Returns whether there are no buffered commands.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Discards commands still in this queue without executing them or publishing results.
    ///
    /// IDs remain reserved. Commands already consumed by a plugin are unaffected.
    pub fn clear(&mut self) {
        self.commands.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::{GameCommandQueue, GameCommandQueueError};

    #[test]
    fn exhausted_ids_do_not_change_pending_commands() {
        let mut queue = GameCommandQueue::default();
        let first = queue.push(10).unwrap();
        queue.next_id = u64::MAX;
        assert_eq!(queue.push(20), Err(GameCommandQueueError::IdExhausted));
        assert_eq!(queue.pop(), Some((first, 10)));
        assert!(queue.is_empty());
    }
}
