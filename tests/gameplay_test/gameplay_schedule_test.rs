use bevy::prelude::*;
use bevy_gas::{GameplayAbilitySystemPlugin, GameplayAbilitySystemSet};
use bevy_gas_template::gameplay::position::LogicalPosition;
use bevy_gas_template::gameplay::targeting::{
    Targetable, TargetingContinuation, TargetingDefinition, TargetingInput, TargetingOperation,
    TargetingPlugin, TargetingRequestQueue, TargetingResultEvent, TargetingSpatialIndex,
};
use bevy_gas_template::gameplay::{GameplaySchedulePlugin, GameplaySet};
use std::sync::Arc;

#[derive(Resource, Default)]
struct ObservedPositions(Vec<Vec3>);

#[derive(Resource, Default)]
struct CapturedTargets(Vec<Entity>);

#[test]
fn independent_schedule_exposes_deferred_movement_to_tasks_and_requests_in_the_same_tick() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        GameplayAbilitySystemPlugin,
        GameplaySchedulePlugin,
    ))
    .init_resource::<ObservedPositions>();
    let actor = app.world_mut().spawn(LogicalPosition(Vec3::ZERO)).id();

    app.add_systems(
        FixedUpdate,
        (move |mut commands: Commands| {
            commands.entity(actor).insert(LogicalPosition(Vec3::X));
        })
        .in_set(GameplayAbilitySystemSet::EffectTicks),
    )
    .add_systems(
        FixedUpdate,
        (move |positions: Query<&LogicalPosition>, mut commands: Commands| {
            let position = positions.get(actor).unwrap().0;
            assert_eq!(position, Vec3::X);
            commands
                .entity(actor)
                .insert(LogicalPosition(position + Vec3::Y));
        })
        .in_set(GameplaySet::Movement),
    )
    .add_systems(
        FixedUpdate,
        (move |positions: Query<&LogicalPosition>, mut observed: ResMut<ObservedPositions>| {
            observed.0.push(positions.get(actor).unwrap().0);
        })
        .in_set(GameplayAbilitySystemSet::AbilityTasks),
    )
    .add_systems(
        FixedUpdate,
        (move |positions: Query<&LogicalPosition>, mut observed: ResMut<ObservedPositions>| {
            observed.0.push(positions.get(actor).unwrap().0);
        })
        .in_set(GameplayAbilitySystemSet::RequestProducers),
    );

    assert!(!app.is_plugin_added::<TargetingPlugin>());
    assert!(!app.world().contains_resource::<TargetingSpatialIndex>());
    assert!(!app.world().contains_resource::<TargetingRequestQueue>());

    for _ in 0..2 {
        app.world_mut().run_schedule(FixedUpdate);
    }

    assert_eq!(
        app.world().resource::<ObservedPositions>().0,
        vec![Vec3::X + Vec3::Y; 4]
    );
    assert!(!app.world().contains_resource::<TargetingSpatialIndex>());
    assert!(!app.world().contains_resource::<TargetingRequestQueue>());
}

#[test]
fn targeting_installs_schedule_and_acquires_deferred_movement_in_the_same_tick() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, GameplayAbilitySystemPlugin, TargetingPlugin));

    assert!(app.is_plugin_added::<GameplaySchedulePlugin>());
    assert_targeting_observes_deferred_movement(&mut app);
}

#[test]
fn explicitly_installed_schedule_composes_with_targeting() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        GameplayAbilitySystemPlugin,
        GameplaySchedulePlugin,
        TargetingPlugin,
    ));

    assert!(app.is_plugin_added::<GameplaySchedulePlugin>());
    assert_targeting_observes_deferred_movement(&mut app);
}

fn assert_targeting_observes_deferred_movement(app: &mut App) {
    app.init_resource::<CapturedTargets>();
    app.world_mut().add_observer(
        |event: On<TargetingResultEvent>, mut captured: ResMut<CapturedTargets>| {
            captured.0.extend(
                event
                    .get_result()
                    .as_ref()
                    .unwrap()
                    .get_hits()
                    .iter()
                    .map(|hit| hit.get_entity()),
            );
        },
    );
    let source = app.world_mut().spawn(LogicalPosition(Vec3::ZERO)).id();
    let target = app
        .world_mut()
        .spawn((Targetable, LogicalPosition(Vec3::X * 100.0)))
        .id();
    app.add_systems(
        FixedUpdate,
        (move |mut commands: Commands| {
            commands.entity(target).insert(LogicalPosition(Vec3::X));
        })
        .in_set(GameplaySet::Movement),
    );
    let definition = Arc::new(
        TargetingDefinition::new(vec![TargetingOperation::SelectSphere { radius: 2.0 }]).unwrap(),
    );
    app.world_mut()
        .resource_mut::<TargetingRequestQueue>()
        .push_request(
            source,
            TargetingInput::new(Vec3::ZERO, Vec3::X),
            definition,
            TargetingContinuation::EmitResult,
        );

    app.world_mut().run_schedule(FixedUpdate);

    assert_eq!(app.world().resource::<CapturedTargets>().0, vec![target]);
    assert_eq!(app.world().resource::<TargetingSpatialIndex>().len(), 1);
}
