use crate::config::{AbilityId, ConfiguredAbilities, compile_catalog, grant_ability, load_tables};
use bevy::prelude::*;
use bevy_gas::{
    AbilityActivationContext, AbilitySpecHandle, ActiveGameplayAbility, AttributeId,
    AttributeIdManager, AttributeSet, GameplayAbilitySystemBundle, GameplayExecutionQueue,
    Targetable, TargetingContinuation, TargetingDefinition, TargetingInput, TargetingRequestQueue,
};
use std::error::Error;
use std::path::Path;
use std::sync::Arc;

/// The result of game initialization or deterministic gameplay verification.
///
/// Successful calls return their requested value; errors preserve configuration,
/// attribute, or runtime failures with contextual startup information.
pub type GameResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

/// Observed state after executing the configured level-five fireball.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FireballReport {
    /// The target's current Health attribute.
    pub target_health: f32,
    /// The caster's current Mana attribute.
    pub caster_mana: f32,
    /// The number of active ability entities remaining in the scenario.
    pub active_abilities: usize,
}

#[derive(Resource)]
pub(super) struct FireballScenario {
    pub caster: Entity,
    pub target: Entity,
    pub health: AttributeId,
    pub mana: AttributeId,
    pub ability: AbilitySpecHandle,
    pub targeting: Arc<TargetingDefinition>,
}

#[derive(Resource, Default)]
pub(super) struct CastInput(pub bool);

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
            .ok_or("sample requires fireball ability 1001")?
            .targeting(),
    );
    let caster = world
        .spawn((
            Name::new("Caster"),
            caster_bundle,
            bindings,
            Transform::IDENTITY,
            GlobalTransform::IDENTITY,
        ))
        .id();
    let target = world
        .spawn((
            Name::new("Target"),
            target_bundle,
            Targetable,
            Transform::from_translation(Vec3::X * 5.0),
            GlobalTransform::from_translation(Vec3::X * 5.0),
        ))
        .id();
    world.insert_resource(catalog);
    world.insert_resource(FireballScenario {
        caster,
        target,
        health,
        mana,
        ability,
        targeting,
    });
    Ok(())
}

pub(super) fn queue_fireball(
    mut input: ResMut<CastInput>,
    scenario: Res<FireballScenario>,
    mut execution: ResMut<GameplayExecutionQueue>,
    mut targeting: ResMut<TargetingRequestQueue>,
) {
    if !std::mem::take(&mut input.0) {
        return;
    }
    let chain = execution.new_root_chain(scenario.ability);
    targeting.push_request(
        scenario.caster,
        TargetingInput::new(Vec3::ZERO, Vec3::X).with_explicit_target(scenario.target),
        Arc::clone(&scenario.targeting),
        TargetingContinuation::activate_ability(
            scenario.ability,
            AbilityActivationContext::direct(scenario.caster, chain),
        ),
    );
}

pub(super) fn read_report(
    scenario: Res<FireballScenario>,
    manager: Res<AttributeIdManager>,
    mut attributes: Query<&mut AttributeSet>,
    active: Query<&ActiveGameplayAbility>,
) -> GameResult<FireballReport> {
    collect_report(&scenario, &manager, &mut attributes, &active)
}

pub(super) fn collect_report(
    scenario: &FireballScenario,
    manager: &AttributeIdManager,
    attributes: &mut Query<&mut AttributeSet>,
    active: &Query<&ActiveGameplayAbility>,
) -> GameResult<FireballReport> {
    let target_health = attributes
        .get_mut(scenario.target)?
        .get_current_value(manager, scenario.health)?
        .ok_or("missing target Health")?;
    let caster_mana = attributes
        .get_mut(scenario.caster)?
        .get_current_value(manager, scenario.mana)?
        .ok_or("missing caster Mana")?;
    Ok(FireballReport {
        target_health,
        caster_mana,
        active_abilities: active.iter().count(),
    })
}
