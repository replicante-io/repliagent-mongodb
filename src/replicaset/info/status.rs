//! Detect the node status for Replica Set members.
use anyhow::Result;
use mongodb::bson::Document;
use mongodb::error::Error;
use mongodb::error::ErrorKind;
use mongodb::error::Result as MdbResult;
use slog::Logger;

use replisdk::agent::models::NodeStatus;

use crate::constants::MemberState;

/// Get the current [`NodeStatus`] of the managed node based on the replSetGetStatus command.
pub async fn get(result: MdbResult<Document>, logger: &Logger) -> Result<NodeStatus> {
    let status = match result {
        Ok(status) => status,
        Err(error) => {
            slog::debug!(logger, "Error executing replSetGetStatus"; "server_error" => %error);
            let status = status_for_error(error).await?;
            return Ok(status);
        }
    };

    // Determine the node status based on the replica set status.
    let state = status.get_i32("myState").unwrap_or(6);
    let state = match MemberState::try_from(state) {
        Ok(state) => state,
        Err(error) => {
            let status = NodeStatus::Unknown(error.to_string());
            return Ok(status);
        }
    };
    let state = match state {
        MemberState::Startup | MemberState::Recovering | MemberState::Rollback => {
            NodeStatus::Unhealthy
        }
        MemberState::Primary | MemberState::Secondary => NodeStatus::Healthy,
        MemberState::Startup2 => NodeStatus::JoiningCluster,
        MemberState::Removed => NodeStatus::NotInCluster,
        state => {
            let state = format!(
                "Unable to determine status of mode with replica set state {}",
                state
            );
            NodeStatus::Unknown(state)
        }
    };
    Ok(state)
}

/// Determine the [`NodeStatus`] based on the error response to the `replSetGetStatus` command.
async fn status_for_error(error: Error) -> Result<NodeStatus> {
    // Check for connection related errors, suggesting the store process is down.
    let is_connection_error = matches!(
        *error.kind,
        ErrorKind::Authentication { .. }
            | ErrorKind::ConnectionPoolCleared { .. }
            | ErrorKind::Io(_)
            | ErrorKind::ServerSelection { .. }
    );
    if is_connection_error {
        return Ok(NodeStatus::Unavailable);
    }

    // Check for the server error response indicating the replica set is not initialised.
    if crate::client::admin::replica_set_not_initialised(&error) {
        return Ok(NodeStatus::NotInCluster);
    }

    // Consider all other errors unknown.
    let message = error.to_string();
    Ok(NodeStatus::Unknown(message))
}
