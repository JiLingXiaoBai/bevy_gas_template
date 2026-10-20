use super::support_test::{
    add_modifier, attribute_set, current_value, empty_effect_tags, give_ability,
    register_attribute, run_fixed_update,
};
use bevy::prelude::*;
use bevy::shape::Sphere;
use bevy_gas::{
    AbilitySpecHandle, AbilityTask, AbilityTaskDef, AbilityTaskOnFinishedDef,
    ActiveGameplayAbility, ActiveGameplayEffects, AttributeId, AttributeIdManager,
    EffectDurationTicks, EffectRequirementDiagnostics, GameplayAbility,
    GameplayAbilitySystemBundle, GameplayEffect, GameplayExecutionQueue, ModifierMagnitude, Random,
    StackingPolicy,
};
use bevy_gas_template::application::{SimulationPlugin, SimulationSettings};
use bevy_gas_template::gameplay::GameplaySet;
use bevy_gas_template::gameplay::commands::{
    AbilityCommand, AbilityCommandOutcome, AbilityCommandPlugin, AbilityCommandResult,
    AbilityCommandSet, GameCommandQueue,
};
use bevy_gas_template::gameplay::position::LogicalPosition;
use bevy_gas_template::gameplay::randoms::SpatialRandom;
use bevy_gas_template::gameplay::targeting::{
    Targetable, TargetingDefinition, TargetingInput, TargetingOperation, TargetingPlugin,
    TargetingRequestQueue, TargetingSortOrder, TargetingSpatialIndex,
};
use std::sync::Arc;
use std::time::{Duration, Instant};

const CAST_INTERVAL: usize = 6;
const INITIAL_HEALTH: f32 = 5000.0;

#[derive(Component)]
struct SimulationUnit(usize);

#[derive(Resource)]
struct UnitOrder(Vec<Entity>);

#[derive(Debug, Clone, PartialEq, Eq)]
struct CommandRecord {
    command: u64,
    source: usize,
    targeted: bool,
    submitted: bool,
    succeeded: bool,
}

#[derive(Resource, Default)]
struct ObservedCommands(Vec<CommandRecord>);

#[derive(Debug, PartialEq)]
struct UnitSnapshot {
    id: usize,
    position: Vec3,
    health: f32,
    power: f32,
    effects: usize,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct RuntimeCounts {
    effects: usize,
    abilities: usize,
    tasks: usize,
}

#[derive(Debug, PartialEq)]
struct TickSnapshot {
    units: Vec<UnitSnapshot>,
    runtime: RuntimeCounts,
    commands: Vec<CommandRecord>,
}

struct Scenario {
    app: App,
    units: Vec<Entity>,
    handles: Vec<AbilitySpecHandle>,
    health: AttributeId,
    power: AttributeId,
    targeting: Arc<TargetingDefinition>,
}

#[test]
fn simulation_settings_reject_invalid_rates_and_preserve_explicit_values() {
    assert!(SimulationSettings::new(Duration::ZERO).is_err());
    for rate in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(SimulationSettings::try_from_hz(rate).is_err());
    }
    let settings = SimulationSettings::try_from_hz(20.0)
        .unwrap()
        .with_random_seed(42);
    assert_eq!(settings.timestep(), Duration::from_millis(50));
    assert_eq!(settings.random_seed(), 42);
}

#[test]
fn shared_simulation_installs_time_and_seed_without_spatial_or_command_plugins() {
    let settings = SimulationSettings::new(Duration::from_millis(25))
        .unwrap()
        .with_random_seed(42);
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, SimulationPlugin::new(settings)));

    assert_eq!(
        app.world().resource::<Time<Fixed>>().timestep(),
        Duration::from_millis(25),
    );
    assert!(app.world().contains_resource::<AttributeIdManager>());
    assert!(app.world().contains_resource::<GameplayExecutionQueue>());
    assert!(!app.is_plugin_added::<TargetingPlugin>());
    assert!(!app.world().contains_resource::<TargetingSpatialIndex>());
    assert!(!app.world().contains_resource::<TargetingRequestQueue>());
    assert!(
        !app.world()
            .contains_resource::<GameCommandQueue<AbilityCommand>>()
    );
    assert!(!app.world().contains_resource::<SpatialRandom>());
    let mut expected = Random::from_seed(42);
    for _ in 0..16 {
        assert_eq!(
            app.world_mut().resource_mut::<Random>().random_bool(0.37),
            expected.random_bool(0.37),
        );
    }
    run_fixed_update(&mut app);
}

