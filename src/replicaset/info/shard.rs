//! Model the replica set status into a [`Shard`].
use anyhow::Context;
use anyhow::Result;
use mongodb::bson::Document;

use replisdk::agent::models::Shard;
use replisdk::agent::models::ShardCommitOffset;
use replisdk::agent::models::ShardRole;

use crate::constants::MemberState;
use crate::errors::MongoInfoError;

/// Model the replica set status into a [`Shard`].
pub fn shard(status: Document) -> Result<Shard> {
    let members = status
        .get_array("members")
        .context(MongoInfoError::ReplicaSetStatusNoMembers)?;

    // Find the information about the current node.
    let my_self = members
        .iter()
        .find(|member| {
            member
                .as_document()
                .map(|member| member.get_bool("self").unwrap_or(false))
                .unwrap_or(false)
        })
        .ok_or(MongoInfoError::ReplicaSetStatusNoSelf)?;
    let my_self = my_self
        .as_document()
        .ok_or(MongoInfoError::ReplicaSetStatusNoSelf)?;

    // Find the information about the primary node, if other then ourselves.
    let primary = members
        .iter()
        .find(|member| {
            member
                .as_document()
                .map(|member| {
                    let state = member
                        .get_i32("state")
                        .unwrap_or(MemberState::Unknown as i32);
                    state == (MemberState::Primary as i32)
                })
                .unwrap_or(false)
        })
        .and_then(|primary| primary.as_document())
        .and_then(|primary| {
            if primary.get_i32("_id") == my_self.get_i32("_id") {
                None
            } else {
                Some(primary)
            }
        });

    // Extract the relevant attributes.
    //  - Replica Set name (as Shard ID).
    let shard_id = my_self
        .get_str("name")
        .context(MongoInfoError::ReplicaSetStatusInvalidSelf)?
        .to_string();
    //  - Current node optime (as Commit Offset).
    let optime = my_self
        .get_datetime("optimeDate")
        .context(MongoInfoError::ReplicaSetStatusInvalidSelf)?
        .timestamp_millis();
    //  - Replica Set member state (as Role).
    let role = my_self
        .get_i32("state")
        .context(MongoInfoError::ReplicaSetStatusInvalidSelf)?;
    let role = MemberState::try_from(role)?;
    let role = ShardRole::from(role);
    //  - Delta between primary node and current member.
    let lag = if let Some(primary) = primary {
        let primary_optime = primary
            .get_datetime("optimeDate")
            .context(MongoInfoError::ReplicaSetStatusInvalidSelf)?
            .timestamp_millis();
        let lag = ShardCommitOffset::milliseconds(primary_optime - optime);
        Some(lag)
    } else {
        None
    };

    Ok(Shard {
        commit_offset: ShardCommitOffset::milliseconds(optime),
        lag,
        role,
        shard_id,
    })
}
