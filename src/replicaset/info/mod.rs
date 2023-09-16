//! NodeInfo implementation for ReplicaSet nodes.
use anyhow::Context as AnyContext;
use anyhow::Result;
use mongodb::bson::Document;
use mongodb::Client;
use once_cell::sync::Lazy;
use opentelemetry_api::trace::FutureExt;

use replisdk::agent::framework::NodeInfo;
use replisdk::agent::framework::StoreVersionChain;
use replisdk::agent::framework::StoreVersionStrategy;
use replisdk::agent::models::AttributesMap;
use replisdk::agent::models::Node;
use replisdk::agent::models::ShardsInfo;
use replisdk::agent::models::StoreExtras;
use replisdk::context::Context;
use replisdk::utils::metrics::CountFutureErrExt;
use replisdk::utils::trace::TraceFutureErrExt;

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
use crate::constants::FEATURE_COMPATIBILITY_VERSION;
use crate::errors::MongoInfoError;
use crate::metrics::observe_mongodb_op;

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
        let trace = crate::trace::mongodb_client_context(FEATURE_COMPATIBILITY_VERSION);
        let (err_count, _timer) = observe_mongodb_op(FEATURE_COMPATIBILITY_VERSION);

        let admin = self.client.database(DB_ADMIN);
        let command = {
            let mut command = Document::new();
            command.insert(CMD_GET_PARAMETER, 1);
            command.insert(FEATURE_COMPATIBILITY_VERSION, 1);
            command
        };

        // Wrap the command to be traced into an anonymous future to decorate.
        let observed = async {
            let params = admin
                .run_command(command, None)
                .await
                .context(MongoInfoError::FeatCompatVerUnknown)?;
            match params.get_document(FEATURE_COMPATIBILITY_VERSION) {
                Err(error) => {
                    Err(anyhow::anyhow!(error).context(MongoInfoError::FeatCompatVerUnknown))
                }
                Ok(doc) => {
                    let version = doc
                        .get_str("version")
                        .context(MongoInfoError::FeatCompatVerNotSet)?
                        .to_string();
                    Ok(version)
                }
            }
        };

        // Decorate the operation once for all return clauses and execute.
        observed
            .count_on_err(err_count)
            .trace_on_err_with_status()
            .with_context(trace)
            .await
    }

    /// Lookup oplog collection max size.
    async fn oplog_size(&self) -> Result<i32> {
        let trace = crate::trace::mongodb_client_context(CMD_COLL_STATS);
        let (err_count, _timer) = observe_mongodb_op(CMD_COLL_STATS);

        let command = {
            let mut command = Document::new();
            command.insert(CMD_COLL_STATS, "oplog.rs");
            command
        };
        let local = self.client.database(DB_LOCAL);

        // Wrap the command to be traced into an anonymous future to decorate.
        let observed = async {
            let stats = local
                .run_command(command, None)
                .await
                .context(MongoInfoError::OplogStatsUnknown)?;
            let max_size = stats
                .get_i32("maxSize")
                .context(MongoInfoError::OplogStatsNoSize)?;
            Ok(max_size)
        };

        // Decorate the operation once for all return clauses and execute.
        observed
            .count_on_err(err_count)
            .trace_on_err_with_status()
            .with_context(trace)
            .await
    }
}

#[async_trait::async_trait]
impl NodeInfo for MongoInfo {
    async fn node_info(&self, context: &Context) -> Result<Node> {
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

    async fn shards(&self, _: &Context) -> Result<ShardsInfo> {
        let status = replica_set_status(&self.client)
            .await
            .context(MongoInfoError::ReplicaSetStatusUnknown)?;
        let shard = shard::shard(status)?;
        Ok(ShardsInfo {
            shards: vec![shard],
        })
    }

    async fn store_info(&self, _: &Context) -> Result<StoreExtras> {
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