#[test]
fn identical_seed_and_ordered_commands_replay_each_fixed_tick() {
    // This checks repeatability in this build, not cross-platform lockstep compatibility.
    let first = replay(42, false);
    let second = replay(42, false);
    assert_eq!(first, second);
    assert_ne!(first, replay(42, true));
    assert_ne!(first, replay(43, false));
    assert_eq!(
        first.iter().map(|tick| tick.commands.len()).sum::<usize>(),
        40
    );
    assert!(
        first
            .iter()
            .flat_map(|tick| &tick.commands)
            .all(|result| result.succeeded)
    );
    assert!(
        first
            .iter()
            .any(|tick| tick.runtime.effects > 0 && tick.runtime.tasks > 0)
    );
    assert!(
        first
            .last()
            .unwrap()
            .units
            .iter()
            .any(|unit| unit.health < INITIAL_HEALTH)
    );
    assert_eq!(first.last().unwrap().runtime, RuntimeCounts::default());
}

#[test]
#[ignore = "Measures complete simulation ticks; run explicitly with --nocapture in release mode."]
fn measure_multi_unit_effect_and_task_workloads() {
    const TICKS: usize = 24;
    for unit_count in [128, 1024] {
        let mut scenario = scenario(unit_count, 42, 1.0);
        scenario
            .app
            .world_mut()
            .resource_mut::<EffectRequirementDiagnostics>()
            .set_enabled(true);
        run_fixed_update(&mut scenario.app);
        scenario
            .app
            .world_mut()
            .resource_mut::<EffectRequirementDiagnostics>()
            .reset();
        let mut elapsed = Duration::ZERO;
        let mut peak = RuntimeCounts::default();
        for tick in 0..TICKS {
            if tick % CAST_INTERVAL == 0 {
                submit_round(&mut scenario, false);
            }
            let started = Instant::now();
            run_fixed_update(&mut scenario.app);
            elapsed += started.elapsed();
            assert_queues_empty(&scenario.app);
            let counts = runtime_counts(scenario.app.world_mut());
            peak.effects = peak.effects.max(counts.effects);
            peak.abilities = peak.abilities.max(counts.abilities);
            peak.tasks = peak.tasks.max(counts.tasks);
        }
        let expected_commands = unit_count * (TICKS / CAST_INTERVAL);
        let results = &scenario.app.world().resource::<ObservedCommands>().0;
        assert_eq!(results.len(), expected_commands);
        assert!(results.iter().all(|result| result.succeeded));
        assert!(peak.effects >= unit_count);
        assert_eq!(peak.abilities, unit_count);
        assert_eq!(peak.tasks, unit_count * 2);
        assert_eq!(
            runtime_counts(scenario.app.world_mut()),
            RuntimeCounts::default()
        );
        let metrics = scenario
            .app
            .world()
            .resource::<EffectRequirementDiagnostics>()
            .metrics();
        println!(
            "units={unit_count} ticks={TICKS} commands={expected_commands} peak_effects={} peak_abilities={} peak_tasks={} total_ms={:.3} mean_tick_ms={:.3} convergence_calls={} decision_effect_visits={} effect_snapshots={} convergence_ms={:.3}",
            peak.effects,
            peak.abilities,
            peak.tasks,
            elapsed.as_secs_f64() * 1000.0,
            elapsed.as_secs_f64() * 1000.0 / TICKS as f64,
            metrics.convergence_calls,
            metrics.decision_effect_visits,
            metrics.effect_snapshots,
            metrics.elapsed.as_secs_f64() * 1000.0,
        );
    }
}

