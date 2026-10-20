use std::error::Error;
use std::path::PathBuf;

/// The result of game initialization or execution.
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
