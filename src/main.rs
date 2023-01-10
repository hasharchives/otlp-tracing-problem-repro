#![forbid(unsafe_code)]

use std::io;

use opentelemetry::{
    global,
    sdk::{
        propagation::TraceContextPropagator,
        trace::{RandomIdGenerator, Sampler, Tracer},
        Resource,
    },
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use tonic::metadata::MetadataMap;

use tracing::{span, Level};
use tracing_error::SpanTrace;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{
    filter::{Directive, LevelFilter},
    fmt::writer::BoxMakeWriter,
    layer::{Layered, SubscriberExt},
    EnvFilter, Registry,
};

#[tokio::main]
async fn main() {
    // let _log_guard = init_logger();
    let filter = std::env::var("RUST_LOG").map_or_else(
        |_| EnvFilter::default().add_directive(Directive::from(LevelFilter::TRACE)),
        EnvFilter::new,
    );

    let otlp_endpoint = "http://localhost:4317";

    let opentelemetry_layer = configure_opentelemetry_layer(otlp_endpoint);
    let output_writer = BoxMakeWriter::new(io::stderr);
    let output_layer = tracing_subscriber::fmt::layer()
        .with_ansi(true)
        .with_writer(output_writer);

    let error_layer = tracing_error::ErrorLayer::default();

    tracing_subscriber::registry()
        .with(filter)
        .with(opentelemetry_layer)
        .with(output_layer)
        .with(error_layer)
        .try_init()
        .unwrap();

    let span = span!(Level::INFO, "some_span");
    let _entered = span.enter();

    let captrued_span = SpanTrace::capture();
    // Uncomment to let run to completion
    // tracing::error!("Some error trace!");
    tracing::error!(error=?captrued_span, "Some error trace!");
}

fn configure_opentelemetry_layer(
    otlp_endpoint: &str,
) -> OpenTelemetryLayer<Layered<EnvFilter, Registry>, Tracer> {
    // Allow correlating trace IDs
    global::set_text_map_propagator(TraceContextPropagator::new());
    // If we need to set any tokens in the header for the tracing collector, this would be the place
    // we do so.
    let map = MetadataMap::new();

    let pipeline = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(otlp_endpoint)
        .with_metadata(map);

    let trace_config = opentelemetry::sdk::trace::config()
        .with_sampler(Sampler::AlwaysOn)
        .with_id_generator(RandomIdGenerator::default())
        .with_resource(Resource::new(vec![KeyValue::new("service.name", "test")]));

    // The tracer batch sends traces asynchronously instead of per-span.
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(pipeline)
        .with_trace_config(trace_config)
        .install_batch(opentelemetry::runtime::Tokio)
        .expect("failed to create OTLP tracer, check configuration values");

    tracing_opentelemetry::layer().with_tracer(tracer)
}
