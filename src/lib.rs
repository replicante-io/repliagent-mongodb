use anyhow::Result;
use clap::Parser;
use once_cell::sync::Lazy;

use replisdk::agent::models::AgentVersion;

mod cli;
mod client;
mod conf;
mod constants;
mod errors;
mod replicaset;

use self::cli::Cli;
use self::cli::Mode;

/// ID of the agent release in sentry recommanded format.
const RELEASE_ID: &str = concat!(env!("CARGO_PKG_NAME"), "@", env!("CARGO_PKG_VERSION"));

/// Agent version information to expose as part of the node status.
static AGENT_VERSION: Lazy<AgentVersion> = Lazy::new(|| AgentVersion {
    checkout: env!("GIT_BUILD_HASH").into(),
    number: env!("CARGO_PKG_VERSION").into(),
    taint: env!("GIT_BUILD_TAINT").into(),
});

/// Initialise the process to run MongoDB agents.
pub fn run() -> Result<()> {
    // Parse command line options and decide what to run.
    let args = Cli::parse();
    match args.mode {
        Mode::ReplicaSet => self::replicaset::run(args),
    }
}
