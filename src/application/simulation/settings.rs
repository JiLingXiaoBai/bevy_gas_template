use bevy::prelude::Resource;
use bevy_gas::Random;
use std::error::Error;
use std::fmt;
use std::time::Duration;

/// Validated startup settings shared by windowed, headless, and test simulations.
///
/// Changing this resource after startup does not reconfigure the clock or reset randomness.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimulationSettings {
    timestep: Duration,
    random_seed: u64,
}

impl Default for SimulationSettings {
    fn default() -> Self {
        Self {
            timestep: Duration::from_millis(50),
            random_seed: Random::DEFAULT_SEED,
        }
    }
}

impl SimulationSettings {
    /// Creates settings for `timestep` with the default GAS random seed.
    ///
    /// Returns an error for a zero duration, which cannot advance a fixed simulation.
    pub fn new(timestep: Duration) -> Result<Self, SimulationSettingsError> {
        if timestep.is_zero() {
            return Err(SimulationSettingsError::ZeroTimestep);
        }
        Ok(Self {
            timestep,
            ..Self::default()
        })
    }

    /// Creates settings from a positive, finite tick rate in hertz.
    ///
    /// Returns an error if the reciprocal cannot be represented as a nonzero `Duration`.
    pub fn try_from_hz(hertz: f64) -> Result<Self, SimulationSettingsError> {
        if !hertz.is_finite() || hertz <= 0.0 {
            return Err(SimulationSettingsError::InvalidTickRate);
        }
        let timestep = Duration::try_from_secs_f64(hertz.recip())
            .map_err(|_| SimulationSettingsError::InvalidTickRate)?;
        if timestep.is_zero() {
            return Err(SimulationSettingsError::InvalidTickRate);
        }
        Self::new(timestep)
    }

    /// Selects the initial GAS effect-probability seed and returns the settings.
    ///
    /// Games seed their independent spatial or presentation random streams separately.
    pub fn with_random_seed(mut self, seed: u64) -> Self {
        self.random_seed = seed;
        self
    }

    /// Returns the duration of one simulation tick.
    pub fn timestep(self) -> Duration {
        self.timestep
    }

    /// Returns the initial seed for GAS effect-probability evaluation.
    pub fn random_seed(self) -> u64 {
        self.random_seed
    }
}

/// Explains why simulation timing settings cannot be installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimulationSettingsError {
    /// A fixed tick must have a nonzero duration.
    ZeroTimestep,
    /// The rate is nonpositive, non-finite, or outside nonzero duration precision.
    InvalidTickRate,
}

impl fmt::Display for SimulationSettingsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroTimestep => formatter.write_str("simulation timestep must be nonzero"),
            Self::InvalidTickRate => formatter.write_str(
                "simulation tick rate must be positive, finite, and representable as a nonzero duration",
            ),
        }
    }
}

impl Error for SimulationSettingsError {}
