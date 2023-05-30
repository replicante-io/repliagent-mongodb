//! Agent for MongoDB nodes running in a Replica Set cluster.
use anyhow::Result;

use replisdk::agent::framework::Agent;
use replisdk::agent::framework::AgentConf;
use replisdk::agent::framework::AgentOptions;
use replisdk::runtime::telemetry::TelemetryOptions;

use crate::Cli;

const DEFAULT_CONF_PATH: &str = "mongoagent.yaml";

/// Configuration of MongoDB agents.
type MongoConf = AgentConf<()>;

/// Run a Replicante Agent for MongoDB nodes in ReplicaSet clusters.
pub fn run(args: Cli) -> Result<()> {
    let conf = crate::conf::load(DEFAULT_CONF_PATH, MongoConf::default())?;
    conf.runtime
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
    let agent = Agent::build()
        .configure(conf)
        .options(options)
        .telemetry_options(telemetry);

    // Run the agent until error or shutdown.
    agent.run().await
}

/* *** Agent process template ***
    Agent::build()
        .agent_info(info::AgentInfo::new(...))
        .watch_task(background::custom_worker_task(...))
        .watch_task(background::store_monitor_task(...))
        .register_action(actions::custom(...))
        .register_action(actions::cluster::init(...))
        .register_action(actions::cluster::join(...))

        / Once the agent is configured we can run it forever.
        .run()
        .await
})
*/
