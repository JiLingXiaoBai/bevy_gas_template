use super::runtime_test::{app, tables as base_tables};
use bevy::ecs::system::{SystemParam, SystemParamItem, SystemState};
use bevy::prelude::*;
use bevy_gas::{
    AbilityActivationCheckError, AbilityActivationCheckParams, AbilityActivationContext,
    AbilityActivationError, AbilityCommitError, AdditionalCost, AdditionalCostContext,
    AdditionalCostError, AdditionalCostProvider, GameplayAbilitySystemBundle,
    GameplayExecutionError, GameplayExecutionOutcome, GameplayExecutionQueue,
    GameplayExecutionResult, UniqueNamePool, can_activate_ability,
};
use bevy_gas_template::application::SimulationPlugin;
use bevy_gas_template::config::generated::gas::{AbilityAdditionalCost, TbAbility, TbTargeting};
use bevy_gas_template::config::generated::{ByteBuf, Tables};
use bevy_gas_template::config::{
    AbilityId, ConfigErrorKind, ConfigLocation, ConfiguredAbilities, GameplayCatalog,
    compile_catalog, grant_ability,
};
#[cfg(feature = "config-validation")]
use bevy_gas_template::config::{describe_ability, describe_ability_at_level, validate_tables};
use std::sync::Arc;

fn location(row: &str, index: usize, field: &str) -> ConfigLocation {
    ConfigLocation::Table {
        table: "Ability",
        row: Some(row.to_owned()),
        field: Some(format!("additional_costs[{index}].{field}")),
    }
}

fn cost(resource: i32, amount: i64) -> AbilityAdditionalCost {
    AbilityAdditionalCost { resource, amount }
}

fn tables(rows: Vec<AbilityAdditionalCost>) -> Tables {
    let mut tables = base_tables(None);
    let mut ability = tables.tb_ability.get(&1).unwrap().as_ref().clone();
    ability.additional_costs = rows;
    tables.tb_ability = Arc::new(TbAbility::from_rows(vec![ability]).unwrap());
    tables
}

#[test]
fn generated_ability_decodes_nested_integer_resource_ids_and_long_amounts() {
    // One ability, with four nested costs covering the full signed resource-ID range.
    let mut bytes = vec![1, 1, 1, b'A', 2, 0, 4];
    // i32::MIN costs one unit, -1 costs two units, and 0 costs three units.
    bytes.extend_from_slice(&[0xf0, 0x80, 0, 0, 0, 1]);
    bytes.extend_from_slice(&[0xf0, 0xff, 0xff, 0xff, 0xff, 2]);
    bytes.extend_from_slice(&[0, 3]);
    // i32::MAX costs u32::MAX units; the resource uses int and the amount uses long.
    bytes.extend_from_slice(&[0xf0, 0x7f, 0xff, 0xff, 0xff]);
    bytes.extend_from_slice(&[0xf0, 0xff, 0xff, 0xff, 0xff]);
    // No cooldown or tags; self targeting and one terminal task.
    let suffix = [0, 0, 0, 0, 0, 0, 1, 0, 1, 1];
    bytes.extend_from_slice(&suffix);
    let mut empty = vec![1, 1, 1, b'A', 2, 0, 0];
    empty.extend_from_slice(&suffix);

    for (encoded, expected) in [
        (&empty, vec![]),
        (
            &bytes,
            vec![(i32::MIN, 1), (-1, 2), (0, 3), (i32::MAX, u32::MAX)],
        ),
    ] {
        let decoded = Tables::new(|name| {
            Ok(ByteBuf::new(if name == "gas_tbability" {
                encoded.clone()
            } else {
                vec![0]
            }))
        })
        .unwrap();
        let ability = decoded.tb_ability.get(&1).unwrap();
        assert_eq!(ability.additional_costs.len(), expected.len());
        for (row, &(resource, amount)) in ability.additional_costs.iter().zip(&expected) {
            assert_eq!(row.resource, resource);
            assert_eq!(row.amount, i64::from(amount));
        }
        let mut tables = base_tables(None);
        tables.tb_ability = decoded.tb_ability;
        let mut app = app();
        let catalog = compile_catalog(&tables, app.world_mut()).unwrap();
        let requirements = catalog
            .ability(AbilityId(1))
            .unwrap()
            .definition()
            .get_additional_costs();
        assert_eq!(requirements.len(), expected.len());
        for (requirement, &(resource, amount)) in requirements.iter().zip(&expected) {
            assert_eq!(requirement.resource(), resource);
            assert_eq!(requirement.amount(), amount);
        }
    }

    for malformed in (0..bytes.len())
        .map(|length| bytes[..length].to_vec())
        .chain([[bytes.clone(), vec![0]].concat()])
    {
        assert!(
            Tables::new(|name| {
                Ok(ByteBuf::new(if name == "gas_tbability" {
                    malformed.clone()
                } else {
                    vec![0]
                }))
            })
            .is_err()
        );
    }
}

