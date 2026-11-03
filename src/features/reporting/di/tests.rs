use super::*;
use crate::platform::errors::errcodes;
use sqlx::postgres::PgPoolOptions;

fn lazy_pool() -> PgPool {
    PgPoolOptions::new()
        .connect_lazy("postgres://localhost/none")
        .expect("lazy pool")
}

fn cfg_enabled(enabled: bool) -> Config {
    let yaml = format!(
        r#"
app: {{ name: t, environment: dev, host: 0.0.0.0, port: 8080 }}
http: {{ host: 0.0.0.0, port: 8080, body_limit: "1M" }}
grpc: {{ host: 0.0.0.0, port: 50051 }}
db: {{ host: localhost, port: 5432, name: app, user: postgres, password: postgres, ssl_mode: disable, max_conns: 1, min_conns: 0 }}
valkey: {{ host: localhost, port: 6379, db: 0 }}
otel: {{ exporter: none, service_name: t, sampling: 1.0 }}
log: {{ level: info, format: json }}
reporting: {{ enabled: {enabled} }}
"#
    );
    ::config::Config::builder()
        .add_source(::config::File::from_str(&yaml, ::config::FileFormat::Yaml))
        .build()
        .expect("build")
        .try_deserialize()
        .expect("deserialize")
}

#[test]
fn domain_sentinel_maps_to_invalid_input() {
    assert_eq!(
        AppError::from(Error::InvalidTopMachines).code(),
        errcodes::INVALID_INPUT
    );
}

#[test]
fn domain_internal_maps_to_internal() {
    assert_eq!(
        AppError::from(Error::Internal { cause: None }).code(),
        errcodes::INTERNAL
    );
}

#[tokio::test]
async fn disabled_feature_registers_nothing() {
    let wired = register(&cfg_enabled(false), lazy_pool());
    assert!(!wired.http.has_routes());
}

#[tokio::test]
async fn enabled_feature_registers_summary_route() {
    let wired = register(&cfg_enabled(true), lazy_pool());
    assert!(
        wired.http.has_routes(),
        "enabled reporting mounts GET /api/v1/reports/summary"
    );
}
