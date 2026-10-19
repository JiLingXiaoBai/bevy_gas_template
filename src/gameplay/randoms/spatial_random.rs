use bevy::math::FromRng;
use bevy::prelude::Resource;
use bevy_shape::ShapeSample;
use rand::{
    SeedableRng,
    distr::{Distribution, StandardUniform},
    rngs::StdRng,
};

/// Seedable spatial sampling stream owned by the game, separate from GAS effect randomness.
///
/// Reproducibility requires identical dependency versions, seed, and sampling order. Explicitly
/// order systems that share this resource; mutable access alone does not establish an order.
/// Presentation-only randomness should use another resource when spatial samples affect gameplay.
#[derive(Resource, Debug)]
pub struct SpatialRandom {
    rng: StdRng,
}

impl Default for SpatialRandom {
    fn default() -> Self {
        Self::from_seed(Self::DEFAULT_SEED)
    }
}

impl SpatialRandom {
    /// Seed used when the resource is initialized with its default configuration.
    pub const DEFAULT_SEED: u64 = 123456;

    /// Creates an independent spatial sampling stream initialized with `seed`.
    pub fn from_seed(seed: u64) -> Self {
        Self {
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Restarts this stream at `seed`, discarding its previous sampling position.
    /// Does not reset or consume the GAS probability stream.
    pub fn set_seed(&mut self, seed: u64) {
        self.rng = StdRng::seed_from_u64(seed);
    }

    /// Returns a value using Bevy's [`FromRng`] implementation, advancing this stream.
    pub fn from_rng<T>(&mut self) -> T
    where
        T: FromRng,
        StandardUniform: Distribution<T>,
    {
        T::from_rng(&mut self.rng)
    }

    /// Returns a point inside `shape` in its local coordinates, advancing this stream.
    /// The shape must satisfy its Bevy [`ShapeSample`] implementation's sampling requirements.
    pub fn sample_interior<S>(&mut self, shape: &S) -> S::Output
    where
        S: ShapeSample,
    {
        shape.sample_interior(&mut self.rng)
    }

    /// Returns a point on `shape`'s local boundary, advancing this stream.
    /// The shape must satisfy its Bevy [`ShapeSample`] implementation's sampling requirements.
    pub fn sample_boundary<S>(&mut self, shape: &S) -> S::Output
    where
        S: ShapeSample,
    {
        shape.sample_boundary(&mut self.rng)
    }
}
