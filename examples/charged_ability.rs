//! Demonstrates game-owned charging, release, cancellation, and timeout before GAS activation.
//!
//! Run with `cargo run --example charged_ability`. Charging is not an active GAS ability:
//! release submits the ordinary ability command and GAS then commits cost and cooldown.
//! This does not add WaitSignal tasks or deferred commit to already activated abilities.

use bevy::ecs::system::RunSystemOnce;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy_gas::gas::attributes::AttributeIdManager;
use bevy_gas::gas::gameplay_abilities::{AbilityTaskDef, AbilityTaskOnFinishedDef};
use bevy_gas::prelude::*;
use bevy_gas_template::application::SimulationPlugin;
use bevy_gas_template::gameplay::GameplaySet;
use bevy_gas_template::gameplay::commands::{
    AbilityCommand, AbilityCommandOutcome, AbilityCommandPlugin, AbilityCommandResult,
    AbilityCommandSet, GameCommandQueue,
};
use std::error::Error;
use std::sync::Arc;

type ExampleResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

const CHARGE_TIMEOUT_TICKS: u32 = 3;

#[derive(Clone, Copy)]
enum ChargeCommand {
    Begin {
        source: Entity,
        ability: AbilitySpecHandle,
        target: Entity,
    },
    Release {
        source: Entity,
    },
    Cancel {
        source: Entity,
    },
}

struct Charging {
    ability: AbilitySpecHandle,
    target: Entity,
    remaining_ticks: u32,
}

// The slot exists on the actor even while idle, so same-tick changes are immediately visible.
#[derive(Component, Default)]
struct ChargeSlot(Option<Charging>);

#[derive(Resource, Default)]
struct ChargeReport {
    successful_casts: usize,
    failed_casts: usize,
}

#[derive(Clone, Copy)]
struct Scenario {
    source: Entity,
    target: Entity,
    ability: AbilitySpecHandle,
    mana: AttributeId,
    health: AttributeId,
}

impl Scenario {
    fn begin(self) -> ChargeCommand {
        ChargeCommand::Begin {
            source: self.source,
            ability: self.ability,
            target: self.target,
        }
    }

    fn release(self) -> ChargeCommand {
        ChargeCommand::Release {
            source: self.source,
        }
    }

    fn cancel(self) -> ChargeCommand {
        ChargeCommand::Cancel {
            source: self.source,
        }
    }
}

fn main() -> ExampleResult<()> {
    let (mut app, scenario) = make_app(true)?;
    enqueue(&mut app, scenario.begin())?;
    tick(&mut app);
    check_state(&mut app, scenario, 50.0, 100.0, 0, true)?;
    info!("Charging started: no GAS activation and no cost has been committed");

    enqueue(&mut app, scenario.release())?;
    enqueue(&mut app, scenario.release())?;
    tick(&mut app);
    check_state(&mut app, scenario, 40.0, 90.0, 1, false)?;
    info!("Repeated release submitted one activation and spent exactly 10 Mana");

    enqueue(&mut app, scenario.begin())?;
    enqueue(&mut app, scenario.cancel())?;
    enqueue(&mut app, scenario.release())?;
    tick(&mut app);
    check_state(&mut app, scenario, 40.0, 90.0, 1, false)?;
    info!("Begin, cancel, and release in one tick spent no additional Mana");

    enqueue(&mut app, scenario.begin())?;
    tick(&mut app);
    for _ in 0..CHARGE_TIMEOUT_TICKS - 1 {
        tick(&mut app);
    }
    enqueue(&mut app, scenario.release())?;
    tick(&mut app);
    check_state(&mut app, scenario, 40.0, 90.0, 1, false)?;
    info!("A release on the timeout tick was ignored; timeout takes precedence");

    // Commands may outlive their producer, for example after a unit dies.
    if !app.world_mut().despawn(scenario.source) {
        return Err("charged actor disappeared before the despawn demonstration".into());
    }
    enqueue(&mut app, scenario.begin())?;
    enqueue(&mut app, scenario.release())?;
    tick(&mut app);
    if app.world().resource::<ChargeReport>().successful_casts != 1
        || app.world().resource::<ChargeReport>().failed_casts != 0
    {
        return Err("commands for the despawned actor unexpectedly activated an ability".into());
    }
    info!("Commands for the despawned actor were discarded without activation");
    Ok(())
}

