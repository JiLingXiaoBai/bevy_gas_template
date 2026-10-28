//! Moves two selected units and casts their abilities through one ordered command stream.
//!
//! Run with `cargo run --example multi_unit_commands`. Selection order is captured before
//! the fixed tick. Move changes a game-owned goal; the movement system advances positions,
//! and the ability adapter submits casts to GAS in the same captured order.

use bevy::ecs::system::RunSystemOnce;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy_gas::gas::attributes::AttributeIdManager;
use bevy_gas::gas::gameplay_abilities::{AbilityTaskDef, AbilityTaskOnFinishedDef};
use bevy_gas::prelude::*;
use bevy_gas_template::application::SimulationPlugin;
use bevy_gas_template::gameplay::GameplaySet;
use bevy_gas_template::gameplay::commands::{
    AbilityCommand, AbilityCommandOrigin, AbilityCommandOutcome, AbilityCommandPlugin,
    AbilityCommandResult, AbilityCommandSet, GameCommandId, GameCommandQueue,
};
use bevy_gas_template::gameplay::position::LogicalPosition;
use std::error::Error;
use std::sync::Arc;

type ExampleResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Clone, Copy)]
enum UnitCommand {
    Move {
        source: Entity,
        destination: Vec3,
    },
    Cast {
        source: Entity,
        ability: AbilitySpecHandle,
        target: Entity,
    },
}

#[derive(Component, Default)]
struct MovementGoal(Option<Vec3>);

#[derive(Resource, Default)]
struct CommandReport {
    dispatched: Vec<(GameCommandId, Entity)>,
    successful_casts: Vec<Entity>,
    failures: usize,
}

struct Scenario {
    first: Entity,
    second: Entity,
    target: Entity,
    first_ability: AbilitySpecHandle,
    second_ability: AbilitySpecHandle,
    health: AttributeId,
}

fn main() -> ExampleResult<()> {
    run_example(true)
}

fn run_example(logging: bool) -> ExampleResult<()> {
    let (mut app, scenario) = make_app(logging)?;
    let submitted = {
        let mut commands = app
            .world_mut()
            .resource_mut::<GameCommandQueue<UnitCommand>>();
        // A selection list has meaningful order. Do not rebuild it from unordered ECS iteration.
        [
            commands.push(UnitCommand::Move {
                source: scenario.second,
                destination: Vec3::new(10.0, 0.0, 0.0),
            })?,
            commands.push(UnitCommand::Cast {
                source: scenario.second,
                ability: scenario.second_ability,
                target: scenario.target,
            })?,
            commands.push(UnitCommand::Move {
                source: scenario.first,
                destination: Vec3::new(5.0, 0.0, 0.0),
            })?,
            commands.push(UnitCommand::Cast {
                source: scenario.first,
                ability: scenario.first_ability,
                target: scenario.target,
            })?,
        ]
    };
    app.world_mut().run_schedule(FixedUpdate);

    let expected_order = [
        (submitted[0], scenario.second),
        (submitted[1], scenario.second),
        (submitted[2], scenario.first),
        (submitted[3], scenario.first),
    ];
    let report = app.world().resource::<CommandReport>();
    if report.dispatched != expected_order
        || report.successful_casts != [scenario.second, scenario.first]
        || report.failures != 0
    {
        return Err("unit commands or cast results did not preserve captured order".into());
    }
    check_position(&app, scenario.first, Vec3::X)?;
    check_position(&app, scenario.second, Vec3::ZERO)?;
    check_health(&mut app, &scenario, 80.0)?;
    info!("Both units moved one step and cast once in the captured selection order");

    // Movement continues on catch-up ticks, but a consumed command never casts again.
    app.world_mut().run_schedule(FixedUpdate);
    check_position(&app, scenario.first, Vec3::new(2.0, 0.0, 0.0))?;
    check_position(&app, scenario.second, Vec3::X)?;
    check_health(&mut app, &scenario, 80.0)?;
    let report = app.world().resource::<CommandReport>();
    if report.dispatched.len() != 4 || report.successful_casts.len() != 2 {
        return Err("a consumed unit command ran again on the next fixed tick".into());
    }
    info!("The next fixed tick advanced movement without repeating either cast");
    Ok(())
}

