use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy_gas::{
    AbilitySpecHandle, AbilitySystemComponent, AttributeId, AttributeIdManager,
    AttributeIdRegister, AttributeRegion, AttributeSet, EffectTags, GameplayAbility,
    GameplayAbilitySystemPlugin, GameplayTag, GameplayTagContainer, GameplayTagManager,
    GameplayTagRegister, Modifier, ModifierMagnitude, ModifierOperation,
};
use bevy_gas_template::gameplay::targeting::TargetingPlugin;
use std::sync::Arc;

pub fn test_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, GameplayAbilitySystemPlugin, TargetingPlugin));
    app
}

pub fn empty_effect_tags() -> EffectTags {
    EffectTags::new(Vec::new(), Vec::new())
}

pub fn add_modifier(attribute: AttributeId, value: f32) -> Modifier {
    Modifier::new(
        attribute,
        ModifierOperation::Add,
        ModifierMagnitude::Flat(value),
    )
}

pub fn register_tag(app: &mut App, name: &str) -> GameplayTag {
    let name = name.to_string();
    app.world_mut()
        .run_system_once(move |mut register: GameplayTagRegister| {
            register.request_or_register_tag(&name).unwrap()
        })
        .unwrap()
}

pub fn register_attribute(app: &mut App, name: &str) -> AttributeId {
    let name = name.to_string();
    app.world_mut()
        .run_system_once(move |mut register: AttributeIdRegister| {
            register
                .request_or_register_attribute_id(&name, AttributeRegion::Cold)
                .unwrap()
        })
        .unwrap()
}

pub fn attribute_set(app: &App, attribute: AttributeId, base_value: f32) -> AttributeSet {
    let mut attributes = AttributeSet::default();
    attributes
        .initialize_attribute(
            app.world().resource::<AttributeIdManager>(),
            attribute,
            base_value,
            None,
        )
        .unwrap();
    attributes
}

pub fn add_tag_to_entity(app: &mut App, entity: Entity, tag: GameplayTag) {
    app.world_mut()
        .run_system_once(
            move |mut query: Query<&mut GameplayTagContainer>,
                  tag_manager: Res<GameplayTagManager>| {
                query
                    .get_mut(entity)
                    .unwrap()
                    .add_tag(&tag, &tag_manager)
                    .unwrap();
            },
        )
        .unwrap();
}

pub fn remove_tag_from_entity(app: &mut App, entity: Entity, tag: GameplayTag) {
    app.world_mut()
        .run_system_once(
            move |mut query: Query<&mut GameplayTagContainer>,
                  tag_manager: Res<GameplayTagManager>| {
                query
                    .get_mut(entity)
                    .unwrap()
                    .remove_tag(&tag, &tag_manager)
                    .unwrap();
            },
        )
        .unwrap();
}

pub fn current_value(app: &mut App, entity: Entity, attribute: AttributeId) -> f32 {
    let manager = app.world().resource::<AttributeIdManager>().clone();
    app.world_mut()
        .entity_mut(entity)
        .get_mut::<AttributeSet>()
        .unwrap()
        .get_current_value(&manager, attribute)
        .unwrap()
        .unwrap()
}

pub fn give_ability(
    app: &mut App,
    owner: Entity,
    ability: Arc<GameplayAbility>,
) -> AbilitySpecHandle {
    app.world_mut()
        .entity_mut(owner)
        .get_mut::<AbilitySystemComponent>()
        .unwrap()
        .give_ability(ability, 1)
        .unwrap()
}

pub fn run_fixed_update(app: &mut App) {
    app.world_mut().run_schedule(FixedUpdate);
}
