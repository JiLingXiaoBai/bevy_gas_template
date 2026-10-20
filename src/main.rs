//! Launches the project's gameplay with shared simulation and platform plugins.

use bevy::app::ScheduleRunnerPlugin;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy_gas_template::application::{GameResult, SimulationPlugin, SimulationSettings};
use bevy_gas_template::game::GamePlugin;
use std::env;
use std::num::NonZeroU64;
use std::time::Duration;

fn main() -> GameResult<()> {
    let mut headless = !cfg!(feature = "windowed");
    let mut settings = SimulationSettings::default();
    let mut seed = None;
    let mut tick_limit = None;
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--headless" => headless = true,
            "--tick-hz" => {
                let rate = arguments
                    .next()
                    .ok_or("--tick-hz requires a positive rate")?;
                settings = SimulationSettings::try_from_hz(rate.parse()?)?;
            }
            "--seed" => {
                let value = arguments
                    .next()
                    .ok_or("--seed requires an unsigned integer")?;
                seed = Some(value.parse::<u64>()?);
            }
            "--ticks" => {
                let count = arguments
                    .next()
                    .ok_or("--ticks requires a positive count")?;
                tick_limit = Some(count.parse::<NonZeroU64>()?);
            }
            _ => {
                return Err(format!(
                    "Unknown argument '{argument}'. Usage: bevy_gas_template [--headless] [--tick-hz <rate>] [--seed <u64>] [--ticks <positive count>]"
                )
                .into());
            }
        }
    }
    if tick_limit.is_some() && !headless {
        return Err("--ticks requires --headless (or a build without the windowed feature)".into());
    }
    if let Some(seed) = seed {
        settings = settings.with_random_seed(seed);
    }

    let mut app = App::new();
    if headless {
        // Poll independently of simulation ticks so slow tick rates do not hit the
        // virtual clock's per-frame delta cap or delay shutdown and external input.
        let poll_interval = settings.timestep().min(Duration::from_millis(16));
        app.add_plugins((
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(poll_interval)),
            LogPlugin::default(),
        ));
    } else {
        #[cfg(feature = "windowed")]
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Game".into(),
                ..default()
            }),
            ..default()
        }));
    }
    app.add_plugins((SimulationPlugin::new(settings), GamePlugin));
    if let Some(limit) = tick_limit {
        app.insert_resource(RemainingTicks(limit.get()))
            .add_systems(FixedLast, exit_after_requested_ticks);
    }
    info!(
        headless,
        tick_hz = settings.timestep().as_secs_f64().recip(),
        seed = settings.random_seed(),
        "Starting game"
    );
    let exit = app.run();
    if exit.is_error() {
        return Err(format!("The game exited unsuccessfully: {exit:?}").into());
    }
    Ok(())
}

#[derive(Resource)]
struct RemainingTicks(u64);

fn exit_after_requested_ticks(
    mut remaining: ResMut<RemainingTicks>,
    mut exit: MessageWriter<AppExit>,
) {
    // Several fixed ticks can run in one frame; only request shutdown once.
    if remaining.0 > 0 {
        remaining.0 -= 1;
        if remaining.0 == 0 {
            exit.write(AppExit::Success);
        }
    }
}
