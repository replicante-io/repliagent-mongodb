//! Functions to handle admin commands against MongoDB.
use std::future::IntoFuture;

use mongodb::bson::Document;
use mongodb::error::Error;
use mongodb::error::ErrorKind;
use mongodb::error::Result as MdbResult;
use mongodb::Client;
use opentelemetry::trace::FutureExt;

use replisdk::utils::trace::TraceFutureStdErrExt;

use crate::constants::CMD_REPL_SET_GET_STATUS;
use crate::constants::DB_ADMIN;
use crate::constants::REPL_SET_NOT_INITIALISED;

/// Run the replSetGetStatus command against the DB.
///
/// ## Errors Telemetry
///
/// This function does not report errors from the MongoDB server as error
/// as part of the generated telemetry data.
/// This is because some failures are actually expected states (replica set no initialised).
///
/// If error information from this function should be attached to telemetry data then
/// it should be done by the caller.
pub async fn replica_set_status(client: &Client) -> MdbResult<Document> {
    let trace = crate::trace::mongodb_client_context(CMD_REPL_SET_GET_STATUS);
    let (_, _timer) = crate::metrics::observe_mongodb_op(CMD_REPL_SET_GET_STATUS);

    let command = {
        let mut command = Document::new();
        command.insert(CMD_REPL_SET_GET_STATUS, 1);
        command
    };
    let admin = client.database(DB_ADMIN);
    admin
        .run_command(command)
        .into_future()
        .trace_on_err()
        .with_context(trace.clone())
        .await
}

/// Check [`replica_set_status`]'s errors to see if the Replica Set is not initialised.
///
/// This function only returns true if the error indicated the replica set is NOT initialised.
/// Other errors do not mean the replica set is initialised but we can't be sure it is not.
pub fn replica_set_not_initialised(error: &Error) -> bool {
    if let ErrorKind::Command(ref inner) = *error.kind {
        return inner.code == REPL_SET_NOT_INITIALISED;
    }
    false
}
