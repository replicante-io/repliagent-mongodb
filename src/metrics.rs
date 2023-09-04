//! Definition of metrics exposed by the MongoDB agent.
use anyhow::Result;
use once_cell::sync::Lazy;
use prometheus::Counter;
use prometheus::CounterVec;
use prometheus::HistogramOpts;
use prometheus::HistogramTimer;
use prometheus::HistogramVec;
use prometheus::Opts;

use replisdk::agent::framework::InitialiseHook;
use replisdk::agent::framework::InitialiseHookArgs;

use crate::conf::Conf;

/// Duration (in seconds) of MongoDB operations issued to the server.
pub static MONGODB_OPS_DURATION: Lazy<HistogramVec> = Lazy::new(|| {
    HistogramVec::new(
        HistogramOpts::new(
            "repliagent_mongodb_operations_duration",
            "Duration (in seconds) of MongoDB operations issued to the server",
        )
        // Buckers: start = 1, next = prev + (idx) * 0.5
        .buckets(vec![1.0, 1.5, 2.5, 4.0, 6.0, 8.5, 11.5, 15.0]),
        &["op"],
    )
    .expect("failed to initialise MONGODB_OPS_DURATION histogram")
});

/// Number of MongoDB operations the server returned an error for.
pub static MONGODB_OPS_ERR: Lazy<CounterVec> = Lazy::new(|| {
    CounterVec::new(
        Opts::new(
            "repliagent_mongodb_operations_error",
            "Number of MongoDB operations the server returned an error for",
        ),
        &["op"],
    )
    .expect("failed to initialise MONGODB_OPS_ERR counter")
});

/// Initialisation hook to register agent metrics.
pub struct Register;

#[async_trait::async_trait]
impl InitialiseHook for Register {
    type Conf = Conf;
    async fn initialise<'a>(&self, args: &InitialiseHookArgs<'a, Self::Conf>) -> Result<()> {
        let collectors: [Box<dyn prometheus::core::Collector>; 2] = [
            Box::new(MONGODB_OPS_DURATION.clone()),
            Box::new(MONGODB_OPS_ERR.clone()),
        ];
        for collector in collectors {
            args.telemetry.metrics.register(collector)?;
        }
        Ok(())
    }
}

/// Observe the execution of a MongoDB server operation.
///
/// ## Returns
///
/// - A started timer to observe the duration of the operation.
/// - A [`Counter`] to increment in case of error.
#[inline]
pub fn observe_mongodb_op(op: &str) -> (Counter, HistogramTimer) {
    let err_count = MONGODB_OPS_ERR.with_label_values(&[op]);
    let timer = MONGODB_OPS_DURATION.with_label_values(&[op]).start_timer();
    (err_count, timer)
}
