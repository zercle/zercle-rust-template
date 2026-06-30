//! Tracing + OpenTelemetry + Prometheus initialization.
//!
//! Mirrors `internal/shared/telemetry/{logger,tracer,meter}.go` from the Go template (see
//! `structure.md` §7 and `canvas.md ## Assumptions` rows 10–12). Behavior:
//!
//! * A single [`tracing_subscriber::Registry`] combines an `EnvFilter` (from `cfg.log.level`),
//!   a `fmt` layer (json or pretty) and, when `cfg.otel.exporter == "otlp"`, an
//!   `OpenTelemetryLayer` backed by an OTLP HTTP trace exporter.
//! * When `cfg.otel.exporter == "none"`, no tracer is installed and
//!   [`Telemetry::tracer_provider`] is `None`; a no-op `SdkMeterProvider` and a fresh
//!   Prometheus registry are still returned so `/metrics` is always served.
//! * The Prometheus exporter attached to the meter provider writes into
//!   [`Telemetry::prometheus_registry`]; the `/metrics` handler renders that registry via
//!   [`metrics_body`].

use std::sync::OnceLock;

use opentelemetry::{KeyValue, global, trace::TracerProvider as _};
use opentelemetry_otlp::{SpanExporter, WithExportConfig};
use opentelemetry_sdk::{
    Resource,
    metrics::SdkMeterProvider,
    trace::{Sampler, TracerProvider},
};
use prometheus::{Encoder, Registry, TextEncoder};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, prelude::*};

use crate::config::Config;

