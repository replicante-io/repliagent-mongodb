use clap::Parser;
use clap::Subcommand;

const DEFAULT_CONF_PATH: &str = "mongoagent.yaml";

/// Replicante Agent for MongoDB.
#[derive(Debug, Parser)]
#[command(version, about)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Path to the agent configuration file.
    #[arg(long, default_value_t = DEFAULT_CONF_PATH.to_string())]
    pub config: String,

    #[command(subcommand)]
    pub mode: Mode,
}

/// Select the mode to run the agent in.
#[derive(Clone, Debug, Subcommand)]
pub enum Mode {
    /// Run the agent in ReplicaSet mode (for members of a Replica Set cluster).
    #[command(alias = "rs", alias = "replica", alias = "replicaset")]
    ReplicaSet,
}
