use super::super::{TargetingSystemParams, acquire_targets};
use super::{
    TargetingContinuation, TargetingContinuationError, TargetingRequestQueue,
    TargetingRequestResult, TargetingResultEvent,
};
use bevy::prelude::*;
use bevy_gas::{AbilityActivationTargets, GameplayExecutionQueue};

/// Processes queued targeting requests and dispatches their continuations.
pub fn process_targeting_request_queue_system(
    mut commands: Commands,
    mut targeting_queue: ResMut<TargetingRequestQueue>,
    mut execution_queue: ResMut<GameplayExecutionQueue>,
    mut results: MessageWriter<TargetingRequestResult>,
    params: TargetingSystemParams,
) {
    while let Some(request) = targeting_queue.pop() {
        let result = acquire_targets(request.source, request.input, &request.definition, &params);

        let continuation = if let Ok(target_data) = &result
            && let TargetingContinuation::ActivateAbility { handle, context } = request.continuation
        {
            let submission = AbilityActivationTargets::acquired(target_data.clone())
                .map_err(TargetingContinuationError::InvalidTargets)
                .and_then(|targets| {
                    execution_queue
                        .push_activation(request.source, targets, handle, *context)
                        .map_err(TargetingContinuationError::ExecutionQueue)
                });
            if let Err(error) = &submission {
                error!("failed to queue targeting continuation: {error}");
            }
            Some(submission)
        } else {
            None
        };

        commands.trigger(TargetingResultEvent::new(
            request.id,
            request.source,
            result.clone(),
        ));
        results.write(TargetingRequestResult {
            request_id: request.id,
            source: request.source,
            result,
            continuation,
        });
    }
}

/// Returns whether the targeting queue contains work.
pub fn targeting_request_queue_has_work(queue: Option<Res<TargetingRequestQueue>>) -> bool {
    queue.is_some_and(|queue| !queue.is_empty())
}
