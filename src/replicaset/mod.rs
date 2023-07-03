//! Agent for MongoDB nodes running in a Replica Set cluster.
use anyhow::Result;

use replisdk::agent::framework::Agent;
use replisdk::agent::framework::AgentConf;
use replisdk::agent::framework::AgentOptions;
use replisdk::runtime::telemetry::TelemetryOptions;

use crate::conf::Conf;
use crate::Cli;

mod info;

const DEFAULT_CONF_PATH: &str = "mongoagent.yaml";

/// Explicitly typed Agent builder for MongoDB agents.
///
/// Having this explicit type can defined decorator functions to set up the agent
/// and surface type-related issues quickly and more clearly.
type MongoAgent = Agent<Conf, info::MongoInfoFactory>;

/// Configuration of MongoDB agents.
type MongoConf = AgentConf<Conf>;

/// Run a Replicante Agent for MongoDB nodes in ReplicaSet clusters.
pub fn run(args: Cli) -> Result<()> {
    let conf = crate::conf::load(DEFAULT_CONF_PATH, MongoConf::default())?;
    conf.runtime
        .tokio
        .clone()
        .into_runtime()
        .expect("failed configuration of tokio runtime")
        .block_on(async_run(args, conf))
}

async fn async_run(_args: Cli, conf: MongoConf) -> Result<()> {
    // Configure the agent process using the `Agent` builder.
    let options = AgentOptions {
        requests_metrics_prefix: "repliagent",
    };
    let telemetry = TelemetryOptions::for_sentry_release(crate::RELEASE_ID);
    let agent = MongoAgent::build()
        .configure(conf)
        .options(options)
        .telemetry_options(telemetry)
        .node_info(info::MongoInfo::factory())
        .initialise_with(crate::client::initialiser());

    // Run the agent until error or shutdown.
    agent.run().await
}

/* *** Agent process template ***
    Agent::build()
        .register_action(actions::custom(...))
        .register_action(actions::cluster::init(...))
        .register_action(actions::cluster::join(...))
})
*/
