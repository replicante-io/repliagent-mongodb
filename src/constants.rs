//! MongoDB Server constants
use std::convert::TryFrom;

use replisdk::agent::models::ShardRole;

use crate::errors::MemberStateParseError;

/// Prefix for MongoDB attributes.
pub const ATTRIBUTE_PREFIX: &str = "mongodb.com";

/// MongoDB command to get server command line and configuration.
pub const CMD_GET_CMD_LINE_OPTS: &str = "getCmdLineOpts";

/// MongoDB command to get server parameters.
pub const CMD_GET_PARAMETER: &str = "getParameter";

/// MongoDB command to get collection statistics.
pub const CMD_COLL_STATS: &str = "collStats";

/// MongoDB command to get the current Replica Set configuration.
pub const CMD_REPL_SET_GET_CONFIG: &str = "replSetGetConfig";

/// MongoDB command to get the current Replica Set status.
pub const CMD_REPL_SET_GET_STATUS: &str = "replSetGetStatus";

/// MongoDB command to initialise a new Replica Set.
pub const CMD_REPL_SET_INIT: &str = "replSetInitiate";

/// MongoDB command to get the current Replica Set configuration.
pub const CMD_REPL_SET_RECONFIG: &str = "replSetReconfig";

/// Name of the database to run admin commands against (also known as the admin database).
pub const DB_ADMIN: &str = "admin";

/// Name of the database with local state on (also known as the local database).
pub const DB_LOCAL: &str = "local";

/// Parameter to the [`CMD_GET_PARAMETER`] command for retrieving the current FCV.
pub const FEATURE_COMPATIBILITY_VERSION: &str = "featureCompatibilityVersion";

/// Error code returned by MongoDB when the Replica Set is not initialised no the node.
pub const REPL_SET_NOT_INITIALISED: i32 = 94;

/// Possible states of a MongoDB replica set member.
///
/// <https://www.mongodb.com/docs/manual/reference/replica-states/>
#[derive(Clone, Debug)]
pub enum MemberState {
    Startup = 0,
    Primary = 1,
    Secondary = 2,
    Recovering = 3,
    Startup2 = 5,
    Unknown = 6,
    Arbiter = 7,
    Down = 8,
    Rollback = 9,
    Removed = 10,
}

impl std::fmt::Display for MemberState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemberState::Arbiter => write!(f, "ARBITER"),
            MemberState::Down => write!(f, "DOWN"),
            MemberState::Primary => write!(f, "PRIMARY"),
            MemberState::Recovering => write!(f, "RECOVERING"),
            MemberState::Removed => write!(f, "REMOVED"),
            MemberState::Rollback => write!(f, "ROLLBACK"),
            MemberState::Secondary => write!(f, "SECONDARY"),
            MemberState::Startup => write!(f, "STARTUP"),
            MemberState::Startup2 => write!(f, "STARTUP2"),
            MemberState::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

impl TryFrom<i32> for MemberState {
    type Error = MemberStateParseError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        let value = match value {
            0 => MemberState::Startup,
            1 => MemberState::Primary,
            2 => MemberState::Secondary,
            3 => MemberState::Recovering,
            5 => MemberState::Startup2,
            6 => MemberState::Unknown,
            7 => MemberState::Arbiter,
            8 => MemberState::Down,
            9 => MemberState::Rollback,
            10 => MemberState::Removed,
            value => return Err(MemberStateParseError::from(value)),
        };
        Ok(value)
    }
}

impl From<MemberState> for ShardRole {
    fn from(value: MemberState) -> Self {
        match value {
            MemberState::Primary => ShardRole::Primary,
            MemberState::Secondary => ShardRole::Secondary,
            MemberState::Recovering | MemberState::Startup | MemberState::Startup2 => {
                Self::Recovering
            }
            other => ShardRole::Other(other.to_string()),
        }
    }
}
