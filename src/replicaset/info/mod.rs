//! NodeInfo implementation for ReplicaSet nodes.
use anyhow::Context;
use anyhow::Result;
use mongodb::bson::Document;
use mongodb::Client;
use once_cell::sync::Lazy;

use replisdk::agent::framework::DefaultContext;
use replisdk::agent::framework::NodeInfo;
use replisdk::agent::framework::StoreVersionChain;
use replisdk::agent::framework::StoreVersionStrategy;
use replisdk::agent::models::AttributesMap;
use replisdk::agent::models::Node;
use replisdk::agent::models::ShardsInfo;
use replisdk::agent::models::StoreExtras;

mod factory;
mod shard;
mod status;
mod version;

pub use self::factory::MongoInfoFactory;

use crate::client::admin::replica_set_status;
use crate::constants::ATTRIBUTE_PREFIX;
use crate::constants::CMD_COLL_STATS;
use crate::constants::CMD_GET_PARAMETER;
use crate::constants::DB_ADMIN;
use crate::constants::DB_LOCAL;
use crate::errors::MongoInfoError;

/// Store ID reported for nodes.
const STORE_ID: &str = "mongo.replica";

/// Set of never-changing agent attributes to include in responses.
static STATIC_ATTRIBUTES: Lazy<AttributesMap> = Lazy::new(|| {
    let mut attributes = AttributesMap::new();
    attributes.insert(format!("{}/mode", ATTRIBUTE_PREFIX), "replica-set".into());
    attributes
});

/// Gather MongoDB node information.
#[derive(Clone, Debug)]
pub struct MongoInfo {
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

impl MongoInfo {
    /// Lookup MongoDB current feature compatibility version (FCV).
    async fn feature_compatibility_version(&self) -> Result<String> {
        // TODO(tracing): trace request to MongoDB.
        let command = {
            let mut command = Document::new();
            command.insert(CMD_GET_PARAMETER, 1);
            command.insert("featureCompatibilityVersion", 1);
            command
        };
        let admin = self.client.database(DB_ADMIN);
        let params = admin.run_command(command, None).await?;
        match params.get_document("featureCompatibilityVersion") {
            Err(error) => Err(anyhow::anyhow!(error).context(MongoInfoError::FeatCompatVerUnknown)),
            Ok(doc) => {
                let version = doc
                    .get_str("version")
                    .context(MongoInfoError::FeatCompatVerNotSet)?
                    .to_string();
                Ok(version)
            }
        }
    }

    /// Lookup oplog collection max size.
    async fn oplog_size(&self) -> Result<i32> {
        // TODO(tracing): trace request to MongoDB.
        let command = {
            let mut command = Document::new();
            command.insert(CMD_COLL_STATS, "oplog.rs");
            command
        };
        let local = self.client.database(DB_LOCAL);
        let stats = local
            .run_command(command, None)
            .await
            .context(MongoInfoError::OplogStatsUnknown)?;
        let max_size = stats
            .get_i32("maxSize")
            .context(MongoInfoError::OplogStatsNoSize)?;
        Ok(max_size)
    }
}

#[async_trait::async_trait]
impl NodeInfo for MongoInfo {
    type Context = DefaultContext;

    async fn node_info(&self, context: &Self::Context) -> Result<Node> {
        let rs = replica_set_status(&self.client).await;
        let node_status = self::status::get(rs, &context.logger).await?;
        let store_version = self.version.version(context).await?;
        let node = Node {
            agent_version: crate::AGENT_VERSION.clone(),
            attributes: STATIC_ATTRIBUTES.clone(),
            node_id: self.node_id.clone(),
            node_status,
            store_id: STORE_ID.into(),
            store_version,
        };
        Ok(node)
    }

    async fn shards(&self, _: &Self::Context) -> Result<ShardsInfo> {
        let status = replica_set_status(&self.client)
            .await
            .context(MongoInfoError::ReplicaSetStatusUnknown)?;
        let shard = shard::shard(status)?;
        Ok(ShardsInfo {
            shards: vec![shard],
        })
    }

    async fn store_info(&self, _: &Self::Context) -> Result<StoreExtras> {
        // Get the cluster ID from the RS status.
        let status = replica_set_status(&self.client)
            .await
            .context(MongoInfoError::ReplicaSetStatusUnknown)?;
        let name = status
            .get_str("set")
            .context(MongoInfoError::ReplicaSetStatusNoName)?;

        // Build additional attributes.
        let mut attributes = AttributesMap::new();
        let oplog_size = self.oplog_size().await?;
        attributes.insert(
            format!("{}/oplog.size", ATTRIBUTE_PREFIX),
            serde_json::Number::from(oplog_size).into(),
        );
        let feature_compat_ver = self.feature_compatibility_version().await?;
        attributes.insert(
            format!("{}/feature-compatibility", ATTRIBUTE_PREFIX),
            feature_compat_ver.into(),
        );

        Ok(StoreExtras {
            cluster_id: name.to_string(),
            attributes,
        })
    }
}
