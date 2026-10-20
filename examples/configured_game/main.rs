//! Launches the configured game example or verifies its laser without a window.

mod demo;

use bevy::log::{LogPlugin, info};
use bevy::prelude::*;
use bevy_gas_template::application::{GameResult, default_config_directory};
use demo::{run_headless, run_windowed};
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
                    "Unknown argument '{argument}'. Usage: configured_game [--headless] [--config <directory>]"
                )
                .into());
            }
        }
    }
    if headless {
        // The demo entry remains subscriber-free so tests can call it repeatedly.
        App::new().add_plugins(LogPlugin::default());
        let report = run_headless(&directory)?;
        info!(
            "Laser level 5 after 12 ticks: target Health={}, caster Mana={}, active abilities={}",
            report.target_health, report.caster_mana, report.active_abilities
        );
        Ok(())
    } else {
        run_windowed(&directory)
    }
}
