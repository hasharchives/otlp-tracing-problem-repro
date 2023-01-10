#![forbid(unsafe_code)]

use std::io;
use std::{fmt, net::SocketAddr};

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

use axum::{http::StatusCode, routing::get, Router};
use error_stack::{Context, Report};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{
    filter::{Directive, LevelFilter},
    fmt::writer::BoxMakeWriter,
    layer::{Layered, SubscriberExt},
    util::TryInitError,
    EnvFilter, Registry,
};

#[derive(Debug)]
pub struct SomeError;
impl Context for SomeError {}

impl fmt::Display for SomeError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str("oh no! An error")
    }
}

async fn make_error() -> std::result::Result<&'static str, StatusCode> {
    Err(Report::new(SomeError)).map_err(|report| {
        // Any kind of tracing will make the task to spin forever.
        // But commenting out both will allow the program to run.

        // This line below behaves the same as the `error!`
        // tracing::info!(error=?report, "Some error trace!");
        tracing::error!(error=?report, "Some error trace!");

        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok("success")
}

#[tokio::main]
async fn main() {
    let _log_guard = init_logger();

    let app = Router::new().route("/", get(make_error));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

pub fn init_logger() -> Result<(), TryInitError> {
    let otlp_endpoint = "http://localhost:4317";

    let filter = std::env::var("RUST_LOG").map_or_else(
        |_| EnvFilter::default().add_directive(Directive::from(LevelFilter::WARN)),
        EnvFilter::new,
    );

    let opentelemetry_layer = configure_opentelemetry_layer(otlp_endpoint);
    let error_layer = tracing_error::ErrorLayer::default();

    let output_writer = BoxMakeWriter::new(io::stderr);
    let output_layer = tracing_subscriber::fmt::layer()
        .with_ansi(true)
        .with_writer(output_writer);

    let console_layer = console_subscriber::spawn();

    tracing_subscriber::registry()
        .with(filter)
        // The halting issues stop when:
        // commenting out the OpenTelemetry layer
        .with(opentelemetry_layer)
        // or commenting out the error layer
        .with(error_layer)
        .with(output_layer)
        .with(console_layer)
        .try_init()?;

    Ok(())
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
