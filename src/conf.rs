//! Configuration logic and models.
use std::fs::File;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;

/// Errors while loading server configuration.
#[derive(Debug, thiserror::Error)]
pub enum ConfError {
    /// Unable to load configuration from file.
    ///
    /// Error parameters:
    ///
    /// - Path to the configuration file.
    #[error("unable to load configuration from file: '{0}'")]
    Load(String),

    /// Unable to open the configuration file.
    ///
    /// Error parameters:
    ///
    /// - Path to the configuration file.
    #[error("unable to open the configuration file: '{0}'")]
    Open(String),
}

/// Load the agent configuration from file, if the file exists.
pub fn load<C>(path: &str, default: C) -> Result<C>
where
    C: serde::de::DeserializeOwned,
{
    // Check if the configuration file exists and return the default if it does not.
    if !PathBuf::from(path).exists() {
        return Ok(default);
    }

    // Load and deserialize the agent configuration.
    let file = File::open(path).with_context(|| ConfError::Open(path.into()))?;
    let conf = serde_yaml::from_reader(file).with_context(|| ConfError::Load(path.into()))?;
    Ok(conf)
}
