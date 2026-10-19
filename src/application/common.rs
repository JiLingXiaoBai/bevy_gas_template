use std::error::Error;
#[cfg(not(feature = "windowed"))]
use std::path::Path;
use std::path::PathBuf;

/// The result of game initialization or deterministic gameplay verification.
///
/// Successful calls return their requested value; errors preserve configuration,
/// attribute, or runtime failures with contextual startup information.
pub type GameResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

/// Returns the game's default binary configuration directory.
///
/// The absolute path is based on this package's manifest directory, so launching
/// the executable from another working directory still loads the game's data.
pub fn default_config_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/config")
}

/// Reports that the interactive sample requires the disabled `windowed` feature.
///
/// `directory` is accepted for API compatibility but no configuration is loaded.
/// Returns an error explaining how to enable window support or run headlessly.
#[cfg(not(feature = "windowed"))]
pub fn run_windowed(_directory: &Path) -> GameResult<()> {
    Err("The interactive sample requires the `windowed` feature. Enable it with `--features windowed`, or pass `--headless` to run without a window.".into())
}
