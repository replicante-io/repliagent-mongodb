use anyhow::Result;
use clap::Parser;

mod cli;
mod conf;
mod replicaset;

use self::cli::Cli;
use self::cli::Mode;

/// ID of the agent release in sentry recommanded format.
const RELEASE_ID: &str = concat!(env!("CARGO_PKG_NAME"), "@", env!("CARGO_PKG_VERSION"));

/// Initialise the process to run MongoDB agents.
pub fn run() -> Result<()> {
    // Parse command line options and decide what to run.
    let args = Cli::parse();
    match args.mode {
        Mode::ReplicaSet => self::replicaset::run(args),
    }
}
