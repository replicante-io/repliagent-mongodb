//! Possible errors encountered by the agent.

/// Errors related to the [MongoDB Client](mongodb::Client).
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// The configured node address is not valid.
    ///
    /// Error parameters:
    ///
    /// - The node address that caused the error.
    #[error("the configured node address is not valid: '{0}'")]
    AddressNotValid(String),

    /// Unable to create a MongoDB client
    #[error("unable to create a MongoDB client")]
    CreateFailed,
}

impl ClientError {
    /// The configured node address is not valid.
    pub fn address_not_valid<S: Into<String>>(address: S) -> Self {
        Self::AddressNotValid(address.into())
    }
}

/// Errors related to loading or validating agent configuration.
#[derive(Debug, thiserror::Error)]
pub enum ConfError {
    /// The node cluster address is missing from both configuration and environment.
    #[error("the node cluster address is missing from both configuration and environment")]
    NoClusterAddress,
}

/// Unrecognised member state code.
#[derive(Debug, thiserror::Error)]
#[error("unrecognised member state code {state}")]
pub struct MemberStateParseError {
    state: i32,
}

impl From<i32> for MemberStateParseError {
    fn from(state: i32) -> Self {
        MemberStateParseError { state }
    }
}

/// Possible errors while gathering node information.
#[derive(Debug, thiserror::Error)]
pub enum MongoInfoError {
    /// Output of feature compatibility version command does not include a version.
    #[error("output of feature compatibility version command does not include a version")]
    FeatCompatVerNotSet,

    /// Get feature compatibility version command failed.
    #[error("get feature compatibility version command failed")]
    FeatCompatVerUnknown,

    /// Output of the oplog collection stats command does not include a collection size.
    #[error("output of the oplog collection stats command does not include a collection size")]
    OplogStatsNoSize,

    /// Get oplog collection statistics command failed.
    #[error("get oplog collection statistics command failed")]
    OplogStatsUnknown,

    /// Self member in the output of the replica set status command is invalid.
    #[error("self member in the output of the replica set status command is invalid")]
    ReplicaSetStatusInvalidSelf,

    /// Output of the replica set status command does not include a members list.
    #[error("output of the replica set status command does not include a members list")]
    ReplicaSetStatusNoMembers,

    /// Output of the replica set status command does not include a set name.
    #[error("output of the replica set status command does not include a set name")]
    ReplicaSetStatusNoName,

    /// Output of the replica set status command does not include the node itself.
    #[error("output of the replica set status command does not include the node itself")]
    ReplicaSetStatusNoSelf,

    /// Get replica set status command failed.
    #[error("get replica set status command failed")]
    ReplicaSetStatusUnknown,
}
