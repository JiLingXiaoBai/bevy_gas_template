use super::ability::CommandTargets;
use super::{
    AbilityCommand, AbilityCommandOrigin, AbilityCommandOutcome, AbilityCommandResult,
    AbilityCommandSubmissionError, GameCommandId, GameCommandQueue,
};
use crate::gameplay::targeting::{
    TargetingContinuation, TargetingRequestId, TargetingRequestQueue, TargetingRequestResult,
};
use bevy::prelude::*;
use bevy_gas::{
    AbilityActivationContext, AbilityActivationTargets, AbilitySpecHandle,
    GameplayExecutionOutcome, GameplayExecutionQueue, GameplayExecutionResult,
};
use std::collections::BTreeMap;

enum PreparedTargets {
    Direct(AbilityActivationTargets),
    Targeted(TargetingRequestId),
}

struct PreparedCommand {
    id: GameCommandId,
    source: Entity,
    handle: AbilitySpecHandle,
    origin: AbilityCommandOrigin,
    targets: PreparedTargets,
}

#[derive(Resource, Default)]
pub(super) struct AbilityCommandState {
    prepared: Vec<PreparedCommand>,
    completions: Vec<AbilityCommandResult>,
}

pub(super) fn prepare_ability_commands(
    mut commands: ResMut<GameCommandQueue<AbilityCommand>>,
    mut targeting: ResMut<TargetingRequestQueue>,
    mut state: ResMut<AbilityCommandState>,
) {
    while let Some((id, command)) = commands.pop() {
        let targets = match command.targets {
            CommandTargets::Direct(targets) => PreparedTargets::Direct(targets),
            CommandTargets::Targeted { input, definition } => {
                PreparedTargets::Targeted(targeting.push_request(
                    command.source,
                    input,
                    definition,
                    TargetingContinuation::EmitResult,
                ))
            }
        };
        state.prepared.push(PreparedCommand {
            id,
            source: command.source,
            handle: command.handle,
            origin: command.origin,
            targets,
        });
    }
}

pub(super) fn submit_ability_commands(
    mut targeting_results: MessageReader<TargetingRequestResult>,
    mut execution: ResMut<GameplayExecutionQueue>,
    mut state: ResMut<AbilityCommandState>,
) {
    if state.prepared.is_empty() {
        targeting_results.clear();
        return;
    }
    let mut acquired = targeting_results
        .read()
        .map(|result| (result.request_id.get_value(), &result.result))
        .collect::<BTreeMap<_, _>>();
    let state = &mut *state;
    for command in state.prepared.drain(..) {
        let targeting_request_id = match &command.targets {
            PreparedTargets::Direct(_) => None,
            PreparedTargets::Targeted(id) => Some(*id),
        };
        let targets = match command.targets {
            PreparedTargets::Direct(targets) => Ok(targets),
            PreparedTargets::Targeted(id) => match acquired.remove(&id.get_value()) {
                Some(Ok(data)) => {
                    AbilityActivationTargets::acquired(data.clone()).map_err(|error| {
                        AbilityCommandOutcome::SubmissionFailed(
                            AbilityCommandSubmissionError::InvalidTargets(error),
                        )
                    })
                }
                Some(Err(error)) => Err(AbilityCommandOutcome::TargetingFailed(error.clone())),
                None => Err(AbilityCommandOutcome::SubmissionFailed(
                    AbilityCommandSubmissionError::MissingTargetingResult,
                )),
            },
        };
        let mut completion = AbilityCommandResult {
            command_id: command.id,
            source: command.source,
            targeting_request_id,
            execution_request_id: None,
            outcome: AbilityCommandOutcome::SubmissionFailed(
                AbilityCommandSubmissionError::MissingExecutionResult,
            ),
        };
        match targets {
            Ok(targets) => {
                let chain = execution.new_root_chain(command.handle);
                let context = match command.origin {
                    AbilityCommandOrigin::Input => {
                        AbilityActivationContext::input(command.source, chain)
                    }
                    AbilityCommandOrigin::Direct => {
                        AbilityActivationContext::direct(command.source, chain)
                    }
                };
                match execution.push_activation(command.source, targets, command.handle, context) {
                    Ok(id) => completion.execution_request_id = Some(id),
                    Err(error) => {
                        completion.outcome = AbilityCommandOutcome::SubmissionFailed(
                            AbilityCommandSubmissionError::ExecutionQueue(error),
                        );
                    }
                }
            }
            Err(outcome) => completion.outcome = outcome,
        }
        state.completions.push(completion);
    }
}

pub(super) fn collect_ability_command_results(
    mut execution_results: MessageReader<GameplayExecutionResult>,
    mut results: MessageWriter<AbilityCommandResult>,
    mut state: ResMut<AbilityCommandState>,
) {
    if state.completions.is_empty() {
        execution_results.clear();
        return;
    }
    let mut executed = execution_results
        .read()
        .map(|result| (result.request_id.get_value(), &result.outcome))
        .collect::<BTreeMap<_, _>>();
    for mut completion in state.completions.drain(..) {
        if let Some(id) = completion.execution_request_id
            && let Some(outcome) = executed.remove(&id.get_value())
        {
            completion.outcome = match outcome {
                GameplayExecutionOutcome::Succeeded => AbilityCommandOutcome::Succeeded,
                GameplayExecutionOutcome::Rejected(error) => {
                    AbilityCommandOutcome::Rejected(error.clone())
                }
                GameplayExecutionOutcome::Failed(error) => {
                    AbilityCommandOutcome::Failed(error.clone())
                }
            };
        }
        results.write(completion);
    }
}
