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

/// Unrecognised member state code.
#[derive(Debug, thiserror::Error)]
#[error("unrecognised member state code {state}")]
pub struct MemberStateParseError {
    state: i64,
}

impl From<i64> for MemberStateParseError {
    fn from(state: i64) -> Self {
        MemberStateParseError { state }
    }
}
