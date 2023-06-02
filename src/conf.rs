//! Configuration logic and models.
use std::fs::File;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;

use replisdk::agent::framework::StoreVersionCommandConf;

/// Agent configuration specific to MongoDB.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Conf {
    /// MongoDB node address for the agent to connect to.
    #[serde(default = "Conf::default_node_address")]
    pub node_address: String,

    // TODO: connection timeout
    // TODO: credentials
    // TODO: heartbeat frequency
    // TODO? max idle time
    // TODO? replica set name
    // TODO: TLS
    /// Configure MongoDB version detection strategies.
    #[serde(default)]
    pub version_detect: VersionDetect,
}

impl Conf {
    fn default_node_address() -> String {
        "localhost:27017".into()
    }
}

impl Default for Conf {
    fn default() -> Self {
        Conf {
            node_address: Self::default_node_address(),
            version_detect: VersionDetect::default(),
        }
    }
}

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

/// Configure MongoDB version detection strategies.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct VersionDetect {
    /// Override default store detection command.
    #[serde(default)]
    pub command: Option<StoreVersionCommandConf>,

    /// Optional file to detect the MongoDB version from.
    #[serde(default)]
    pub file: Option<String>,
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
