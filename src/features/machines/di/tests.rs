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
machines: {{ enabled: {enabled} }}
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
fn domain_sentinels_map_to_boundary_codes() {
    let code = |e: Error| AppError::from(e).code().to_string();
    assert_eq!(code(Error::MachineNotFound), errcodes::NOT_FOUND);
    assert_eq!(code(Error::InvalidId), errcodes::INVALID_INPUT);
    assert_eq!(code(Error::InvalidMachineLabel), errcodes::INVALID_INPUT);
    assert_eq!(code(Error::UnsupportedCoin), errcodes::INVALID_INPUT);
}

#[test]
fn migrations_own_version_two() {
    let m = migrations();
    assert!(m.iter().any(|m| m.version == 2), "machines owns version 2");
}

#[tokio::test]
async fn disabled_feature_registers_nothing() {
    let wired = register(&cfg_enabled(false), lazy_pool());
    assert!(!wired.http.has_routes());
}

#[tokio::test]
async fn enabled_feature_registers_routes() {
    let wired = register(&cfg_enabled(true), lazy_pool());
    assert!(
        wired.http.has_routes(),
        "enabled feature mounts its /api/v1 routes"
    );
}