#[test]
fn integer_resources_and_cost_order_are_independent_of_ability_rows_and_name_pools() {
    let mut tables = tables(vec![cost(10, 3), cost(10, 2), cost(2, 1)]);
    let first = tables.tb_ability.get(&1).unwrap().as_ref().clone();
    let mut second = first.clone();
    second.id = 2;
    second.additional_costs = vec![cost(-7, 4)];
    let mut compiled = Vec::new();
    for (reverse, extra_names) in [(false, 0), (false, 3), (true, 0), (true, 3)] {
        let mut app = app();
        for index in 0..extra_names {
            app.world_mut()
                .resource_mut::<UniqueNamePool>()
                .new_name(&format!("Unrelated{index}"))
                .unwrap();
        }
        let mut expected_names = app.world().resource::<UniqueNamePool>().clone();
        let mut rows = vec![first.clone(), second.clone()];
        if reverse {
            rows.reverse();
        }
        tables.tb_ability = Arc::new(TbAbility::from_rows(rows).unwrap());
        let catalog = compile_catalog(&tables, app.world_mut()).unwrap();
        let requirements = catalog
            .ability(AbilityId(1))
            .unwrap()
            .definition()
            .get_additional_costs();
        assert_eq!(
            requirements,
            [
                AdditionalCost::new(10, 3).unwrap(),
                AdditionalCost::new(10, 2).unwrap(),
                AdditionalCost::new(2, 1).unwrap(),
            ]
        );
        assert_eq!(
            catalog
                .ability(AbilityId(2))
                .unwrap()
                .definition()
                .get_additional_costs(),
            [AdditionalCost::new(-7, 4).unwrap()]
        );
        // The fixture has no tags or attributes, so costs must not add pool entries.
        assert_eq!(
            app.world_mut()
                .resource_mut::<UniqueNamePool>()
                .new_name("AfterCompilation")
                .unwrap(),
            expected_names.new_name("AfterCompilation").unwrap()
        );
        compiled.push(requirements.to_vec());
    }
    assert!(compiled.windows(2).all(|pair| pair[0] == pair[1]));
}

#[test]
fn malformed_costs_are_rejected_with_field_locations_in_every_feature_mode() {
    let mut cases = Vec::new();
    for amount in [i64::MIN, -1, 0, i64::from(u32::MAX) + 1, i64::MAX] {
        cases.push((vec![cost(5001, amount)], 0, "amount"));
    }
    cases.push((
        vec![cost(5001, i64::from(u32::MAX)), cost(5001, 1)],
        1,
        "amount",
    ));
    for (rows, index, field) in cases {
        let mut tables = tables(rows);
        let mut ability = tables.tb_ability.get(&1).unwrap().as_ref().clone();
        ability.id = 7;
        tables.tb_ability = Arc::new(TbAbility::from_rows(vec![ability]).unwrap());
        let error = compile_catalog(&tables, app().world_mut()).err().unwrap();
        assert_eq!(error.kind(), ConfigErrorKind::InvalidValue);
        assert_eq!(error.location(), &location("7", index, field));
    }
}

#[test]
fn failed_compilation_preserves_name_pool_and_existing_catalog() {
    let mut app = app();
    let original = compile_catalog(&base_tables(None), app.world_mut()).unwrap();
    let original_ability = Arc::clone(original.ability(AbilityId(1)).unwrap().definition());
    app.world_mut().insert_resource(original);
    let names_before = app.world().resource::<UniqueNamePool>().clone();
    let mut invalid = tables(vec![cost(5001, 1)]);
    invalid.tb_targeting = Arc::new(TbTargeting::from_rows(vec![]).unwrap());
    assert!(compile_catalog(&invalid, app.world_mut()).is_err());

    let mut expected_names = names_before.clone();
    let mut actual_names = app.world().resource::<UniqueNamePool>().clone();
    assert_eq!(
        actual_names.new_name("AfterFailure").unwrap(),
        expected_names.new_name("AfterFailure").unwrap()
    );
    assert!(Arc::ptr_eq(
        app.world()
            .resource::<GameplayCatalog>()
            .ability(AbilityId(1))
            .unwrap()
            .definition(),
        &original_ability
    ));

    let repaired = compile_catalog(&tables(vec![cost(5001, 1)]), app.world_mut()).unwrap();
    assert_eq!(
        repaired
            .ability(AbilityId(1))
            .unwrap()
            .definition()
            .get_additional_costs(),
        [AdditionalCost::new(5001, 1).unwrap()]
    );
}

