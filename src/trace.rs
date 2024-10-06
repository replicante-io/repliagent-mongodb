//! Tools to instrument the MongoDB agent with tracing data.
use opentelemetry::trace::SpanKind;
use opentelemetry::trace::TraceContextExt;
use opentelemetry::trace::Tracer;
use opentelemetry::Context;

/// Initialised a new span and context for MongoDB client operations,
///
/// The new span and context are automatically children of the active span and context.
pub fn mongodb_client_context(op: &str) -> Context {
    let op = format!("mongodb.{}", op);
    let tracer = opentelemetry::global::tracer(env!("CARGO_PKG_NAME"));
    let mut builder = tracer.span_builder(op);
    builder.span_kind = Some(SpanKind::Client);
    let parent = Context::current();
    let span = tracer.build_with_context(builder, &parent);
    parent.with_span(span)
}
