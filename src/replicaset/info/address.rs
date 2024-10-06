//! Lookup node addresses.
use anyhow::Context;
use anyhow::Result;

use replisdk::agent::framework::constants::ENV_NODE_ADDR_MEMBER;
use replisdk::agent::models::NodeAddresses;

/// Detect replica set addresses to reach the node.
pub fn detect() -> Result<NodeAddresses> {
    let member = std::env::var(ENV_NODE_ADDR_MEMBER)
        .context(crate::errors::ConfError::NoNodeMemberAddress)?;
    let address = NodeAddresses {
        client: Some(member.clone()),
        member: Some(member),
        other: Default::default(),
    };
    Ok(address)
}
