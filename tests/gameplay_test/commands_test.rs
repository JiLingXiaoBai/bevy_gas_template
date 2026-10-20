use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy_gas::{
    AbilityActivationContext, AbilityActivationReason, AbilitySpecHandle, ActiveGameplayAbility,
    GameplayAbility, GameplayAbilitySystemBundle, GameplayAbilitySystemPlugin,
    GameplayAbilitySystemSet, GameplayExecutionQueue,
};
use bevy_gas_template::gameplay::GameplaySet;
use bevy_gas_template::gameplay::commands::{
    AbilityCommand, AbilityCommandOrigin, AbilityCommandOutcome, AbilityCommandPlugin,
    AbilityCommandResult, AbilityCommandSet, AbilityCommandSubmissionError, GameCommandQueue,
};
use bevy_gas_template::gameplay::position::LogicalPosition;
use bevy_gas_template::gameplay::targeting::{
    Targetable, TargetingContinuation, TargetingDefinition, TargetingError, TargetingInput,
    TargetingOperation, TargetingRequestQueue, TargetingRequestResult,
};
use std::sync::Arc;
use std::time::Duration;

#[derive(Resource, Default)]
struct CapturedResults(Vec<AbilityCommandResult>);

fn collect_results(
    mut messages: MessageReader<AbilityCommandResult>,
    mut captured: ResMut<CapturedResults>,
) {
    captured.0.extend(messages.read().cloned());
}

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        GameplayAbilitySystemPlugin,
        AbilityCommandPlugin,
    ))
    .init_resource::<CapturedResults>()
    .add_systems(
        FixedUpdate,
        collect_results.after(AbilityCommandSet::Results),
    );
    app
}

fn actor(app: &mut App, allow_multiple: bool) -> (Entity, AbilitySpecHandle) {
    let mut bundle = GameplayAbilitySystemBundle::default();
    let handle = bundle
        .ability_system
        .give_ability(
            Arc::new(GameplayAbility::default().with_allow_multiple_instances(allow_multiple)),
            1,
        )
        .unwrap();
    let actor = app
        .world_mut()
        .spawn((bundle, LogicalPosition(Vec3::ZERO)))
        .id();
    (actor, handle)
}

fn explicit_definition() -> Arc<TargetingDefinition> {
    Arc::new(TargetingDefinition::new(vec![TargetingOperation::SelectExplicitEntity]).unwrap())
}

#[test]
fn generic_queue_is_independent_and_does_not_reuse_cleared_ids() {
    let mut queue = GameCommandQueue::<&str>::default();
    let first = queue.push("move").unwrap();
    let second = queue.push("build").unwrap();
    assert_eq!(queue.pop(), Some((first, "move")));
    queue.clear();
    let third = queue.push("attack").unwrap();
    assert!(third.get_value() > second.get_value());
    assert_eq!(queue.pop(), Some((third, "attack")));
    assert!(queue.is_empty());
}

