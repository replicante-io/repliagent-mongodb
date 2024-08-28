//! Agent for MongoDB nodes running in a Replica Set cluster.
use anyhow::Result;

use replisdk::agent::framework::Agent;
use replisdk::agent::framework::AgentConf;
use replisdk::agent::framework::AgentOptions;
use replisdk::runtime::telemetry::TelemetryOptions;

use crate::conf::Conf;
use crate::Cli;

mod actions;
mod info;

/// Explicitly typed Agent builder for MongoDB agents.
///
/// Having this explicit type can defined decorator functions to set up the agent
/// and surface type-related issues quickly and more clearly.
type MongoAgent = Agent<Conf, info::MongoInfoFactory>;

/// Configuration of MongoDB agents.
type MongoConf = AgentConf<Conf>;

/// Run a Replicante Agent for MongoDB nodes in ReplicaSet clusters.
pub fn run(args: Cli) -> Result<()> {
    let mut conf = crate::conf::load(&args.config, MongoConf::default())?;
    crate::conf::apply_overrides(&mut conf.custom)?;
    conf.runtime
        .tokio
        .clone()
        .into_runtime()
        .expect("failed configuration of tokio runtime")
        .block_on(async_run(args, conf))
}

async fn async_run(_args: Cli, conf: MongoConf) -> Result<()> {
    // SAFETY: Existence of this value if guaranteed by `crate::conf::apply_overrides`.
    let host = conf.custom.addresses.cluster.clone().unwrap();
    let options = AgentOptions {
        requests_metrics_prefix: "repliagent",
    };
    let telemetry = TelemetryOptions::for_sentry_release(crate::RELEASE_ID)
        .for_app(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
        .finish();

    // Configure the agent process using the `Agent` builder.
    let agent = MongoAgent::build()
        .configure(conf)
        .options(options)
        .telemetry_options(telemetry)
        .node_info(info::MongoInfo::factory())
        .initialise_with(crate::client::Initialise)
        .initialise_with(crate::metrics::Register)
        .register_actions(replisdk::agent::framework::actions::wellknown::test::all())
        .register_action(actions::cluster::Add::metadata())
        .register_action(actions::cluster::Init::metadata(host));

    // Run the agent until error or shutdown.
    agent.run().await
}
