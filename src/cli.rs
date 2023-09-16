use clap::Parser;
use clap::Subcommand;

/// Replicante Agent for MongoDB.
#[derive(Debug, Parser)]
#[command(version, about)]
#[command(propagate_version = true)]
pub struct Cli {
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
