//! Configuration logic and models.
use std::fs::File;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;

use replisdk::agent::framework::StoreVersionCommandConf;

const AGENT_ADDRESS_CLUSTER: &str = "RA_ADDRESS_CLUSTER";
const MONGO_CREDENTIAL_PASSWORD: &str = "MONGO_PASSWORD";

/// Network addresses for the MongoDB node depending on intended client.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Addresses {
    /// MongoDB node address within the replica set.
    #[serde(default)]
    pub cluster: Option<String>,

    /// MongoDB node address for the agent to connect to.
    #[serde(default = "Addresses::default_local")]
    pub local: String,
}

impl Addresses {
    fn default_local() -> String {
        "localhost:27017".into()
    }
}

impl Default for Addresses {
    fn default() -> Self {
        Addresses {
            cluster: None,
            local: Self::default_local(),
        }
    }
}

/// Agent configuration specific to MongoDB.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Conf {
    /// Network addresses for the MongoDB node depending on intended client.
    pub addresses: Addresses,
    
    /// Timeout in seconds for connections with the server to be established.
    #[serde(default)]
    pub connection_timeout: Option<u64>,

    /// MongoDB authentication credentials and mode.
    #[serde(default)]
    pub credentials: Option<Credentials>,

    /// The amount of time in seconds to wait between server health checks.
    #[serde(default)]
    pub heartbeat_frequency: Option<u64>,

    /// The amount of time in seconds to keep idle connections open for reuse.
    ///
    /// A value of zero means that connections will not be closed for being idle.
    pub max_idle_time: Option<u64>,

    /// TLS configuration for connections to the server.
    #[serde(default)]
    pub tls: Option<Tls>,

    /// Configure MongoDB version detection strategies.
    #[serde(default)]
    pub version_detect: VersionDetect,
}

impl Default for Conf {
    fn default() -> Self {
        Conf {
            addresses: Addresses::default(),
            connection_timeout: None,
            credentials: None,
            heartbeat_frequency: None,
            max_idle_time: None,
            tls: None,
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

/// MongoDB authentication credentials and mode.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Credentials {
    /// The authentication mechanism to use.
    #[serde(default)]
    pub mechanism: Option<CredentialsMechanism>,

    /// Username to authenticate to MongoDB with.
    #[serde(default)]
    pub username: Option<String>,

    /// Name of the users database to authenticate against.
    #[serde(default)]
    pub source: Option<String>,
}

impl From<Credentials> for mongodb::options::Credential {
    fn from(value: Credentials) -> Self {
        let password = std::env::var(MONGO_CREDENTIAL_PASSWORD).ok();
        let mechanism = value.mechanism.map(mongodb::options::AuthMechanism::from);
        mongodb::options::Credential::builder()
            .mechanism(mechanism)
            .password(password)
            .source(value.source)
            .username(value.username)
            .build()
    }
}

/// Supported authentication mechanisms to authenticate with the server.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CredentialsMechanism {
    /// Use the Kerberos mechanism.
    #[serde(rename = "GSS-API")]
    Gssapi,

    /// Use the MONGODB-X509 mechanism.
    #[serde(rename = "MONGODB-X509")]
    MongoDbX509,

    /// Use the PLAIN mechanism.
    #[serde(rename = "PLAIN")]
    Plain,

    /// Use the SCRAM-SHA-1 mechanism.
    #[serde(rename = "SCRAM-SHA-1")]
    ScramSha1,

    /// Use the SCRAM-SHA-256 mechanism.
    #[serde(rename = "SCRAM-SHA-256")]
    ScramSha256,
}

impl From<CredentialsMechanism> for mongodb::options::AuthMechanism {
    fn from(value: CredentialsMechanism) -> Self {
        match value {
            CredentialsMechanism::Gssapi => mongodb::options::AuthMechanism::Gssapi,
            CredentialsMechanism::MongoDbX509 => mongodb::options::AuthMechanism::MongoDbX509,
            CredentialsMechanism::Plain => mongodb::options::AuthMechanism::Plain,
            CredentialsMechanism::ScramSha1 => mongodb::options::AuthMechanism::ScramSha1,
            CredentialsMechanism::ScramSha256 => mongodb::options::AuthMechanism::ScramSha256,
        }
    }
}

/// TLS configuration for connections to the server.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tls {
    /// The client should ignore invalid certificates from the server.
    #[serde(default)]
    pub allow_invalid_certificates: Option<bool>,

    /// The client should ignore invalid certificate hostnames.
    #[serde(default)]
    pub allow_invalid_hostnames: Option<bool>,

    /// Path to the CA file to verify the server certificate with.
    #[serde(default)]
    pub ca_file_path: Option<String>,

    /// Path to the client certificate to present to server.
    #[serde(default)]
    pub cert_key_file_path: Option<String>,
}

impl Tls {
    /// Convert the Agent TLS configuration into a MongoDB client configuration.
    pub fn into_client_option(value: &Option<Tls>) -> mongodb::options::Tls {
        let value = match value {
            None => return mongodb::options::Tls::Disabled,
            Some(value) => value,
        };
        let options = mongodb::options::TlsOptions::builder()
            .allow_invalid_certificates(value.allow_invalid_certificates)
            .allow_invalid_hostnames(value.allow_invalid_hostnames)
            .ca_file_path(value.ca_file_path.clone().map(std::path::PathBuf::from))
            .cert_key_file_path(
                value
                    .cert_key_file_path
                    .clone()
                    .map(std::path::PathBuf::from),
            )
            .build();
        mongodb::options::Tls::Enabled(options)
    }
}

/// Apply configuration overrides from the process environment.
pub fn apply_overrides(conf: &mut Conf) -> Result<()> {
    if let Ok(address) = std::env::var(AGENT_ADDRESS_CLUSTER) {
        conf.addresses.cluster = Some(address);
    }

    // Ensure addresses.cluster is set once overrides are applied.
    if conf.addresses.cluster.is_none() {
        anyhow::bail!(crate::errors::ConfError::NoClusterAddress);
    }
    Ok(())
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