#[test]
fn abilities_without_inline_costs_keep_an_empty_requirement_list() {
    let catalog = compile_catalog(&base_tables(None), app().world_mut()).unwrap();
    assert!(
        catalog
            .ability(AbilityId(1))
            .unwrap()
            .definition()
            .get_additional_costs()
            .is_empty()
    );
}

#[cfg(feature = "config-validation")]
#[test]
fn authoring_validation_accepts_the_full_i32_resource_id_range() {
    for resource in [i32::MIN, -1, 0, 1, i32::MAX] {
        validate_tables(&tables(vec![cost(resource, 1)])).unwrap();
    }
}

#[cfg(feature = "config-validation")]
#[test]
fn inspection_displays_ordered_costs_with_constant_amounts_at_each_level() {
    let rows = vec![cost(5002, 3), cost(5001, 2), cost(-1, 1)];
    let report = describe_ability(&tables(rows.clone()), AbilityId(1)).unwrap();
    let cost_lines: Vec<_> = report
        .lines()
        .filter(|line| line.starts_with("Additional cost"))
        .collect();
    assert_eq!(
        cost_lines,
        [
            "Additional cost / order 0: resource=5002, amount=3",
            "Additional cost / order 1: resource=5001, amount=2",
            "Additional cost / order 2: resource=-1, amount=1",
        ]
    );
    let level_two = describe_ability_at_level(&tables(rows), AbilityId(1), 2).unwrap();
    assert_eq!(
        cost_lines,
        level_two
            .lines()
            .filter(|line| line.starts_with("Additional cost"))
            .collect::<Vec<_>>()
    );
}

#[derive(Component)]
struct Inventory {
    resource: i32,
    bombs: u32,
}

impl Inventory {
    fn required(&self, costs: &[AdditionalCost]) -> Result<u32, AdditionalCostError> {
        let mut required = 0_u32;
        for cost in costs {
            if cost.resource() != self.resource {
                return Err(AdditionalCostError::UnsupportedResource {
                    resource: cost.resource(),
                });
            }
            required = required.checked_add(cost.amount()).unwrap();
        }
        if required > self.bombs {
            return Err(AdditionalCostError::InsufficientResource {
                resource: self.resource,
                required,
                available: self.bombs,
            });
        }
        Ok(required)
    }
}

#[derive(SystemParam)]
struct InventoryProvider<'w, 's> {
    inventories: Query<'w, 's, &'static mut Inventory>,
}

impl AdditionalCostProvider for InventoryProvider<'static, 'static> {
    type ReadOnly = Query<'static, 'static, &'static Inventory>;
    type Receipt = u32;

    fn check(
        params: &SystemParamItem<'_, '_, Self>,
        context: &AdditionalCostContext<'_>,
        costs: &[AdditionalCost],
    ) -> Result<(), AdditionalCostError> {
        params
            .inventories
            .get(context.source)
            .unwrap()
            .required(costs)
            .map(|_| ())
    }

    fn check_readonly(
        params: &SystemParamItem<'_, '_, Self::ReadOnly>,
        context: &AdditionalCostContext<'_>,
        costs: &[AdditionalCost],
    ) -> Result<(), AdditionalCostError> {
        params
            .get(context.source)
            .unwrap()
            .required(costs)
            .map(|_| ())
    }

    fn prepare(
        params: &mut SystemParamItem<'_, '_, Self>,
        context: &AdditionalCostContext<'_>,
        costs: &[AdditionalCost],
    ) -> Result<Self::Receipt, AdditionalCostError> {
        let mut inventory = params.inventories.get_mut(context.source).unwrap();
        let required = inventory.required(costs)?;
        inventory.bombs -= required;
        Ok(required)
    }

    fn rollback(
        params: &mut SystemParamItem<'_, '_, Self>,
        context: &AdditionalCostContext<'_>,
        receipt: Self::Receipt,
    ) -> Result<(), AdditionalCostError> {
        params.inventories.get_mut(context.source).unwrap().bombs += receipt;
        Ok(())
    }
}

