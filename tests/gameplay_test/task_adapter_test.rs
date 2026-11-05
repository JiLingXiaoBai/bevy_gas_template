use bevy::prelude::*;
use bevy_gas::{
    AbilityActivationContext, AbilityActivationStatus, AbilityChainContext, AbilitySpecHandle,
    AbilityTaskExecution, AbilityTaskExecutionContext, AbilityTaskNode, AbilityTaskProgram,
    ActiveGameplayAbility, CustomAbilityTask, GameplayAbilitySystemSet, attach_ability_task_graph,
};
use bevy_gas_template::{
    application::{SimulationPlugin, SimulationSettings},
    gameplay::{GameplaySet, ability_tasks::phase_sample},
};

#[derive(Component, Clone)]
struct SampleTask;
impl CustomAbilityTask for SampleTask {}
#[derive(Resource, Default)]
struct Sample {
    movement_revision: u32,
    observations: Vec<(u32, usize)>,
}

#[test]
fn custom_task_queries_the_post_movement_world_and_samples_its_own_interval() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        SimulationPlugin::new(SimulationSettings::default()),
    ))
    .init_resource::<Sample>()
    .add_systems(
        FixedUpdate,
        (|mut sample: ResMut<Sample>| sample.movement_revision += 1).in_set(GameplaySet::Movement),
    )
    .add_systems(
        FixedUpdate,
        (|tasks: Query<&AbilityTaskExecution, With<SampleTask>>, mut sample: ResMut<Sample>| {
            for task in &tasks {
                if task.is_running() {
                    let revision = sample.movement_revision;
                    sample.observations.push((
                        revision,
                        phase_sample(task.sample_tick(), task.duration_ticks(), 3),
                    ));
                }
            }
        })
        .in_set(GameplayAbilitySystemSet::RequestProducers),
    );
    let source = app.world_mut().spawn_empty().id();
    let handle = AbilitySpecHandle::new(1);
    let context = AbilityActivationContext::direct(source, AbilityChainContext::root(handle, 1));
    let ability = app
        .world_mut()
        .spawn(ActiveGameplayAbility::new(
            source,
            handle,
            source,
            AbilityActivationStatus::Active,
            context,
        ))
        .id();
    let root = app.world_mut().spawn_empty().id();
    let plan = AbilityTaskProgram::compile(AbilityTaskNode::Sequence(vec![
        AbilityTaskNode::wait(1),
        AbilityTaskNode::task(3, SampleTask),
    ]))
    .expect("program");
    attach_ability_task_graph(
        app.world_mut(),
        root,
        ability,
        AbilityTaskExecutionContext::new(source, handle, 1),
        &plan,
        true,
    )
    .expect("attach");
    for _ in 0..4 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    assert_eq!(
        app.world().resource::<Sample>().observations,
        [(2, 0), (3, 1), (4, 2)]
    );
}

#[test]
fn phase_sampling_clamps_handles_empty_clips_and_avoids_overflow() {
    assert_eq!(phase_sample(u32::MAX, 10, 4), 3);
    assert_eq!(phase_sample(0, 0, 0), 0);
    assert!(phase_sample(u32::MAX, u32::MAX, usize::MAX) < usize::MAX);
}
