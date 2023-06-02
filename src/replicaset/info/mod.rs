//! NodeInfo implementation for ReplicaSet nodes.
use anyhow::Result;
use mongodb::Client;

use replisdk::agent::framework::DefaultContext;
use replisdk::agent::framework::NodeInfo;
use replisdk::agent::framework::StoreVersionChain;
use replisdk::agent::framework::StoreVersionStrategy;
use replisdk::agent::models::Node;

mod factory;
mod status;
mod version;

pub use self::factory::MongoInfoFactory;

/// Store ID reported for nodes.
const STORE_ID: &str = "mongo.replica";

/// Gather MongoDB node information.
#[derive(Clone, Debug)]
pub struct MongoInfo {
    // TODO: remove once client is used.
    #[allow(dead_code)]
    client: Client,
    node_id: String,
    version: StoreVersionChain,
}

impl MongoInfo {
    /// Return the factory for [`MongoInfo`] instances.
    pub fn factory() -> MongoInfoFactory {
        MongoInfoFactory {}
    }
}

#[async_trait::async_trait]
impl NodeInfo for MongoInfo {
    type Context = DefaultContext;

    async fn node_info(&self, context: &Self::Context) -> Result<Node> {
        let node_status = self::status::get(&self.client, &context.logger).await?;
        let store_version = self.version.version(context).await?;
        let node = Node {
            agent_version: crate::AGENT_VERSION.clone(),
            attributes: Default::default(),
            node_id: self.node_id.clone(),
            node_status,
            store_id: STORE_ID.into(),
            store_version,
        };
        Ok(node)
    }
}
