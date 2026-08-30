//! Valkey (Redis-protocol) connection manager + readiness checker.
//!
//! Mirrors `internal/infrastructure/messaging/valkey/{client,health}.go` (structure.md §11).
//!
//! * [`new_client`] builds a [`redis::aio::ConnectionManager`] (clone-cheap, `Send + Sync`)
//!   from [`Config`], pings the server, and returns the manager. On ping failure the manager is
//!   closed before returning the error.
//! * [`ValkeyChecker`] implements [`shared::health::Checker`] and PINGs the server for readiness.

use anyhow::{Context, Result};
use async_trait::async_trait;
use redis::{Client, aio::ConnectionManager};

use crate::{platform::config::Config, platform::health::Checker};

/// Build the Valkey/Redis connection URL from `cfg`.
///
/// Format: `redis://[:password@]host:port/db` (matches `cfg.valkey_addr()` shape).
/// The password is URL-encoded because production Valkey/Redis passwords often
/// contain reserved URL characters (`@`, `:`, `#`, `/`, ...) that would otherwise
/// break URL parsing or authenticate incorrectly.
fn build_url(cfg: &Config) -> String {
    let mut url = url::Url::parse("redis://localhost").expect("valid base url");
    let _ = url.set_host(Some(&cfg.valkey.host));
    let _ = url.set_port(Some(cfg.valkey.port));
    if !cfg.valkey.password.is_empty() {
        let _ = url.set_password(Some(&cfg.valkey.password));
    }
    url.set_path(&cfg.valkey.db.to_string());
    url.to_string()
}

/// Build a connected [`ConnectionManager`] from `cfg` and PING it before returning.
pub async fn new_client(cfg: &Config) -> Result<ConnectionManager> {
    let url = build_url(cfg);
    let client =
        Client::open(url.as_str()).with_context(|| format!("create redis client for {url}"))?;

    let manager = ConnectionManager::new(client)
        .await
        .with_context(|| format!("create connection manager for {url}"))?;

    // PING round-trip to verify the connection is live before we hand it to AppState.
    let mut conn = manager.clone();
    let pong: String = redis::cmd("PING")
        .query_async(&mut conn)
        .await
        .with_context(|| format!("ping valkey at {url}"))?;
    if pong != "PONG" {
        return Err(anyhow::anyhow!(
            "ping valkey at {url}: unexpected response {pong}"
        ));
    }

    Ok(manager)
}

/// Readiness checker that PINGs the underlying Valkey connection.
#[derive(Clone)]
pub struct ValkeyChecker {
    conn: ConnectionManager,
}

impl ValkeyChecker {
    pub fn new(conn: ConnectionManager) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl Checker for ValkeyChecker {
    fn name(&self) -> &'static str {
        "valkey"
    }

    async fn check(&self) -> Result<()> {
        let mut conn = self.conn.clone();
        let pong: String = redis::cmd("PING")
            .query_async(&mut conn)
            .await
            .context("ping valkey")?;
        if pong != "PONG" {
            return Err(anyhow::anyhow!("unexpected PING response: {pong}"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_without_password() {
        let yaml = r#"
app:
  name: t
  environment: dev
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
  host: valkey.local
  port: 6379
  password: ""
  db: 2
  connect_timeout: 5
otel:
  exporter: none
  endpoint: ""
  service_name: t
  sampling: 1.0
log:
  level: info
  format: json
example:
  enabled: true
  default_page_size: 20
  max_page_size: 100
  max_name_length: 255
"#;
        let settings = ::config::Config::builder()
            .add_source(::config::File::from_str(yaml, ::config::FileFormat::Yaml))
            .build()
            .unwrap();
        let cfg: Config = settings.try_deserialize().unwrap();
        assert_eq!(build_url(&cfg), "redis://valkey.local:6379/2");
    }

    #[test]
    fn url_with_password() {
        let yaml = r#"
app:
  name: t
  environment: dev
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
  host: valkey.local
  port: 6379
  password: "secret"
  db: 0
  connect_timeout: 5
otel:
  exporter: none
  endpoint: ""
  service_name: t
  sampling: 1.0
log:
  level: info
  format: json
example:
  enabled: true
  default_page_size: 20
  max_page_size: 100
  max_name_length: 255
"#;
        let settings = ::config::Config::builder()
            .add_source(::config::File::from_str(yaml, ::config::FileFormat::Yaml))
            .build()
            .unwrap();
        let cfg: Config = settings.try_deserialize().unwrap();
        assert_eq!(build_url(&cfg), "redis://:secret@valkey.local:6379/0");
    }

    #[test]
    fn url_with_special_password() {
        let yaml = r#"
app:
  name: t
  environment: dev
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
  host: valkey.local
  port: 6379
  password: "p@ss:w/ord#"
  db: 1
  connect_timeout: 5
otel:
  exporter: none
  endpoint: ""
  service_name: t
  sampling: 1.0
log:
  level: info
  format: json
example:
  enabled: true
  default_page_size: 20
  max_page_size: 100
  max_name_length: 255
"#;
        let settings = ::config::Config::builder()
            .add_source(::config::File::from_str(yaml, ::config::FileFormat::Yaml))
            .build()
            .unwrap();
        let cfg: Config = settings.try_deserialize().unwrap();
        assert_eq!(
            build_url(&cfg),
            "redis://:p%40ss%3Aw%2Ford%23@valkey.local:6379/1"
        );
    }

    #[tokio::test]
    #[ignore = "requires a live Valkey at valkey.local:6379"]
    async fn check_pings_real_valkey() {
        let cfg = Config::load().expect("load config");
        let conn = new_client(&cfg).await.expect("connect");
        let c = ValkeyChecker::new(conn);
        c.check().await.expect("live Valkey ping");
    }
}