#[test]
fn mixed_commands_keep_fifo_and_report_once_after_frames_without_ticks() {
    let mut app = app();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
    let (source, handle) = actor(&mut app, false);
    let target = app
        .world_mut()
        .spawn((Targetable, LogicalPosition(Vec3::X)))
        .id();
    let first = app
        .world_mut()
        .resource_mut::<GameCommandQueue<AbilityCommand>>()
        .push(AbilityCommand::targeted(
            source,
            handle,
            TargetingInput::new(Vec3::ZERO, Vec3::X).with_explicit_target(target),
            explicit_definition(),
        ))
        .unwrap();
    let second = app
        .world_mut()
        .resource_mut::<GameCommandQueue<AbilityCommand>>()
        .push(AbilityCommand::direct(source, handle, source))
        .unwrap();
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(
        app.world()
            .resource::<GameCommandQueue<AbilityCommand>>()
            .len(),
        2
    );
    app.world_mut().run_schedule(FixedUpdate);
    let results = &app.world().resource::<CapturedResults>().0;
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].command_id, first);
    assert_eq!(results[0].outcome, AbilityCommandOutcome::Succeeded);
    assert!(results[0].targeting_request_id.is_some());
    assert_eq!(results[1].command_id, second);
    assert!(matches!(
        results[1].outcome,
        AbilityCommandOutcome::Rejected(_)
    ));
    assert!(results[1].targeting_request_id.is_none());
    assert!(
        results[0].execution_request_id.unwrap().get_value()
            < results[1].execution_request_id.unwrap().get_value()
    );
    let mut active = app.world_mut().query::<&ActiveGameplayAbility>();
    let abilities = active.iter(app.world()).collect::<Vec<_>>();
    assert_eq!(abilities.len(), 1);
    assert_eq!(abilities[0].get_target(), target);
    assert_eq!(
        abilities[0].get_activation_context().get_reason(),
        AbilityActivationReason::Input
    );
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().run_schedule(FixedUpdate);
    assert_eq!(app.world().resource::<CapturedResults>().0.len(), 2);
}

#[test]
fn targeting_failure_does_not_block_later_commands_and_origin_can_be_direct() {
    let mut app = app();
    let (source, handle) = actor(&mut app, true);
    let missing_target = app.world_mut().spawn_empty().id();
    app.world_mut().despawn(missing_target);
    let failed = app
        .world_mut()
        .resource_mut::<GameCommandQueue<AbilityCommand>>()
        .push(AbilityCommand::targeted(
            source,
            handle,
            TargetingInput::new(Vec3::ZERO, Vec3::X).with_explicit_target(missing_target),
            explicit_definition(),
        ))
        .unwrap();
    let succeeded = app
        .world_mut()
        .resource_mut::<GameCommandQueue<AbilityCommand>>()
        .push(
            AbilityCommand::direct(source, handle, source)
                .with_origin(AbilityCommandOrigin::Direct),
        )
        .unwrap();
    app.world_mut().run_schedule(FixedUpdate);
    let results = &app.world().resource::<CapturedResults>().0;
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].command_id, failed);
    assert_eq!(
        results[0].outcome,
        AbilityCommandOutcome::TargetingFailed(TargetingError::TargetNotFound {
            target: missing_target
        })
    );
    assert!(results[0].targeting_request_id.is_some());
    assert!(results[0].execution_request_id.is_none());
    assert_eq!(results[1].command_id, succeeded);
    assert_eq!(results[1].outcome, AbilityCommandOutcome::Succeeded);
    let mut active = app.world_mut().query::<&ActiveGameplayAbility>();
    assert_eq!(
        active
            .iter(app.world())
            .next()
            .unwrap()
            .get_activation_context()
            .get_reason(),
        AbilityActivationReason::Direct
    );
}

#[test]
fn empty_range_selection_reports_a_terminal_command_result() {
    let mut app = app();
    let (source, handle) = actor(&mut app, true);
    let id = app
        .world_mut()
        .resource_mut::<GameCommandQueue<AbilityCommand>>()
        .push(AbilityCommand::targeted(
            source,
            handle,
            TargetingInput::new(Vec3::ZERO, Vec3::X),
            Arc::new(
                TargetingDefinition::new(vec![TargetingOperation::SelectSphere { radius: 10.0 }])
                    .unwrap(),
            ),
        ))
        .unwrap();
    app.world_mut().run_schedule(FixedUpdate);
    let results = &app.world().resource::<CapturedResults>().0;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].command_id, id);
    assert_eq!(
        results[0].outcome,
        AbilityCommandOutcome::TargetingFailed(TargetingError::NoTargetsFound)
    );
    assert!(results[0].execution_request_id.is_none());
}