fn replay(seed: u64, skip_first: bool) -> Vec<TickSnapshot> {
    let mut scenario = scenario(8, seed, 0.5);
    let mut snapshots = Vec::new();
    for tick in 0..30 {
        if tick % CAST_INTERVAL == 0 {
            submit_round(&mut scenario, skip_first && tick == 0);
        }
        run_fixed_update(&mut scenario.app);
        assert_queues_empty(&scenario.app);
        let mut units = Vec::new();
        for (id, &entity) in scenario.units.iter().enumerate() {
            units.push(UnitSnapshot {
                id,
                position: scenario
                    .app
                    .world()
                    .get::<LogicalPosition>(entity)
                    .unwrap()
                    .0,
                health: current_value(&mut scenario.app, entity, scenario.health),
                power: current_value(&mut scenario.app, entity, scenario.power),
                effects: scenario
                    .app
                    .world()
                    .get::<ActiveGameplayEffects>(entity)
                    .unwrap()
                    .len(),
            });
        }
        snapshots.push(TickSnapshot {
            units,
            runtime: runtime_counts(scenario.app.world_mut()),
            commands: std::mem::take(
                &mut scenario
                    .app
                    .world_mut()
                    .resource_mut::<ObservedCommands>()
                    .0,
            ),
        });
    }
    snapshots
}

fn scenario(unit_count: usize, seed: u64, damage_probability: f32) -> Scenario {
    let mut app = App::new();
    let settings = SimulationSettings::default().with_random_seed(seed);
    app.add_plugins((
        MinimalPlugins,
        SimulationPlugin::new(settings),
        AbilityCommandPlugin,
    ))
    .insert_resource(SpatialRandom::from_seed(seed))
    .init_resource::<ObservedCommands>()
    .add_systems(FixedUpdate, move_units.in_set(GameplaySet::Movement))
    .add_systems(
        FixedUpdate,
        record_commands.after(AbilityCommandSet::Results),
    );
    let health = register_attribute(&mut app, "Simulation.Health");
    let power = register_attribute(&mut app, "Simulation.Power");
    let damage = effect(health, -2.0, EffectDurationTicks::Instant, 1.0);
    let bonus_damage = effect(
        health,
        -3.0,
        EffectDurationTicks::Instant,
        damage_probability,
    );
    let buff = effect(
        power,
        1.0,
        EffectDurationTicks::DurationTicks(ModifierMagnitude::Flat(4.0)),
        1.0,
    );
    let ability = Arc::new(GameplayAbility::default().with_startup_tasks(vec![
        AbilityTaskDef::instant(AbilityTaskOnFinishedDef::Batch {
            actions: vec![
                AbilityTaskOnFinishedDef::ApplyGameplayEffectToTargets { effect: damage },
                AbilityTaskOnFinishedDef::ApplyGameplayEffectToTargets { effect: buff },
            ],
        }),
        AbilityTaskDef::wait_ticks(
            1,
            AbilityTaskOnFinishedDef::ApplyGameplayEffectToTargets {
                effect: bonus_damage,
            },
        ),
        AbilityTaskDef::wait_ticks(5, AbilityTaskOnFinishedDef::EndAbility),
    ]));
    let mut units = Vec::with_capacity(unit_count);
    let mut handles = Vec::with_capacity(unit_count);
    for id in 0..unit_count {
        let mut attributes = attribute_set(&app, health, INITIAL_HEALTH);
        attributes
            .initialize_attribute(
                app.world().resource::<AttributeIdManager>(),
                power,
                0.0,
                None,
            )
            .unwrap();
        let unit = app
            .world_mut()
            .spawn((
                GameplayAbilitySystemBundle {
                    attributes,
                    ..Default::default()
                },
                SimulationUnit(id),
                LogicalPosition(Vec3::new(id as f32 * 2.0, 0.0, 0.0)),
                Targetable,
            ))
            .id();
        handles.push(give_ability(&mut app, unit, Arc::clone(&ability)));
        units.push(unit);
    }
    app.insert_resource(UnitOrder(units.clone()));
    app.finish();
    app.cleanup();
    Scenario {
        app,
        units,
        handles,
        health,
        power,
        targeting: Arc::new(
            TargetingDefinition::new(vec![
                TargetingOperation::SelectSphere { radius: 8.0 },
                TargetingOperation::FilterSource,
                TargetingOperation::SortByDistance {
                    order: TargetingSortOrder::Ascending,
                },
                TargetingOperation::Limit { count: 2 },
            ])
            .unwrap(),
        ),
    }
}

