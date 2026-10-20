use bevy::prelude::{App, Plugin};

/// Installs the project's own gameplay after the shared simulation is configured.
///
/// This empty starting point creates no entities and loads no data. Add game-specific
/// plugins, resources, and systems here as the project takes shape.
pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, _app: &mut App) {}
}