/// Errors raised by [`init`]. Kept narrow so callers can convert into the shared
/// `AppError::Internal` without losing the cause chain.
#[derive(Debug, thiserror::Error)]
pub enum TelemetryError {
    #[error("OTEL_EXPORTER_OTLP_ENDPOINT is required when OTEL_EXPORTER=otlp")]
    MissingEndpoint,
    #[error("invalid OTLP trace endpoint {0}: {1}")]
    InvalidEndpoint(String, url::ParseError),
    #[error("build OTLP trace exporter: {0}")]
    BuildExporter(#[source] opentelemetry::trace::TraceError),
    #[error("build Prometheus exporter: {0}")]
    BuildPrometheus(#[source] opentelemetry_sdk::metrics::MetricError),
}

/// Handle returned by [`init`]; passed to [`shutdown`].
#[derive(Debug)]
pub struct Telemetry {
    /// OTLP tracer provider (`Some` when `cfg.otel.exporter == "otlp"`, otherwise `None`).
    pub tracer_provider: Option<TracerProvider>,
    /// Meter provider (always present so `/metrics` is served even with no OTLP exporter).
    pub meter_provider: SdkMeterProvider,
    /// Prometheus registry the `/metrics` handler encodes.
    pub prometheus_registry: Registry,
}

static GLOBAL_DISPATCH: OnceLock<()> = OnceLock::new();

/// Initialize tracing + (optionally) OTLP traces + Prometheus metrics from `cfg`.
///
/// Also installs the global tracing dispatch. Safe to call more than once: if a global dispatch
/// has already been set (typical in tests), the call logs a warning and proceeds without
/// re-installing.
pub fn init(cfg: &Config) -> Result<Telemetry, TelemetryError> {
    let env_filter = EnvFilter::try_new(&cfg.log.level).unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = if cfg.log.format == "json" {
        fmt::layer().json().boxed()
    } else {
        fmt::layer().boxed()
    };

    let (tracer_provider, registry_with_otel) = build_tracer(cfg)?;

    let (meter_provider, prometheus_registry) = build_meter()?;

    // Install the global dispatch (best-effort idempotent — see contract on `init`).
    if let Some(tracer_provider) = tracer_provider.as_ref() {
        let tracer = tracer_provider.tracer(cfg.otel.service_name.clone());
        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
        let subscriber = registry_with_otel
            .with(env_filter)
            .with(fmt_layer)
            .with(otel_layer);
        install_global(subscriber);
        // Make the SDK's global tracer point at the new provider so anything that uses
        // `tracing` macros outside of the `tracing-opentelemetry` layer still produces OTel
        // spans when an explicit tracer is requested via `global::tracer(...)`.
        global::set_tracer_provider(tracer_provider.clone());
    } else {
        let subscriber = registry_with_otel.with(env_filter).with(fmt_layer);
        install_global(subscriber);
        // Install a no-op tracer provider so `global::tracer(...)` never panics.
        global::set_tracer_provider(TracerProvider::default());
    }

    // Hold the global meter provider as the process-wide default so application code that uses
    // the `opentelemetry::global` API records into the same registry.
    global::set_meter_provider(meter_provider.clone());

    Ok(Telemetry {
        tracer_provider,
        meter_provider,
        prometheus_registry,
    })
}

/// Resolve the OTLP traces endpoint, mirroring Go `tracer.go`'s `/v1/traces` suffix logic.
fn resolve_traces_endpoint(endpoint: &str) -> Result<String, TelemetryError> {
    let trimmed = endpoint.trim();
    if trimmed.is_empty() {
        return Err(TelemetryError::MissingEndpoint);
    }
    let normalized = if trimmed.ends_with("/v1/traces") {
        trimmed.to_string()
    } else {
        format!("{}/v1/traces", trimmed.trim_end_matches('/'))
    };
    // Validate by parsing — the otel exporter accepts opaque URLs but we want a fail-fast.
    url::Url::parse(&normalized)
        .map_err(|e| TelemetryError::InvalidEndpoint(normalized.clone(), e))?;
    Ok(normalized)
}

/// Build (and register) the tracer provider; return the base registry to chain layers onto.
fn build_tracer(
    cfg: &Config,
) -> Result<(Option<TracerProvider>, tracing_subscriber::Registry), TelemetryError> {
    let registry = tracing_subscriber::registry();

    if cfg.otel.exporter == "none" {
        return Ok((None, registry));
    }

    let endpoint = resolve_traces_endpoint(&cfg.otel.endpoint)?;
    let exporter = SpanExporter::builder()
        .with_http()
        .with_endpoint(endpoint)
        .build()
        .map_err(TelemetryError::BuildExporter)?;

    let resource = Resource::new(vec![KeyValue::new(
        "service.name",
        cfg.otel.service_name.clone(),
    )]);
    let sampler = Sampler::TraceIdRatioBased(cfg.otel.sampling);

    let provider = TracerProvider::builder()
        .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
        .with_resource(resource)
        .with_sampler(sampler)
        .build();

    Ok((Some(provider), registry))
}

/// Build the Prometheus-backed meter provider and its registry.
fn build_meter() -> Result<(SdkMeterProvider, Registry), TelemetryError> {
    let registry = Registry::new();
    let exporter = opentelemetry_prometheus::exporter()
        .with_registry(registry.clone())
        .build()
        .map_err(TelemetryError::BuildPrometheus)?;
    let provider = SdkMeterProvider::builder().with_reader(exporter).build();
    Ok((provider, registry))
}

/// Install the global tracing dispatch exactly once. On subsequent calls, log a warning and
/// continue — the requested dispatch is constructed (and therefore validated) but not installed.
fn install_global<S>(subscriber: S)
where
    S: tracing::Subscriber + Send + Sync + 'static,
{
    if GLOBAL_DISPATCH.set(()).is_ok() {
        if let Err(e) = tracing::subscriber::set_global_default(subscriber) {
            // Another test/install raced us between the `set` and the install (effectively
            // impossible in practice, but keep the warning in case the OnceLock semantics ever
            // shift).
            tracing::warn!(error = %e, "telemetry: failed to install global tracing dispatch");
        }
    } else {
        tracing::warn!("telemetry: global tracing dispatch already set; skipping install");
    }
}

/// Flush + shutdown providers, best-effort. Errors are logged at `error` level.
pub fn shutdown(telemetry: Telemetry) {
    if let Some(tracer_provider) = telemetry.tracer_provider {
        for result in tracer_provider.force_flush() {
            if let Err(e) = result {
                tracing::error!(error = %e, "telemetry: tracer force_flush failed");
            }
        }
        if let Err(e) = tracer_provider.shutdown() {
            tracing::error!(error = %e, "telemetry: tracer shutdown failed");
        }
    }
    if let Err(e) = telemetry.meter_provider.force_flush() {
        tracing::error!(error = %e, "telemetry: meter force_flush failed");
    }
    if let Err(e) = telemetry.meter_provider.shutdown() {
        tracing::error!(error = %e, "telemetry: meter shutdown failed");
    }

    // Shut down the process-wide global tracer provider registered via
    // `opentelemetry::global::set_tracer_provider` in `init`, releasing its
    // resources and flushing any remaining batched exports. (The global meter
    // provider in opentelemetry 0.27 has no top-level shutdown; the local
    // `telemetry.meter_provider.shutdown()` above is the only knob.)
    opentelemetry::global::shutdown_tracer_provider();
}

/// Render the Prometheus text exposition format for the `/metrics` endpoint.
pub fn metrics_body(registry: &Registry) -> String {
    let encoder = TextEncoder::new();
    let metric_families = registry.gather();
    let mut buffer = Vec::with_capacity(1024);
    if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
        tracing::error!(error = %e, "telemetry: prometheus encode failed");
        return String::new();
    }
    String::from_utf8(buffer).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_cfg(exporter: &str, endpoint: &str) -> Config {
        // Use a fresh yaml for each call so tests don't depend on disk state.
        let yaml = format!(
            r#"
app:
  name: test-svc
  environment: development
  host: 0.0.0.0
  port: 8080
  shutdown_timeout: 15
http:
  host: 0.0.0.0
  port: 8080
  read_timeout: 15
  write_timeout: 15
  idle_timeout: 60
  body_limit: "1M"
  health_probe_timeout: 5
grpc:
  host: 0.0.0.0
  port: 50051
db:
  host: localhost
  port: 5432
  name: app
  user: postgres
  password: postgres
  ssl_mode: disable
  max_conns: 10
  min_conns: 2
  max_conn_idle: 1800
  max_conn_life: 3600
  connect_timeout: 5
valkey:
  host: localhost
  port: 6379
  password: ""
  db: 0
  connect_timeout: 5
otel:
  exporter: {exporter}
  endpoint: "{endpoint}"
  service_name: test-svc
  sampling: 1.0
log:
  level: info
  format: json
example:
  enabled: true
  default_page_size: 20
  max_page_size: 100
  max_name_length: 255
"#
        );
        let settings = ::config::Config::builder()
            .add_source(::config::File::from_str(&yaml, ::config::FileFormat::Yaml))
            .build()
            .expect("yaml builds");
        settings.try_deserialize::<Config>().expect("deserialize")
    }

    #[test]
    fn init_with_none_returns_no_tracer_and_usable_registry() {
        let cfg = sample_cfg("none", "");
        let t = init(&cfg).expect("init with none must succeed");
        assert!(t.tracer_provider.is_none(), "no OTLP exporter → no tracer");
        // registry is usable: gathering a fresh one must not panic.
        let _ = t.prometheus_registry.gather();
        // metrics_body must not panic and must return a String.
        let body = metrics_body(&t.prometheus_registry);
        let _ = body.len();
        shutdown(t);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn init_with_otlp_returns_tracer_provider() {
        // The OTLP HTTP exporter constructs a `BatchSpanProcessor` on the Tokio runtime, so
        // this test must run inside one (multi-thread, since the batch processor's worker
        // task needs its own scheduler). Use an unroutable but well-formed endpoint — the
        // builder succeeds without a live collector (the exporter only opens a connection on
        // the first batch export).
        let cfg = sample_cfg("otlp", "http://127.0.0.1:65535");
        let t = init(&cfg).expect("init with otlp must succeed (lazy exporter)");
        assert!(
            t.tracer_provider.is_some(),
            "otlp exporter → tracer provider"
        );
        let _ = metrics_body(&t.prometheus_registry);
        shutdown(t);
    }

    #[test]
    fn init_rejects_otlp_with_empty_endpoint() {
        let cfg = sample_cfg("otlp", "");
        let err = init(&cfg).expect_err("empty endpoint must fail");
        assert!(matches!(err, TelemetryError::MissingEndpoint));
    }

    #[test]
    fn init_rejects_otlp_with_malformed_endpoint() {
        let cfg = sample_cfg("otlp", "not a url");
        let err = init(&cfg).expect_err("malformed endpoint must fail");
        assert!(matches!(err, TelemetryError::InvalidEndpoint(..)));
    }

    #[test]
    fn resolve_traces_endpoint_appends_v1_traces() {
        let r = resolve_traces_endpoint("http://localhost:4318").expect("ok");
        assert_eq!(r, "http://localhost:4318/v1/traces");

        let r2 = resolve_traces_endpoint("http://localhost:4318/").expect("ok");
        assert_eq!(r2, "http://localhost:4318/v1/traces");

        let r3 = resolve_traces_endpoint("http://localhost:4318/v1/traces").expect("ok");
        assert_eq!(r3, "http://localhost:4318/v1/traces");
    }

    #[test]
    fn metrics_body_on_empty_registry_returns_string_and_does_not_panic() {
        let registry = Registry::new();
        let body = metrics_body(&registry);
        // An empty registry produces the Prometheus "no metrics" output (still valid text);
        // we only require that it does not panic and decodes as UTF-8.
        let _ = body.len();
    }

    #[test]
    fn shutdown_is_safe_with_no_tracer() {
        let cfg = sample_cfg("none", "");
        let t = init(&cfg).expect("init");
        // Must not panic even when there's no tracer provider to flush.
        shutdown(t);
    }

    #[test]
    fn init_can_be_called_repeatedly() {
        // Each subsequent call should log a warning (since the OnceLock guards the global
        // install) but must not panic. Tests intentionally run `init` more than once to mirror
        // the per-test pattern used across this crate.
        let cfg = sample_cfg("none", "");
        let t1 = init(&cfg).expect("first init");
        let t2 = init(&cfg).expect("second init");
        shutdown(t1);
        shutdown(t2);
    }
}
