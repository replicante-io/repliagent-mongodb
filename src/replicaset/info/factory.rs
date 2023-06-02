//! Factory for MongoInfo instances.
use anyhow::Result;

use replisdk::agent::framework::detect_node_id;
use replisdk::agent::framework::DefaultContext;
use replisdk::agent::framework::NodeInfoFactory;
use replisdk::agent::framework::NodeInfoFactoryArgs;

use super::MongoInfo;
use crate::conf::Conf;

/// Create instances of [`MongoInfo`] at the correct process initialisation time.
pub struct MongoInfoFactory {}

#[async_trait::async_trait]
impl NodeInfoFactory for MongoInfoFactory {
    type Conf = Conf;
    type Context = DefaultContext;
    type NodeInfo = MongoInfo;

    async fn factory<'a>(&self, args: NodeInfoFactoryArgs<'a, Self::Conf>) -> Result<MongoInfo> {
        // Grab identifiers to report from the API.
        let node_id = detect_node_id(args.conf, &args.telemetry.logger).await?;

        // Configure the store version detection strategies.
        let version = super::version::configure_strategies(args.clone())?;

        // Initialise the MongoDB client.
        slog::debug!(args.telemetry.logger, "Initialising MongoDB client");
        let client = crate::client::initialise(&args.conf.custom)?;

        // Create the MongoInfo instance.
        Ok(MongoInfo {
            client,
            node_id,
            version,
        })
    }
}
