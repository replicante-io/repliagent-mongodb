//! Agent action to add a node to the current Replica Set.
//!
//! The action will reconfigure the replica set to add a node with [`replSetInitiate`].
//! If the current node is not the Replica Set primary the action will fail.
//!
//! ## Arguments
//!
//! TODO
//!
//! [`replSetReconfig`]: https://www.mongodb.com/docs/manual/reference/command/replSetReconfig/
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

const RS_ATTR_MEMBER_ID: &str = "_id";
const RS_ATTR_MEMBERS: &str = "members";
const RS_ATTR_VERSION: &str = "version";

/// Add a node to the Replica Set cluster.
pub struct Add;

impl Add {
    /// Registration metadata for the cluster initialisation action.
    pub fn metadata() -> ActionMetadata {
        replisdk::agent::framework::actions::wellknown::cluster::add(Add)
    }
}

#[async_trait::async_trait]
impl ActionHandler for Add {
    async fn invoke(&self, context: &DefaultContext, action: &ActionExecution) -> Result<Changes> {
        let args: AddArgs =
            serde_json::from_value(action.args.clone()).context(AddError::InvalidArgs)?;
        let client = crate::client::global();

        // Get current RS configuration.
        let admin = client.database(DB_ADMIN);
        let command = mongodb::bson::doc! {"replSetGetConfig": 1};
        // TODO(tracing): trace request to MongoDB.
        // TODO(metrics): count request to MongoDB.
        let rs = admin
            .run_command(command, None)
            .await
            .context(AddError::Failed)?
            .remove("config")
            .ok_or_else(|| anyhow::anyhow!("server did not return replica set configuration"))
            .context(AddError::RsConf)?;
        let mut rs = match rs {
            mongodb::bson::Bson::Document(rs) => rs,
            _ => {
                let error = anyhow::anyhow!("server returned invalid type for rs configuration");
                anyhow::bail!(error.context(AddError::RsConf))
            }
        };
        let members = rs
            .get_array_mut(RS_ATTR_MEMBERS)
            .context(AddError::RsAttr(RS_ATTR_MEMBERS))?;

        // Build new node document.
        let mut nid = 0;
        for member in members.iter() {
            let id = member
                .as_document()
                .ok_or_else(|| anyhow::anyhow!("elements in members array must be an object"))
                .context(AddError::RsConf)?
                .get_i32(RS_ATTR_MEMBER_ID)
                .context(AddError::RsAttr(RS_ATTR_MEMBER_ID))?;
            if id > nid {
                nid = id;
            }
        }
        let node = mongodb::bson::doc! {
            "_id": nid + 1,
            "host": args.host,
        };

        // Reconfigure the replica set.
        slog::info!(context.logger, "Adding node to replica set"; "node" => %node);
        members.push(node.into());
        let version = rs
            .get_i32_mut(RS_ATTR_VERSION)
            .context(AddError::RsAttr(RS_ATTR_VERSION))?;
        *version += 1;

        let command = mongodb::bson::doc! {"replSetReconfig": rs};
        // TODO(tracing): trace request to MongoDB.
        // TODO(metrics): count request to MongoDB.
        admin
            .run_command(command, None)
            .await
            .context(AddError::Failed)?;
        let changes = Changes::to(ActionExecutionPhase::Done);
        Ok(changes)
    }
}

/// Arguments to add a new node to the replica set.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AddArgs {
    /// Index to use for the new node `_id` attribute.
    ///
    /// If not set, largest integer not currently in use is assigned.
    pub id: Option<u32>,

    /// Value of the new node for the `host` attribute.
    pub host: String,
}

/// Errors encountered while adding the new node.
#[derive(Debug, thiserror::Error)]
pub enum AddError {
    /// Unable to add node to replica set.
    #[error("unable to add node to replica set")]
    Failed,

    /// Arguments provided to the [`Add`] action are not valid.
    #[error("arguments provided to the add action are not valid")]
    InvalidArgs,

    /// Attribute is missing on has unexpected type.
    #[error("attribute '{0}' is missing on has unexpected type")]
    // (attribute,)
    RsAttr(&'static str),

    /// Invalid replica set configuration.
    #[error("invalid replica set configuration")]
    RsConf,
}