#[test]
fn configured_duplicate_costs_reach_the_inventory_provider_without_deduplication() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(SimulationPlugin::default().with_additional_costs::<InventoryProvider>());
    let mut tables = tables(vec![cost(5001, 1), cost(5001, 1), cost(5001, 1)]);
    let mut row = tables.tb_ability.get(&1).unwrap().as_ref().clone();
    row.allow_multiple_instances = true;
    tables.tb_ability = Arc::new(TbAbility::from_rows(vec![row]).unwrap());
    let catalog = compile_catalog(&tables, app.world_mut()).unwrap();
    let ability = Arc::clone(catalog.ability(AbilityId(1)).unwrap().definition());
    let mut bundle = GameplayAbilitySystemBundle::default();
    let mut bindings = ConfiguredAbilities::default();
    let handle = grant_ability(
        &mut bundle.ability_system,
        &mut bindings,
        &catalog,
        AbilityId(1),
        2,
    )
    .unwrap();
    let source = app
        .world_mut()
        .spawn((
            bundle,
            bindings,
            Inventory {
                resource: 5001,
                bombs: 3,
            },
        ))
        .id();
    let mut preview =
        SystemState::<AbilityActivationCheckParams<InventoryProvider>>::new(app.world_mut());
    can_activate_ability(
        source,
        source,
        &ability,
        2,
        &preview.get(app.world()).unwrap(),
    )
    .unwrap();
    assert_eq!(app.world().get::<Inventory>(source).unwrap().bombs, 3);

    // The catalog stores requirements; the selected runtime provider supplies payment behavior.
    let mut no_provider = SystemState::<AbilityActivationCheckParams>::new(app.world_mut());
    assert_eq!(
        can_activate_ability(
            source,
            source,
            &ability,
            2,
            &no_provider.get(app.world()).unwrap()
        ),
        Err(AbilityActivationCheckError::Cost(
            AbilityCommitError::AdditionalCost(AdditionalCostError::MissingProvider)
        ))
    );
    {
        let mut queue = app.world_mut().resource_mut::<GameplayExecutionQueue>();
        for _ in 0..2 {
            let context = AbilityActivationContext::direct(source, queue.new_root_chain(handle));
            queue
                .push_activation(source, source, handle, context)
                .unwrap();
        }
    }
    app.world_mut().run_schedule(FixedUpdate);
    assert_eq!(app.world().get::<Inventory>(source).unwrap().bombs, 0);
    let results = app.world().resource::<Messages<GameplayExecutionResult>>();
    let mut cursor = results.get_cursor();
    let outcomes: Vec<_> = cursor.read(results).map(|result| &result.outcome).collect();
    assert_eq!(outcomes.len(), 2);
    assert!(matches!(outcomes[0], GameplayExecutionOutcome::Succeeded));
    assert_eq!(
        outcomes[1],
        &GameplayExecutionOutcome::Rejected(GameplayExecutionError::AbilityActivation(
            AbilityActivationError::CommitPreparationFailed {
                source,
                handle,
                error: AbilityCommitError::AdditionalCost(
                    AdditionalCostError::InsufficientResource {
                        resource: 5001,
                        required: 3,
                        available: 0,
                    }
                ),
            }
        ))
    );
    assert_eq!(
        can_activate_ability(
            source,
            source,
            &ability,
            2,
            &preview.get(app.world()).unwrap()
        ),
        Err(AbilityActivationCheckError::Cost(
            AbilityCommitError::AdditionalCost(AdditionalCostError::InsufficientResource {
                resource: 5001,
                required: 3,
                available: 0
            })
        ))
    );
}

#[test]
fn resource_totals_and_order_positions_are_scoped_to_each_ability() {
    let mut tables = tables(vec![cost(5001, i64::from(u32::MAX))]);
    let first = tables.tb_ability.get(&1).unwrap().as_ref().clone();
    let mut second = first.clone();
    second.id = 2;
    tables.tb_ability = Arc::new(TbAbility::from_rows(vec![first, second]).unwrap());
    let catalog = compile_catalog(&tables, app().world_mut()).unwrap();
    let first_costs = catalog
        .ability(AbilityId(1))
        .unwrap()
        .definition()
        .get_additional_costs();
    let second_costs = catalog
        .ability(AbilityId(2))
        .unwrap()
        .definition()
        .get_additional_costs();
    assert_eq!(first_costs.len(), 1);
    assert_eq!(first_costs[0].amount(), u32::MAX);
    assert_eq!(first_costs, second_costs);
}
