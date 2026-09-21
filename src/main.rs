//! Launches the independent game or verifies its configured fireball without a window.

use bevy::log::{LogPlugin, info};
use bevy::prelude::*;
use bevy_gas_template::gameplay::{
    GameResult, default_config_directory, run_headless, run_windowed,
};
use std::env;
use std::path::PathBuf;

fn main() -> GameResult<()> {
    let mut headless = false;
    let mut directory = default_config_directory();
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--headless" => headless = true,
            "--config" => {
                directory = PathBuf::from(arguments.next().ok_or("--config requires a directory")?);
            }
            _ => {
                return Err(format!(
                    "Unknown argument '{argument}'. Usage: bevy_gas_template [--headless] [--config <directory>]"
                )
                .into());
            }
        }
    }
    if headless {
        // The library entry remains subscriber-free so tests can call it repeatedly.
        App::new().add_plugins(LogPlugin::default());
        let report = run_headless(&directory)?;
        info!(
            "Fireball level 5 after 12 ticks: target Health={}, caster Mana={}, active abilities={}",
            report.target_health, report.caster_mana, report.active_abilities
        );
        Ok(())
    } else {
        run_windowed(&directory)
    }
}
