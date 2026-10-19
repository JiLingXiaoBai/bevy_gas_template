//! Game-owned spatial sampling, independent of GAS effect probability draws.
//!
//! Initialize [`SpatialRandom`] explicitly with `App::init_resource` or insert a seeded instance.
//! This module does not install a plugin or consume the GAS [`bevy_gas::Random`] resource.

mod spatial_random;

pub use spatial_random::SpatialRandom;
