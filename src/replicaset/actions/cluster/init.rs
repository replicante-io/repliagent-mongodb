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
use std::future::IntoFuture;

use anyhow::Context as AnyContext;
use anyhow::Result;
use opentelemetry_api::trace::FutureExt;
use serde::Deserialize;
use serde::Serialize;

use replisdk::agent::framework::actions::ActionHandler;
use replisdk::agent::framework::actions::ActionHandlerChanges as Changes;
use replisdk::agent::framework::actions::ActionMetadata;
use replisdk::agent::models::ActionExecution;
use replisdk::agent::models::ActionExecutionPhase;
use replisdk::context::Context;
use replisdk::utils::metrics::CountFutureErrExt;
use replisdk::utils::trace::TraceFutureErrExt;
use replisdk::utils::trace::TraceFutureStdErrExt;

use crate::constants::CMD_GET_CMD_LINE_OPTS;
use crate::constants::CMD_REPL_SET_INIT;
use crate::constants::DB_ADMIN;
use crate::metrics::observe_mongodb_op;

/// Initialise a MongoDB Replica Set cluster.
#[derive(Debug)]
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
    async fn invoke(&self, context: &Context, action: &ActionExecution) -> Result<Changes> {
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
        let command = mongodb::bson::doc! {CMD_GET_CMD_LINE_OPTS: 1};

        let trace = crate::trace::mongodb_client_context(CMD_GET_CMD_LINE_OPTS);
        let (err_count, timer) = observe_mongodb_op(CMD_GET_CMD_LINE_OPTS);
        // Wrap the command to be traced into an anonymous future to decorate.
        let observed = async {
            let conf = admin
                .run_command(command)
                .await
                .context(InitError::Failed)?;
            let rs_id = conf
                .get_document("parsed")
                .and_then(|parsed| parsed.get_document("replication"))
                .and_then(|replication| {
                    let rs_id_key = if replication.contains_key("replSet") {
                        "replSet"
                    } else {
                        "replSetName"
                    };
                    replication.get_str(rs_id_key)
                })
                .context(InitError::NoReplicaSetName)?
                .to_owned();
            Result::Ok(rs_id)
        };
        // Decorate the operation once for all return clauses and execute.
        let rs_id = TraceFutureErrExt::trace_on_err_with_status(observed)
            .count_on_err(err_count)
            .with_context(trace)
            .await?;
        drop(timer);

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
        let command = mongodb::bson::doc! {CMD_REPL_SET_INIT: init};
        let trace = crate::trace::mongodb_client_context(CMD_REPL_SET_INIT);
        let (err_count, _timer) = observe_mongodb_op(CMD_REPL_SET_INIT);
        admin
            .run_command(command)
            .into_future()
            .count_on_err(err_count)
            .trace_on_err_with_status()
            .with_context(trace)
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
