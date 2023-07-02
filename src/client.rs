//! MongoDB client configuration and initialisation.
use std::sync::RwLock;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use mongodb::options::ClientOptions;
use mongodb::options::ServerAddress;
use mongodb::Client;
use once_cell::sync::Lazy;

use crate::conf::Conf;
use crate::conf::Tls;
use crate::errors::ClientError;

/// Name passed to MongoDB server from the client.
const MONGO_CLIENT_APP_NAME: &str = "repliagent-mongo";

/// Singleton client to interact with MongoDB.
static GLOBAL_CLIENT: Lazy<RwLock<Option<Client>>> = Lazy::new(|| RwLock::new(None));

/// Initialise a MongoDB client and set it as the process default.
///
/// # Panics
///
/// initialisation panics if a client has already been initialised.
pub fn initialise(conf: &Conf) -> Result<Client> {
    // Obtain a lock to initialise the global client.
    let mut global_client = GLOBAL_CLIENT
        .write()
        .expect("GLOBAL_CLIENT RwLock poisoned");

    // If the global client is already initialised panic (without poisoning the lock).
    if global_client.is_some() {
        drop(global_client);
        panic!("MongoDB client already initialised");
    }

    // Initialise the client and, on success update the global client.
    let client = connect(conf)?;
    *global_client = Some(client.clone());
    Ok(client)
}

/// Get the globally initialised MongoDB client.
// TODO: remove if unused by the time actions are defined.
#[allow(dead_code)]
pub fn global() -> Client {
    GLOBAL_CLIENT
        .read()
        .expect("GLOBAL_CLIENT RwLock poisoned")
        .as_ref()
        .expect("global MongoDB client is not initialised")
        .clone()
}

/// Create a new MongoDC client connected to a specific node.
fn connect(conf: &Conf) -> Result<Client> {
    let server = ServerAddress::parse(&conf.node_address)
        .with_context(|| ClientError::address_not_valid(&conf.node_address))?;
    let options = ClientOptions::builder()
        .app_name(MONGO_CLIENT_APP_NAME.to_string())
        // Ensure we connect directly and exclusively to our corresponding node.
        .direct_connection(true)
        .hosts(vec![server])
        // As we local connections only long server selection timeouts hurt us.
        .server_selection_timeout(Duration::from_millis(500))
        // Additional client options.
        .connect_timeout(conf.connection_timeout.map(std::time::Duration::from_secs))
        .credential(
            conf.credentials
                .clone()
                .map(mongodb::options::Credential::from),
        )
        .heartbeat_freq(conf.heartbeat_frequency.map(std::time::Duration::from_secs))
        .max_idle_time(conf.max_idle_time.map(std::time::Duration::from_secs))
        .tls(Tls::into_client_option(&conf.tls))
        .build();
    Client::with_options(options).context(ClientError::CreateFailed)
}