fn make_app(logging: bool) -> ExampleResult<(App, Scenario)> {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        SimulationPlugin::default(),
        AbilityCommandPlugin,
    ))
    .init_resource::<GameCommandQueue<ChargeCommand>>()
    .init_resource::<ChargeReport>()
    .add_systems(
        FixedUpdate,
        (
            (expire_charges, dispatch_charge_commands)
                .chain()
                .before(GameplaySet::Movement)
                .before(AbilityCommandSet::Prepare),
            report_casts.after(AbilityCommandSet::Results),
        ),
    );
    if logging {
        app.add_plugins(LogPlugin::default());
    }
    app.finish();
    app.cleanup();
    let (mana, health) = app
        .world_mut()
        .run_system_once(register_attributes)
        .map_err(|error| format!("attribute registration system failed: {error:?}"))??;
    let ability = Arc::new(
        GameplayAbility::default()
            .with_cost(instant_delta(mana, -10.0))
            .with_startup_tasks(vec![AbilityTaskDef::instant(
                AbilityTaskOnFinishedDef::Batch {
                    actions: vec![
                        AbilityTaskOnFinishedDef::ApplyGameplayEffectToTarget {
                            effect: instant_delta(health, -10.0),
                        },
                        AbilityTaskOnFinishedDef::EndAbility,
                    ],
                },
            )]),
    );
    let mut actor = GameplayAbilitySystemBundle::default();
    actor.attributes.initialize_attribute(
        app.world().resource::<AttributeIdManager>(),
        mana,
        50.0,
        None,
    )?;
    let ability = actor.ability_system.give_ability(ability, 1)?;
    let mut target = GameplayAbilitySystemBundle::default();
    target.attributes.initialize_attribute(
        app.world().resource::<AttributeIdManager>(),
        health,
        100.0,
        None,
    )?;
    let source = app.world_mut().spawn((actor, ChargeSlot::default())).id();
    let target = app.world_mut().spawn(target).id();
    Ok((
        app,
        Scenario {
            source,
            target,
            ability,
            mana,
            health,
        },
    ))
}

fn register_attributes(
    mut register: AttributeIdRegister,
) -> ExampleResult<(AttributeId, AttributeId)> {
    Ok((
        register.request_or_register_attribute_id("Mana", AttributeRegion::Hot)?,
        register.request_or_register_attribute_id("Health", AttributeRegion::Hot)?,
    ))
}

fn instant_delta(attribute: AttributeId, amount: f32) -> Arc<GameplayEffect> {
    Arc::new(GameplayEffect::new(
        vec![Modifier::new(
            attribute,
            ModifierOperation::Add,
            ModifierMagnitude::Flat(amount),
        )],
        EffectDurationTicks::Instant,
        None,
        1.0,
        StackingPolicy::non_stacking(),
        EffectTags::new(Vec::new(), Vec::new()),
    ))
}

fn expire_charges(mut actors: Query<(Entity, &mut ChargeSlot)>) {
    for (source, mut slot) in &mut actors {
        let Some(charging) = slot.0.as_mut() else {
            continue;
        };
        charging.remaining_ticks = charging.remaining_ticks.saturating_sub(1);
        if charging.remaining_ticks == 0 {
            slot.0 = None;
            info!(?source, "Charging timed out without activating GAS");
        }
    }
}

fn dispatch_charge_commands(
    mut commands: ResMut<GameCommandQueue<ChargeCommand>>,
    mut abilities: ResMut<GameCommandQueue<AbilityCommand>>,
    mut actors: Query<(&mut ChargeSlot, &AbilitySystemComponent)>,
    targets: Query<()>,
) {
    while let Some((id, command)) = commands.pop() {
        let source = match command {
            ChargeCommand::Begin { source, .. }
            | ChargeCommand::Release { source }
            | ChargeCommand::Cancel { source } => source,
        };
        let Ok((mut slot, granted)) = actors.get_mut(source) else {
            debug!(
                command_id = id.get_value(),
                ?source,
                "Charge command has no live actor"
            );
            continue;
        };
        match command {
            ChargeCommand::Begin {
                ability, target, ..
            } => {
                // A repeated press does not replace the captured target or reset the timeout.
                if slot.0.is_none()
                    && granted.find_ability_spec(ability).is_some()
                    && targets.get(target).is_ok()
                {
                    slot.0 = Some(Charging {
                        ability,
                        target,
                        remaining_ticks: CHARGE_TIMEOUT_TICKS,
                    });
                }
            }
            ChargeCommand::Release { .. } => {
                // Consume the pending charge synchronously before enqueueing. A second release
                // in this same FIFO batch cannot observe the old state and pay twice.
                let Some(charging) = slot.0.take() else {
                    continue;
                };
                if targets.get(charging.target).is_err() {
                    debug!(?source, "Captured target disappeared before release");
                    continue;
                }
                if let Err(error) = abilities.push(AbilityCommand::direct(
                    source,
                    charging.ability,
                    charging.target,
                )) {
                    error!(%error, "Unable to submit the released ability command");
                }
            }
            ChargeCommand::Cancel { .. } => {
                slot.0 = None;
            }
        }
    }
}