fn make_app(logging: bool) -> ExampleResult<(App, Scenario)> {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        SimulationPlugin::default(),
        AbilityCommandPlugin,
    ))
    .init_resource::<GameCommandQueue<UnitCommand>>()
    .init_resource::<CommandReport>()
    .add_systems(
        FixedUpdate,
        (
            dispatch_unit_commands
                .before(GameplaySet::Movement)
                .before(AbilityCommandSet::Prepare),
            move_units.in_set(GameplaySet::Movement),
            report_casts.after(AbilityCommandSet::Results),
        ),
    );
    if logging {
        app.add_plugins(LogPlugin::default());
    }
    app.finish();
    app.cleanup();
    let health = app
        .world_mut()
        .run_system_once(|mut register: AttributeIdRegister| {
            register.request_or_register_attribute_id("Health", AttributeRegion::Hot)
        })
        .map_err(|error| format!("attribute registration system failed: {error:?}"))??;
    let effect = Arc::new(GameplayEffect::new(
        vec![Modifier::new(
            health,
            ModifierOperation::AddBase,
            ModifierMagnitude::Flat(-10.0),
        )],
        EffectDurationTicks::Instant,
        None,
        1.0,
        StackingPolicy::non_stacking(),
        EffectTags::new(Vec::new(), Vec::new()),
    ));
    let ability =
        Arc::new(
            GameplayAbility::default().with_startup_tasks(vec![AbilityTaskDef::instant(
                AbilityTaskOnFinishedDef::Batch {
                    actions: vec![
                        AbilityTaskOnFinishedDef::ApplyGameplayEffectToTarget { effect },
                        AbilityTaskOnFinishedDef::EndAbility,
                    ],
                },
            )]),
        );
    let mut first = GameplayAbilitySystemBundle::default();
    let first_ability = first.ability_system.give_ability(Arc::clone(&ability), 1)?;
    let mut second = GameplayAbilitySystemBundle::default();
    let second_ability = second.ability_system.give_ability(ability, 1)?;
    let mut target = GameplayAbilitySystemBundle::default();
    target.attributes.initialize_attribute(
        app.world().resource::<AttributeIdManager>(),
        health,
        100.0,
        None,
    )?;
    let first = app
        .world_mut()
        .spawn((first, LogicalPosition(Vec3::ZERO), MovementGoal::default()))
        .id();
    let second = app
        .world_mut()
        .spawn((second, LogicalPosition(-Vec3::X), MovementGoal::default()))
        .id();
    let target = app.world_mut().spawn(target).id();
    Ok((
        app,
        Scenario {
            first,
            second,
            target,
            first_ability,
            second_ability,
            health,
        },
    ))
}

fn dispatch_unit_commands(
    mut commands: ResMut<GameCommandQueue<UnitCommand>>,
    mut abilities: ResMut<GameCommandQueue<AbilityCommand>>,
    mut goals: Query<&mut MovementGoal>,
    mut report: ResMut<CommandReport>,
) {
    while let Some((id, command)) = commands.pop() {
        let source = match command {
            UnitCommand::Move {
                source,
                destination,
            } => {
                if let Ok(mut goal) = goals.get_mut(source) {
                    goal.0 = Some(destination);
                } else {
                    report.failures += 1;
                    warn!(?source, "Move command has no live movement recipient");
                }
                source
            }
            UnitCommand::Cast {
                source,
                ability,
                target,
            } => {
                let command = AbilityCommand::direct(source, ability, target)
                    .with_origin(AbilityCommandOrigin::Direct);
                if let Err(error) = abilities.push(command) {
                    report.failures += 1;
                    error!(%error, "Unable to submit a unit's ability command");
                }
                source
            }
        };
        report.dispatched.push((id, source));
        info!(
            command_id = id.get_value(),
            ?source,
            "Dispatched unit command"
        );
    }
}

fn move_units(mut units: Query<(&mut LogicalPosition, &mut MovementGoal)>) {
    for (mut position, mut goal) in &mut units {
        let Some(destination) = goal.0 else {
            continue;
        };
        let displacement = destination - position.0;
        if displacement.length_squared() <= 1.0 {
            position.0 = destination;
            goal.0 = None;
        } else {
            position.0 += displacement.normalize();
        }
    }
}

fn report_casts(
    mut results: MessageReader<AbilityCommandResult>,
    mut report: ResMut<CommandReport>,
) {
    for result in results.read() {
        if matches!(result.outcome, AbilityCommandOutcome::Succeeded) {
            report.successful_casts.push(result.source);
        } else {
            report.failures += 1;
        }
        info!(
            command_id = result.command_id.get_value(),
            source = ?result.source,
            outcome = ?result.outcome,
            "Unit ability command completed"
        );
    }
}

fn check_position(app: &App, entity: Entity, expected: Vec3) -> ExampleResult<()> {
    let actual = app
        .world()
        .get::<LogicalPosition>(entity)
        .ok_or("unit is missing its logical position")?
        .0;
    if actual != expected {
        return Err(
            format!("unexpected position: actual={actual:?}, expected={expected:?}").into(),
        );
    }
    Ok(())
}

fn check_health(app: &mut App, scenario: &Scenario, expected: f32) -> ExampleResult<()> {
    let manager = app.world().resource::<AttributeIdManager>().clone();
    let actual = app
        .world_mut()
        .get_mut::<AttributeSet>(scenario.target)
        .ok_or("target is missing its attributes")?
        .get_current_value(&manager, scenario.health)?
        .ok_or("target Health was not initialized")?;
    if actual != expected {
        return Err(format!("unexpected Health: actual={actual}, expected={expected}").into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn selected_units_move_and_cast_once_in_order() {
        super::run_example(false)
            .expect("multi-unit commands should preserve order and execute once");
    }
}
