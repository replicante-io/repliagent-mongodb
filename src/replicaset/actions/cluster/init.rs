//! Agent action to perform MongoDB ReplicaSet initialisation.
//!
//! The action will perform cluster initialisation using [`replSetInitiate`].
//! If the replica set is already initialised this action returns an error.
//!
//! ## ReplicaSet configuration
//!
//! This action will configure a replica set based on the following options:
//!
//! - The replica set ID.
//!   This is loaded from the MongoDB configuration with a call to [`getCmdLineOpts`].
//! - A single member is defined: the node itself.
//!   The host string for this node is defined in the `addresses.cluster` agent configuration.
//! - The Replica Set `settings` can be specified to the action arguments.
//!   The options are not checked and simply passed directly to the server.
//!
//! [`getCmdLineOpts`]: https://www.mongodb.com/docs/manual/reference/command/getCmdLineOpts/
//! [`replSetInitiate`]: https://www.mongodb.com/docs/manual/reference/command/replSetInitiate/
use anyhow::Context;
use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;

use replisdk::agent::framework::actions::ActionHandler;
use replisdk::agent::framework::actions::ActionHandlerChanges as Changes;
use replisdk::agent::framework::actions::ActionMetadata;
use replisdk::agent::framework::DefaultContext;
use replisdk::agent::models::ActionExecution;
use replisdk::agent::models::ActionExecutionPhase;

use crate::constants::DB_ADMIN;

/// Initialise a MongoDB Replica Set cluster.
pub struct Init {
    host: String,
}

impl Init {
    /// Registration metadata for the cluster initialisation action.
    pub fn metadata(host: String) -> ActionMetadata {
        let init = Init { host };
        replisdk::agent::framework::actions::wellknown::cluster::init(init)
    }
}

#[async_trait::async_trait]
impl ActionHandler for Init {
    async fn invoke(&self, context: &DefaultContext, action: &ActionExecution) -> Result<Changes> {
        let args = serde_json::from_value::<Option<InitArgs>>(action.args.clone())
            .context(InitError::InvalidArgs)?
            .unwrap_or_default();
        let client = crate::client::global();

        // Check current replica set config on node.
        let status = crate::client::admin::replica_set_status(&client).await;
        match status {
            Err(error) if crate::client::admin::replica_set_not_initialised(&error) => (),
            Err(error) => anyhow::bail!(error),
            Ok(_) => anyhow::bail!(InitError::AlreadyInitialised),
        };

        // Get ReplicaSet ID from getCmdLineOpts.
        let admin = client.database(DB_ADMIN);
        let command = mongodb::bson::doc! {"getCmdLineOpts": 1};
        // TODO(tracing): trace request to MongoDB.
        // TODO(metrics): count request to MongoDB.
        let conf = admin
            .run_command(command, None)
            .await
            .context(InitError::Failed)?;
        let rs_id = conf
            .get_document("parsed")
            .and_then(|parsed| parsed.get_document("replication"))
            .and_then(|replication| replication.get_str("replSetName"))
            .context(InitError::NoReplicaSetName)?;

        // Build replica set initialisation document.
        let mut init = mongodb::bson::doc! {
            "_id": rs_id,
            "members": [{
                "_id": 0,
                "host": &self.host,
            }],
        };
        if let Some(settings) = args.settings {
            init.insert("settings", settings);
        }

        // Initialise replica set.
        slog::info!(context.logger, "Initialising MongoDB replica set"; "conf" => %init);
        let command = mongodb::bson::doc! {"replSetInitiate": init};
        // TODO(tracing): trace request to MongoDB.
        // TODO(metrics): count request to MongoDB.
        admin
            .run_command(command, None)
            .await
            .context(InitError::Failed)?;
        let changes = Changes::to(ActionExecutionPhase::Done);
        Ok(changes)
    }
}

/// Arguments to customise replica set initialisation.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct InitArgs {
    /// Settings passed to the `replSetInitiate` command.
    #[serde(default)]
    pub settings: Option<mongodb::bson::Document>,
}

/// Errors returned by the replica set initialisation action.
#[derive(Debug, thiserror::Error)]
pub enum InitError {
    /// The replica set is already initialised.
    #[error("the replica set is already initialised")]
    AlreadyInitialised,

    /// Unable to initialise the replica set.
    #[error("unable to initialise the replica set")]
    Failed,

    /// Arguments provided to the [`Init`] action are not valid.
    #[error("arguments provided to the init action are not valid")]
    InvalidArgs,

    /// No replica set name was provided in MongoDB configuration or command.
    #[error("no replica set name was provided in MongoDB configuration or command")]
    NoReplicaSetName,
}