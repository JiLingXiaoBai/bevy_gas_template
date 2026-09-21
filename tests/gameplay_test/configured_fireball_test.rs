use bevy_gas_template::gameplay::{default_config_directory, run_headless};

#[test]
fn game_owned_fireball_produces_the_expected_gas_result() {
    let report = run_headless(&default_config_directory())
        .expect("the game's exported fireball configuration should run through GAS");
    assert_eq!(report.target_health, 320.0);
    assert_eq!(report.caster_mana, 80.0);
    assert_eq!(report.active_abilities, 0);
}

#[test]
fn missing_configuration_explains_how_to_export_game_data() {
    let missing = default_config_directory().join("nonexistent-fireball-test-data");
    let error = run_headless(&missing).expect_err("missing game data must be reported");
    let message = error.to_string();
    assert!(message.contains("config/export.ps1"));
    assert!(message.contains("--config <directory>"));
}