fn report_casts(
    mut results: MessageReader<AbilityCommandResult>,
    mut report: ResMut<ChargeReport>,
) {
    for result in results.read() {
        if matches!(result.outcome, AbilityCommandOutcome::Succeeded) {
            report.successful_casts += 1;
        } else {
            report.failed_casts += 1;
        }
        info!(
            command_id = result.command_id.get_value(),
            outcome = ?result.outcome,
            "Released ability command completed"
        );
    }
}

fn enqueue(app: &mut App, command: ChargeCommand) -> ExampleResult<()> {
    app.world_mut()
        .resource_mut::<GameCommandQueue<ChargeCommand>>()
        .push(command)?;
    Ok(())
}

fn tick(app: &mut App) {
    app.world_mut().run_schedule(FixedUpdate);
}

fn read_attribute(app: &mut App, entity: Entity, attribute: AttributeId) -> ExampleResult<f32> {
    let manager = app.world().resource::<AttributeIdManager>().clone();
    app.world_mut()
        .get_mut::<AttributeSet>(entity)
        .ok_or("actor is missing its attributes")?
        .get_current_value(&manager, attribute)?
        .ok_or_else(|| "requested attribute was not initialized".into())
}

fn check_state(
    app: &mut App,
    scenario: Scenario,
    expected_mana: f32,
    expected_health: f32,
    expected_casts: usize,
    expected_charging: bool,
) -> ExampleResult<()> {
    let mana = read_attribute(app, scenario.source, scenario.mana)?;
    let health = read_attribute(app, scenario.target, scenario.health)?;
    let charging = app
        .world()
        .get::<ChargeSlot>(scenario.source)
        .ok_or("source has no charging slot")?
        .0
        .is_some();
    let report = app.world().resource::<ChargeReport>();
    if mana != expected_mana
        || health != expected_health
        || charging != expected_charging
        || report.successful_casts != expected_casts
        || report.failed_casts != 0
    {
        return Err(format!(
            "unexpected charging state: Mana={mana}, Health={health}, charging={charging}, successful={}, failed={}",
            report.successful_casts, report.failed_casts
        )
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn charging_does_not_commit_and_repeated_release_commits_once() -> ExampleResult<()> {
        let (mut app, scenario) = make_app(false)?;
        enqueue(&mut app, scenario.begin())?;
        tick(&mut app);
        check_state(&mut app, scenario, 50.0, 100.0, 0, true)?;
        enqueue(&mut app, scenario.release())?;
        enqueue(&mut app, scenario.release())?;
        tick(&mut app);
        check_state(&mut app, scenario, 40.0, 90.0, 1, false)
    }

    #[test]
    fn same_tick_cancel_prevents_a_following_release() -> ExampleResult<()> {
        let (mut app, scenario) = make_app(false)?;
        enqueue(&mut app, scenario.begin())?;
        enqueue(&mut app, scenario.cancel())?;
        enqueue(&mut app, scenario.release())?;
        tick(&mut app);
        check_state(&mut app, scenario, 50.0, 100.0, 0, false)
    }

    #[test]
    fn duplicate_begin_does_not_extend_timeout_and_timeout_precedes_release() -> ExampleResult<()> {
        let (mut app, scenario) = make_app(false)?;
        enqueue(&mut app, scenario.begin())?;
        tick(&mut app);
        enqueue(&mut app, scenario.begin())?;
        tick(&mut app);
        tick(&mut app);
        enqueue(&mut app, scenario.release())?;
        tick(&mut app);
        check_state(&mut app, scenario, 50.0, 100.0, 0, false)
    }

    #[test]
    fn despawned_source_discards_buffered_release() -> ExampleResult<()> {
        let (mut app, scenario) = make_app(false)?;
        enqueue(&mut app, scenario.begin())?;
        tick(&mut app);
        assert!(app.world_mut().despawn(scenario.source));
        enqueue(&mut app, scenario.release())?;
        tick(&mut app);
        assert_eq!(app.world().resource::<ChargeReport>().successful_casts, 0);
        assert_eq!(app.world().resource::<ChargeReport>().failed_casts, 0);
        assert_eq!(
            read_attribute(&mut app, scenario.target, scenario.health)?,
            100.0
        );
        Ok(())
    }

    #[test]
    fn despawned_target_cancels_release_without_spending() -> ExampleResult<()> {
        let (mut app, scenario) = make_app(false)?;
        enqueue(&mut app, scenario.begin())?;
        tick(&mut app);
        assert!(app.world_mut().despawn(scenario.target));
        enqueue(&mut app, scenario.release())?;
        tick(&mut app);
        assert_eq!(
            read_attribute(&mut app, scenario.source, scenario.mana)?,
            50.0
        );
        assert_eq!(app.world().resource::<ChargeReport>().successful_casts, 0);
        assert_eq!(app.world().resource::<ChargeReport>().failed_casts, 0);
        assert!(
            app.world()
                .get::<ChargeSlot>(scenario.source)
                .is_some_and(|slot| slot.0.is_none())
        );
        Ok(())
    }
}
