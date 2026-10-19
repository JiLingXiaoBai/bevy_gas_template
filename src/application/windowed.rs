use super::GameResult;
use super::scenario::{CastInput, LaserScenario, collect_report, initialize_scenario, queue_laser};
use crate::gameplay::position::{PositionPresentation, PositionPresentationPlugin};
use crate::gameplay::targeting::TargetingPlugin;
use bevy::prelude::*;
use bevy::text::FontSize;
use bevy_gas::{
    ActiveGameplayAbility, AttributeIdManager, AttributeSet, GameplayAbilitySystemPlugin,
    GameplayAbilitySystemSet, GameplayExecutionOutcome, GameplayExecutionResult,
};
use std::path::Path;

/// Runs the interactive laser sample until its window closes or Escape is pressed.
///
/// `directory` supplies the game's Luban binary tables. Space requests the configured
/// level-five laser through game-owned targeting and the GAS execution queue. Initialization and
/// unsuccessful application exits return an error; normal shutdown returns `Ok(())`.
/// Requires the `windowed` feature; builds without it expose an entry that returns an error.
pub fn run_windowed(directory: &Path) -> GameResult<()> {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Game-owned configuration | GAS Laser".into(),
            resolution: (960, 640).into(),
            ..default()
        }),
        ..default()
    }))
    .add_plugins((
        GameplayAbilitySystemPlugin,
        TargetingPlugin,
        PositionPresentationPlugin,
    ))
    .insert_resource(ClearColor(Color::srgb(0.025, 0.035, 0.06)))
    .insert_resource(Time::<Fixed>::from_hz(20.0))
    .init_resource::<CastInput>()
    .init_resource::<CastStatus>()
    .add_systems(Startup, setup_visuals)
    .add_systems(
        FixedUpdate,
        queue_laser.in_set(GameplayAbilitySystemSet::RequestProducers),
    )
    .add_systems(
        Update,
        (read_input, read_execution_results, update_hud).chain(),
    );
    initialize_scenario(app.world_mut(), directory)?;
    info!(configuration = %directory.display(), "Loaded game-owned laser configuration");
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
        Self("Ready. Press Space to cast the configured laser.".into())
    }
}

fn setup_visuals(mut commands: Commands, scenario: Res<LaserScenario>) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scale: 0.012,
            ..OrthographicProjection::default_2d()
        }),
        Transform::from_xyz(2.5, 0.0, 0.0),
    ));
    commands.entity(scenario.caster).insert((
        Sprite::from_color(Color::srgb(0.15, 0.65, 1.0), Vec2::splat(0.8)),
        PositionPresentation::interpolated(),
    ));
    commands.entity(scenario.target).insert((
        Sprite::from_color(Color::srgb(0.9, 0.24, 0.3), Vec2::splat(0.8)),
        PositionPresentation::interpolated(),
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
        Text::new("BLUE: CASTER                         RED: TARGET\n\nSpace: cast     Esc: quit\nEach accepted laser spends 20 Mana. Wait for cooldown before casting again."),
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
        status.0 = "Laser requested; waiting for the next fixed tick.".into();
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
                    "Laser accepted. Its configured tasks control damage and completion.".into();
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
    scenario: Res<LaserScenario>,
    manager: Res<AttributeIdManager>,
    mut attributes: Query<&mut AttributeSet>,
    active: Query<&ActiveGameplayAbility>,
    status: Res<CastStatus>,
    mut hud: Query<&mut Text, With<StatusText>>,
    mut indicator: Query<&mut Visibility, With<CastIndicator>>,
) {
    let display = match collect_report(&scenario, &manager, &mut attributes, &active) {
        Ok(report) => format!(
            "LASER / LEVEL 5\n\nCaster Mana: {:.0}       Target Health: {:.0}\nActive abilities: {}       Fixed ticks: 20/s\n\n{}",
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
