use bevy::prelude::*;
use bevy_gas::{
    AbilitySpecHandle, ActiveGameplayAbility, AttributeId, AttributeIdManager, AttributeSet,
    GameplayAbilitySystemBundle,
};
use bevy_gas_template::application::GameResult;
use bevy_gas_template::config::{
    AbilityId, ConfiguredAbilities, compile_catalog, grant_ability, load_tables,
};
use bevy_gas_template::gameplay::commands::{AbilityCommand, GameCommandId, GameCommandQueue};
use bevy_gas_template::gameplay::position::LogicalPosition;
use bevy_gas_template::gameplay::targeting::{Targetable, TargetingDefinition, TargetingInput};
use std::path::Path;
use std::sync::Arc;

/// Observed state after executing the configured level-five laser.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LaserReport {
    /// The target's current Health attribute.
    pub target_health: f32,
    /// The caster's current Mana attribute.
    pub caster_mana: f32,
    /// The number of active ability entities remaining in the scenario.
    pub active_abilities: usize,
}

#[derive(Resource)]
pub(super) struct LaserScenario {
    pub caster: Entity,
    pub target: Entity,
    pub health: AttributeId,
    pub mana: AttributeId,
    pub ability: AbilitySpecHandle,
    pub targeting: Arc<TargetingDefinition>,
}

impl LaserScenario {
    pub(super) fn command(&self, origin: Vec3) -> AbilityCommand {
        AbilityCommand::targeted(
            self.caster,
            self.ability,
            TargetingInput::new(origin, Vec3::X).with_explicit_target(self.target),
            Arc::clone(&self.targeting),
        )
    }
}

pub(super) fn initialize_scenario(world: &mut World, directory: &Path) -> GameResult<()> {
    let tables = load_tables(directory).map_err(|error| {
        format!(
            "Could not load game configuration from '{}': {error}. Run config/export.ps1 in the game project to generate assets/config, or pass --config <directory>.",
            directory.display()
        )
    })?;
    let catalog = compile_catalog(&tables, world)
        .map_err(|error| format!("Could not compile the game's configuration: {error}"))?;
    let health = catalog
        .attribute("Health")
        .ok_or("sample requires Health")?;
    let mana = catalog.attribute("Mana").ok_or("sample requires Mana")?;
    let manager = world
        .get_resource::<AttributeIdManager>()
        .ok_or("missing GAS attribute registry")?;
    let mut caster_bundle = GameplayAbilitySystemBundle::default();
    let mut target_bundle = GameplayAbilitySystemBundle::default();
    for attributes in [&mut caster_bundle.attributes, &mut target_bundle.attributes] {
        attributes.initialize_attribute(manager, health, 500.0, None)?;
        attributes.initialize_attribute(manager, mana, 100.0, None)?;
    }
    let mut bindings = ConfiguredAbilities::default();
    let ability = grant_ability(
        &mut caster_bundle.ability_system,
        &mut bindings,
        &catalog,
        AbilityId(1001),
        5,
    )?;
    let targeting = Arc::clone(
        catalog
            .ability(AbilityId(1001))
            .ok_or("sample requires laser ability 1001")?
            .targeting(),
    );
    let caster = world
        .spawn((
            Name::new("Caster"),
            caster_bundle,
            bindings,
            LogicalPosition(Vec3::ZERO),
        ))
        .id();
    let target = world
        .spawn((
            Name::new("Target"),
            target_bundle,
            Targetable,
            LogicalPosition(Vec3::X * 5.0),
        ))
        .id();
    world.insert_resource(catalog);
    world.insert_resource(LaserScenario {
        caster,
        target,
        health,
        mana,
        ability,
        targeting,
    });
    Ok(())
}

pub(super) fn queue_laser(
    scenario: Res<LaserScenario>,
    positions: Query<&LogicalPosition>,
    mut commands: ResMut<GameCommandQueue<AbilityCommand>>,
) -> GameResult<GameCommandId> {
    let position = positions.get(scenario.caster)?;
    Ok(commands.push(scenario.command(position.0))?)
}
pub(super) fn read_report(
    scenario: Res<LaserScenario>,
    manager: Res<AttributeIdManager>,
    mut attributes: Query<&mut AttributeSet>,
    active: Query<&ActiveGameplayAbility>,
) -> GameResult<LaserReport> {
    collect_report(&scenario, &manager, &mut attributes, &active)
}

pub(super) fn collect_report(
    scenario: &LaserScenario,
    manager: &AttributeIdManager,
    attributes: &mut Query<&mut AttributeSet>,
    active: &Query<&ActiveGameplayAbility>,
) -> GameResult<LaserReport> {
    let target_health = attributes
        .get_mut(scenario.target)?
        .get_current_value(manager, scenario.health)?
        .ok_or("missing target Health")?;
    let caster_mana = attributes
        .get_mut(scenario.caster)?
        .get_current_value(manager, scenario.mana)?
        .ok_or("missing caster Mana")?;
    Ok(LaserReport {
        target_health,
        caster_mana,
        active_abilities: active.iter().count(),
    })
}