fn effect(
    attribute: AttributeId,
    amount: f32,
    duration: EffectDurationTicks,
    probability: f32,
) -> Arc<GameplayEffect> {
    Arc::new(GameplayEffect::new(
        vec![add_modifier(attribute, amount)],
        duration,
        None,
        probability,
        StackingPolicy::non_stacking(),
        empty_effect_tags(),
    ))
}

fn submit_round(scenario: &mut Scenario, skip_first: bool) {
    // Explicit actor order makes FIFO submission and random consumption reproducible.
    for (index, &source) in scenario.units.iter().enumerate() {
        if skip_first && index == 0 {
            continue;
        }
        let handle = scenario.handles[index];
        let command = if index % 2 == 0 {
            AbilityCommand::direct(
                source,
                handle,
                scenario.units[(index + 1) % scenario.units.len()],
            )
        } else {
            let origin = scenario
                .app
                .world()
                .get::<LogicalPosition>(source)
                .unwrap()
                .0;
            AbilityCommand::targeted(
                source,
                handle,
                TargetingInput::new(origin, Vec3::X),
                Arc::clone(&scenario.targeting),
            )
        };
        scenario
            .app
            .world_mut()
            .resource_mut::<GameCommandQueue<AbilityCommand>>()
            .push(command)
            .unwrap();
    }
}

fn move_units(
    order: Res<UnitOrder>,
    mut random: ResMut<SpatialRandom>,
    mut positions: Query<&mut LogicalPosition>,
) {
    let movement = Sphere::new(0.125);
    for &entity in &order.0 {
        positions.get_mut(entity).unwrap().0 += random.sample_interior(&movement);
    }
}

fn record_commands(
    mut messages: MessageReader<AbilityCommandResult>,
    units: Query<&SimulationUnit>,
    mut observed: ResMut<ObservedCommands>,
) {
    for result in messages.read() {
        observed.0.push(CommandRecord {
            command: result.command_id.get_value(),
            source: units.get(result.source).unwrap().0,
            targeted: result.targeting_request_id.is_some(),
            submitted: result.execution_request_id.is_some(),
            succeeded: matches!(result.outcome, AbilityCommandOutcome::Succeeded),
        });
    }
}

fn assert_queues_empty(app: &App) {
    assert!(
        app.world()
            .resource::<GameCommandQueue<AbilityCommand>>()
            .is_empty()
    );
    assert!(app.world().resource::<TargetingRequestQueue>().is_empty());
    assert!(app.world().resource::<GameplayExecutionQueue>().is_empty());
}

fn runtime_counts(world: &mut World) -> RuntimeCounts {
    RuntimeCounts {
        effects: world
            .query::<&ActiveGameplayEffects>()
            .iter(world)
            .map(ActiveGameplayEffects::len)
            .sum(),
        abilities: world
            .query_filtered::<Entity, With<ActiveGameplayAbility>>()
            .iter(world)
            .count(),
        tasks: world
            .query_filtered::<Entity, With<AbilityTask>>()
            .iter(world)
            .count(),
    }
}
