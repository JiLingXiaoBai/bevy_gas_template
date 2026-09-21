use super::scenario::{
    CastInput, FireballReport, FireballScenario, GameResult, collect_report, initialize_scenario,
    queue_fireball, read_report,
};
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy::text::FontSize;
use bevy_gas::{
    ActiveGameplayAbility, AttributeIdManager, AttributeSet, GameplayAbilitySystemPlugin,
    GameplayAbilitySystemSet, GameplayExecutionOutcome, GameplayExecutionResult,
};
use std::path::{Path, PathBuf};

/// Returns the game's default binary configuration directory.
///
/// The absolute path is based on this package's manifest directory, so launching
/// the executable from another working directory still loads the game's data.
pub fn default_config_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/config")
}

/// Executes the game-owned fireball configuration without opening a window.
///
/// `directory` contains the Luban binary tables. This drives activation tick zero
/// and twelve subsequent fixed ticks, returning the observed attributes and active
/// ability count. Loading, compilation, and unexpected gameplay results return an error.
pub fn run_headless(directory: &Path) -> GameResult<FireballReport> {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, GameplayAbilitySystemPlugin))
        .insert_resource(CastInput(true))
        .add_systems(
            FixedUpdate,
            queue_fireball.in_set(GameplayAbilitySystemSet::RequestProducers),
        );
    initialize_scenario(app.world_mut(), directory)?;
    app.finish();
    app.cleanup();
    for _ in 0..=12 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    let report = app
        .world_mut()
        .run_system_once(read_report)
        .map_err(|error| format!("Could not read the fireball result: {error:?}"))??;
    if report.target_health != 320.0 || report.caster_mana != 80.0 || report.active_abilities != 0 {
        return Err(format!("Configured fireball smoke test failed: {report:?}; expected target Health=320, caster Mana=80, active abilities=0").into());
    }
    Ok(report)
}

/// Runs the interactive fireball sample until its window closes or Escape is pressed.
///
/// `directory` supplies the game's Luban binary tables. Space requests the configured
/// level-five fireball through GAS targeting and execution queues. Initialization and
/// unsuccessful application exits return an error; normal shutdown returns `Ok(())`.
pub fn run_windowed(directory: &Path) -> GameResult<()> {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Game-owned configuration | GAS Fireball".into(),
            resolution: (960, 640).into(),
            ..default()
        }),
        ..default()
    }))
    .add_plugins(GameplayAbilitySystemPlugin)
    .insert_resource(ClearColor(Color::srgb(0.025, 0.035, 0.06)))
    .insert_resource(Time::<Fixed>::from_hz(20.0))
    .init_resource::<CastInput>()
    .init_resource::<CastStatus>()
    .add_systems(Startup, setup_visuals)
    .add_systems(
        FixedUpdate,
        queue_fireball.in_set(GameplayAbilitySystemSet::RequestProducers),
    )
    .add_systems(
        Update,
        (read_input, read_execution_results, update_hud).chain(),
    );
    initialize_scenario(app.world_mut(), directory)?;
    info!(configuration = %directory.display(), "Loaded game-owned fireball configuration");
    let exit = app.run();
    if exit.is_error() {
        return Err(format!("The game exited unsuccessfully: {exit:?}").into());
    }
    Ok(())
}

#[derive(Component)]
struct StatusText;

#[derive(Component)]
struct CastIndicator;

#[derive(Resource)]
struct CastStatus(String);

impl Default for CastStatus {
    fn default() -> Self {
        Self("Ready. Press Space to cast the configured fireball.".into())
    }
}

fn setup_visuals(mut commands: Commands, scenario: Res<FireballScenario>) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scale: 0.012,
            ..OrthographicProjection::default_2d()
        }),
        Transform::from_xyz(2.5, 0.0, 0.0),
    ));
    commands.entity(scenario.caster).insert(Sprite::from_color(
        Color::srgb(0.15, 0.65, 1.0),
        Vec2::splat(0.8),
    ));
    commands.entity(scenario.target).insert(Sprite::from_color(
        Color::srgb(0.9, 0.24, 0.3),
        Vec2::splat(0.8),
    ));
    commands.spawn((
        Sprite::from_color(Color::srgb(1.0, 0.55, 0.12), Vec2::new(4.0, 0.06)),
        Transform::from_xyz(2.5, 0.0, -0.1),
        Visibility::Hidden,
        CastIndicator,
    ));
    commands.spawn((
        Text::new("Loading gameplay state..."),
        TextFont {
            font_size: FontSize::Px(21.0),
            ..default()
        },
        TextColor(Color::srgb(0.85, 0.9, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: px(28),
            left: px(32),
            right: px(32),
            ..default()
        },
        StatusText,
    ));
    commands.spawn((
        Text::new("BLUE: CASTER                         RED: TARGET\n\nSpace: cast     Esc: quit\nEach accepted fireball spends 20 Mana. Wait for cooldown before casting again."),
        TextFont {
            font_size: FontSize::Px(17.0),
            ..default()
        },
        TextColor(Color::srgb(0.55, 0.65, 0.78)),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(32),
            left: px(32),
            ..default()
        },
    ));
}

fn read_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut input: ResMut<CastInput>,
    mut status: ResMut<CastStatus>,
    mut exit: MessageWriter<AppExit>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        input.0 = true;
        status.0 = "Fireball requested; waiting for the next fixed tick.".into();
    }
    if keyboard.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
}

fn read_execution_results(
    mut results: MessageReader<GameplayExecutionResult>,
    mut status: ResMut<CastStatus>,
) {
    for result in results.read() {
        match &result.outcome {
            GameplayExecutionOutcome::Succeeded => {
                status.0 =
                    "Fireball accepted. Its configured tasks control damage and completion.".into();
            }
            GameplayExecutionOutcome::Rejected(error) => {
                status.0 = format!("Cast rejected: {error}");
            }
            GameplayExecutionOutcome::Failed(error) => {
                status.0 = format!("Gameplay execution failed: {error}");
                error!(%error, "Gameplay execution failed");
            }
        }
    }
}

fn update_hud(
    scenario: Res<FireballScenario>,
    manager: Res<AttributeIdManager>,
    mut attributes: Query<&mut AttributeSet>,
    active: Query<&ActiveGameplayAbility>,
    status: Res<CastStatus>,
    mut hud: Query<&mut Text, With<StatusText>>,
    mut indicator: Query<&mut Visibility, With<CastIndicator>>,
) {
    let display = match collect_report(&scenario, &manager, &mut attributes, &active) {
        Ok(report) => format!(
            "FIREBALL / LEVEL 5\n\nCaster Mana: {:.0}       Target Health: {:.0}\nActive abilities: {}       Fixed ticks: 20/s\n\n{}",
            report.caster_mana, report.target_health, report.active_abilities, status.0
        ),
        Err(error) => format!("Could not read gameplay state: {error}"),
    };
    for mut text in &mut hud {
        if text.0 != display {
            text.0.clone_from(&display);
        }
    }
    for mut visibility in &mut indicator {
        *visibility = if active.is_empty() {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    }
}