#[test]
fn despawned_command_source_reports_gas_failure() {
    let mut app = app();
    let (source, handle) = actor(&mut app, true);
    let id = app
        .world_mut()
        .resource_mut::<GameCommandQueue<AbilityCommand>>()
        .push(AbilityCommand::direct(source, handle, source))
        .unwrap();
    app.world_mut().despawn(source);
    app.world_mut().run_schedule(FixedUpdate);
    let results = &app.world().resource::<CapturedResults>().0;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].command_id, id);
    assert!(results[0].execution_request_id.is_some());
    assert!(matches!(
        results[0].outcome,
        AbilityCommandOutcome::Failed(_)
    ));
}

fn discard_targeting(mut queue: ResMut<TargetingRequestQueue>) {
    queue.clear();
}

fn discard_execution(mut queue: ResMut<GameplayExecutionQueue>) {
    queue.clear();
}

#[test]
fn externally_cleared_downstream_queues_end_commands_without_leaking_pending_work() {
    let mut app = app();
    let (source, handle) = actor(&mut app, true);
    app.add_systems(
        FixedUpdate,
        discard_targeting
            .after(AbilityCommandSet::Prepare)
            .before(GameplaySet::SpatialSync),
    )
    .add_systems(
        FixedUpdate,
        discard_execution
            .after(AbilityCommandSet::Submit)
            .before(GameplayAbilitySystemSet::GameplayResolve),
    );
    {
        let mut queue = app
            .world_mut()
            .resource_mut::<GameCommandQueue<AbilityCommand>>();
        queue
            .push(AbilityCommand::targeted(
                source,
                handle,
                TargetingInput::new(Vec3::ZERO, Vec3::X),
                explicit_definition(),
            ))
            .unwrap();
        queue
            .push(AbilityCommand::direct(source, handle, source))
            .unwrap();
    }
    app.world_mut().run_schedule(FixedUpdate);
    let results = &app.world().resource::<CapturedResults>().0;
    assert_eq!(results.len(), 2);
    assert_eq!(
        results[0].outcome,
        AbilityCommandOutcome::SubmissionFailed(
            AbilityCommandSubmissionError::MissingTargetingResult
        )
    );
    assert_eq!(
        results[1].outcome,
        AbilityCommandOutcome::SubmissionFailed(
            AbilityCommandSubmissionError::MissingExecutionResult
        )
    );
    app.world_mut().run_schedule(FixedUpdate);
    assert_eq!(app.world().resource::<CapturedResults>().0.len(), 2);
}

#[derive(Resource, Default)]
struct CapturedTargeting(Vec<TargetingRequestResult>);

fn collect_targeting(
    mut messages: MessageReader<TargetingRequestResult>,
    mut captured: ResMut<CapturedTargeting>,
) {
    captured.0.extend(messages.read().cloned());
}

#[test]
fn existing_targeting_continuations_report_the_accepted_execution_id() {
    let mut app = app();
    app.init_resource::<CapturedTargeting>()
        .add_systems(FixedUpdate, collect_targeting.after(GameplaySet::Targeting));
    let (source, handle) = actor(&mut app, true);
    let context = {
        let mut queue = app.world_mut().resource_mut::<GameplayExecutionQueue>();
        AbilityActivationContext::direct(source, queue.new_root_chain(handle))
    };
    let request_id = app
        .world_mut()
        .resource_mut::<TargetingRequestQueue>()
        .push_request(
            source,
            TargetingInput::new(Vec3::ZERO, Vec3::X),
            Arc::new(TargetingDefinition::new(vec![TargetingOperation::SelectSelf]).unwrap()),
            TargetingContinuation::activate_ability(handle, context),
        );
    app.world_mut().run_schedule(FixedUpdate);
    let results = &app.world().resource::<CapturedTargeting>().0;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].request_id, request_id);
    assert!(results[0].result.is_ok());
    assert!(results[0].continuation.as_ref().unwrap().is_ok());
    assert!(app.world().resource::<CapturedResults>().0.is_empty());
}
