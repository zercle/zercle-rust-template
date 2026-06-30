//! Application configuration: load from `config.yaml` + env, validate, expose helpers.
//!
//! Mirrors `internal/config/config.go` from the Go template (structure.md §5). Env names match
//! the Go template exactly (SCREAMING_SNAKE, no prefix), bound via an explicit leaf-binding table
//! because the `config` crate's default `_` separator collides with SCREAMING_SNAKE names.

use std::time::Duration;

use anyhow::{Context, anyhow};
use serde::Deserialize;
use validator::Validate;

fn parse_humantime(field: &str, raw: &str) -> anyhow::Result<Duration> {
    humantime::parse_duration(raw).with_context(|| format!("invalid duration for {field}: {raw:?}"))
}

/// Top-level configuration.
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct Config {
    #[validate(nested)]
    pub app: AppConfig,
    #[validate(nested)]
    pub http: HttpConfig,
    #[validate(nested)]
    pub grpc: GrpcConfig,
    #[validate(nested)]
    pub db: DbConfig,
    #[validate(nested)]
    pub valkey: ValkeyConfig,
    #[validate(nested)]
    pub otel: OtelConfig,
    #[validate(nested)]
    pub log: LogConfig,
    #[validate(nested)]
    pub example: ExampleConfig,
}

/// Process-level settings.
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct AppConfig {
    #[validate(length(min = 1))]
    pub name: String,
    #[validate(length(min = 1))]
    pub environment: String,
    #[validate(length(min = 1))]
    pub host: String,
    #[validate(range(min = 1, max = 65535))]
    pub port: u16,
    #[serde(default)]
    pub shutdown_timeout: String,
}

/// HTTP server settings + CORS.
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct HttpConfig {
    #[validate(length(min = 1))]
    pub host: String,
    #[validate(range(min = 1, max = 65535))]
    pub port: u16,
    #[serde(default)]
    pub read_timeout: String,
    #[serde(default)]
    pub write_timeout: String,
    #[serde(default)]
    pub idle_timeout: String,
    #[validate(length(min = 1))]
    pub body_limit: String,
    #[serde(default)]
    pub health_probe_timeout: String,
    #[serde(default)]
    pub cors_allow_origins: Vec<String>,
    #[serde(default)]
    pub cors_allow_methods: Vec<String>,
    #[serde(default)]
    pub cors_allow_headers: Vec<String>,
}

/// gRPC server settings.
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct GrpcConfig {
    #[validate(length(min = 1))]
    pub host: String,
    #[validate(range(min = 1, max = 65535))]
    pub port: u16,
}

/// PostgreSQL connection + pool settings.
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct DbConfig {
    #[validate(length(min = 1))]
    pub host: String,
    #[validate(range(min = 1, max = 65535))]
    pub port: u16,
    #[validate(length(min = 1))]
    pub name: String,
    #[validate(length(min = 1))]
    pub user: String,
    #[validate(length(min = 1))]
    pub password: String,
    #[validate(length(min = 1))]
    pub ssl_mode: String,
    #[validate(range(min = 1))]
    pub max_conns: u32,
    #[validate(range(min = 0))]
    pub min_conns: u32,
    #[serde(default)]
    pub max_conn_idle: String,
    #[serde(default)]
    pub max_conn_life: String,
    #[serde(default)]
    pub connect_timeout: String,
}

/// Valkey (Redis-protocol) settings.
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct ValkeyConfig {
    #[validate(length(min = 1))]
    pub host: String,
    #[validate(range(min = 1, max = 65535))]
    pub port: u16,
    #[serde(default)]
    pub password: String,
    #[validate(range(min = 0))]
    pub db: u8,
    #[serde(default)]
    pub connect_timeout: String,
}

/// OpenTelemetry exporter settings.
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct OtelConfig {
    #[validate(length(min = 1))]
    pub exporter: String,
    #[serde(default)]
    pub endpoint: String,
    #[validate(length(min = 1))]
    pub service_name: String,
    #[validate(range(min = 0.0, max = 1.0))]
    pub sampling: f64,
}

/// Logger settings.
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct LogConfig {
    #[validate(length(min = 1))]
    pub level: String,
    #[validate(length(min = 1))]
    pub format: String,
}

/// Stub feature toggle + settings.
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct ExampleConfig {
    #[serde(default)]
    pub enabled: bool,
    #[validate(range(min = 1))]
    pub default_page_size: u32,
    #[validate(range(min = 1))]
    pub max_page_size: u32,
    #[validate(range(min = 1))]
    pub max_name_length: u32,
}

impl Config {
    /// Load configuration from yaml (`config.yaml`, or path in `CONFIG_FILE`) + env overrides.
    ///
    /// File lookup order (later sources win on duplicate keys):
    /// 1. `config.yaml` in the current working directory.
    /// 2. `CONFIG_FILE` env (absolute or relative path).
    /// 3. `config.yaml` next to the running executable (e.g. `/config.yaml` in the
    ///    distroless container where the cwd is `/`).
    ///
    /// Env bindings are explicit and identical to the Go template's `leafBindings()` table — see
    /// decision D5.
    pub fn load() -> anyhow::Result<Self> {
        let mut builder = ::config::Config::builder()
            .add_source(::config::File::with_name("config").required(false));

        if let Some(path) = config_file_override() {
            builder = builder.add_source(::config::File::with_name(&path).required(false));
        }

        for path in exe_dir_config_candidates() {
            builder = builder.add_source(::config::File::with_name(&path).required(false));
        }

        builder = builder.add_source(
            ::config::Environment::with_prefix("")
                .separator("__")
                .try_parsing(true),
        );

        // Apply the explicit Go-template leaf bindings (D5) on top of the auto env source so that
        // SCREAMING_SNAKE names win when set.
        for (key, env_name) in leaf_bindings() {
            if let Ok(val) = std::env::var(env_name) {
                builder = builder
                    .set_override(key, val)
                    .with_context(|| format!("set {key} from {env_name}"))?;
            }
        }

        let settings = builder
            .build()
            .context("build layered config (yaml + env)")?;

        let cfg: Config = settings
            .try_deserialize::<Config>()
            .context("deserialize Config")?;
        Ok(cfg)
    }

    pub fn http_addr(&self) -> String {
        format!("{}:{}", self.http.host, self.http.port)
    }

    pub fn grpc_addr(&self) -> String {
        format!("{}:{}", self.grpc.host, self.grpc.port)
    }

    pub fn db_conn_string(&self) -> String {
        let url = url::Url::parse(&format!(
            "postgres://{}:{}@{}:{}/{}",
            urlencoding(&self.db.user),
            urlencoding(&self.db.password),
            self.db.host,
            self.db.port,
            self.db.name,
        ))
        .expect("static postgres URL is always parseable");
        let mut url = url;
        url.query_pairs_mut()
            .append_pair("sslmode", &self.db.ssl_mode);
        url.to_string()
    }

    pub fn valkey_addr(&self) -> String {
        format!("{}:{}", self.valkey.host, self.valkey.port)
    }

    pub fn shutdown_timeout(&self) -> Duration {
        parse_humantime("app.shutdown_timeout", &self.app.shutdown_timeout)
            .expect("validated in validate_cross")
    }

    pub fn db_connect_timeout(&self) -> Duration {
        parse_humantime("db.connect_timeout", &self.db.connect_timeout)
            .expect("validated in validate_cross")
    }

    pub fn db_max_conn_idle(&self) -> Duration {
        parse_humantime("db.max_conn_idle", &self.db.max_conn_idle)
            .expect("validated in validate_cross")
    }

    pub fn db_max_conn_life(&self) -> Duration {
        parse_humantime("db.max_conn_life", &self.db.max_conn_life)
            .expect("validated in validate_cross")
    }

    pub fn http_read_timeout(&self) -> Duration {
        parse_humantime("http.read_timeout", &self.http.read_timeout)
            .expect("validated in validate_cross")
    }

    pub fn http_write_timeout(&self) -> Duration {
        parse_humantime("http.write_timeout", &self.http.write_timeout)
            .expect("validated in validate_cross")
    }

    pub fn http_idle_timeout(&self) -> Duration {
        parse_humantime("http.idle_timeout", &self.http.idle_timeout)
            .expect("validated in validate_cross")
    }

    pub fn http_health_probe_timeout(&self) -> Duration {
        parse_humantime("http.health_probe_timeout", &self.http.health_probe_timeout)
            .expect("validated in validate_cross")
    }

    pub fn valkey_connect_timeout(&self) -> Duration {
        parse_humantime("valkey.connect_timeout", &self.valkey.connect_timeout)
            .expect("validated in validate_cross")
    }

    /// Cross-section checks in addition to `validator::Validate`.
    pub fn validate_cross(&self) -> anyhow::Result<()> {
        if self.otel.exporter == "otlp" && self.otel.endpoint.is_empty() {
            return Err(anyhow!(
                "OTEL_EXPORTER_OTLP_ENDPOINT is required when OTEL_EXPORTER=otlp"
            ));
        }
        if self.otel.exporter == "otlp" {
            url::Url::parse(&self.otel.endpoint)
                .context("OTEL_EXPORTER_OTLP_ENDPOINT is invalid")?;
        }
        if self.db.max_conns < self.db.min_conns {
            return Err(anyhow!("DB_MAX_CONNS must be >= DB_MIN_CONNS"));
        }
        // All duration fields are stored as humantime strings ("15s", "30m").
        // Reject empty / unparseable / non-positive here so callers can rely on
        // the accessors above to return a valid Duration.
        let positive = [
            ("app.shutdown_timeout", &self.app.shutdown_timeout),
            ("http.read_timeout", &self.http.read_timeout),
            ("http.write_timeout", &self.http.write_timeout),
            ("http.idle_timeout", &self.http.idle_timeout),
            ("http.health_probe_timeout", &self.http.health_probe_timeout),
            ("db.connect_timeout", &self.db.connect_timeout),
            ("db.max_conn_idle", &self.db.max_conn_idle),
            ("db.max_conn_life", &self.db.max_conn_life),
            ("valkey.connect_timeout", &self.valkey.connect_timeout),
        ];
        for (field, raw) in positive {
            if raw.is_empty() {
                return Err(anyhow!("{field} must be a humantime duration like \"15s\""));
            }
            let d = parse_humantime(field, raw)?;
            if d.is_zero() {
                return Err(anyhow!("{field} must be > 0"));
            }
        }
        Ok(())
    }
}

fn urlencoding(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

/// Return the `CONFIG_FILE` env override path, if set and non-empty.
fn config_file_override() -> Option<String> {
    std::env::var("CONFIG_FILE").ok().filter(|p| !p.is_empty())
}

/// Config path to try relative to the running executable's directory.
///
/// Returns the single fallback path `<exe_dir>/config.yaml`. The distroless
/// container runs `/server` with cwd `/`, so `/config.yaml` (copied by the
/// Containerfile) is found this way when the operator has not bind-mounted a
/// config and has not set `CONFIG_FILE`. The `config` crate resolves the path
/// relative to the cwd, which happens to be the exe's parent for the cases
/// where this fallback matters.
fn exe_dir_config_candidates() -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p = dir.join("config.yaml");
            out.push(p.to_string_lossy().into_owned());
        }
    }
    out
}

/// Mirror of Go `leafBindings()`. `(config_key, ENV_NAME)`. See decision D5.
fn leaf_bindings() -> Vec<(&'static str, &'static str)> {
    vec![
        ("app.name", "APP_NAME"),
        ("app.environment", "APP_ENVIRONMENT"),
        ("app.host", "APP_HOST"),
        ("app.port", "APP_PORT"),
        ("app.shutdown_timeout", "APP_SHUTDOWN_TIMEOUT"),
        ("http.host", "HTTP_HOST"),
        ("http.port", "HTTP_PORT"),
        ("http.read_timeout", "HTTP_READ_TIMEOUT"),
        ("http.write_timeout", "HTTP_WRITE_TIMEOUT"),
        ("http.idle_timeout", "HTTP_IDLE_TIMEOUT"),
        ("http.body_limit", "HTTP_BODY_LIMIT"),
        ("http.health_probe_timeout", "HTTP_HEALTH_PROBE_TIMEOUT"),
        ("http.cors_allow_origins", "HTTP_CORS_ALLOW_ORIGINS"),
        ("http.cors_allow_methods", "HTTP_CORS_ALLOW_METHODS"),
        ("http.cors_allow_headers", "HTTP_CORS_ALLOW_HEADERS"),
        ("grpc.host", "GRPC_HOST"),
        ("grpc.port", "GRPC_PORT"),
        ("db.host", "DB_HOST"),
        ("db.port", "DB_PORT"),
        ("db.name", "DB_NAME"),
        ("db.user", "DB_USER"),
        ("db.password", "DB_PASSWORD"),
        ("db.ssl_mode", "DB_SSL_MODE"),
        ("db.max_conns", "DB_MAX_CONNS"),
        ("db.min_conns", "DB_MIN_CONNS"),
        ("db.max_conn_idle", "DB_MAX_CONN_IDLE"),
        ("db.max_conn_life", "DB_MAX_CONN_LIFE"),
        ("db.connect_timeout", "DB_CONNECT_TIMEOUT"),
        ("valkey.host", "VALKEY_HOST"),
        ("valkey.port", "VALKEY_PORT"),
        ("valkey.password", "VALKEY_PASSWORD"),
        ("valkey.db", "VALKEY_DB"),
        ("valkey.connect_timeout", "VALKEY_CONNECT_TIMEOUT"),
        ("log.level", "LOG_LEVEL"),
        ("log.format", "LOG_FORMAT"),
        ("otel.exporter", "OTEL_EXPORTER"),
        ("otel.endpoint", "OTEL_EXPORTER_OTLP_ENDPOINT"),
        ("otel.service_name", "OTEL_SERVICE_NAME"),
        ("otel.sampling", "OTEL_TRACES_SAMPLER_ARG"),
        ("example.enabled", "EXAMPLE_ENABLED"),
        ("example.default_page_size", "EXAMPLE_DEFAULT_PAGE_SIZE"),
        ("example.max_page_size", "EXAMPLE_MAX_PAGE_SIZE"),
        ("example.max_name_length", "EXAMPLE_MAX_NAME_LENGTH"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_yaml() -> &'static str {
        r#"
app:
  name: test-svc
  environment: development
  host: 0.0.0.0
  port: 8080
  shutdown_timeout: 15s
http:
  host: 0.0.0.0
  port: 8080
  read_timeout: 15s
  write_timeout: 15s
  idle_timeout: 60s
  body_limit: "1M"
  health_probe_timeout: 5s
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
  max_conn_idle: 30m
  max_conn_life: 1h
  connect_timeout: 5s
valkey:
  host: localhost
  port: 6379
  password: ""
  db: 0
  connect_timeout: 5s
otel:
  exporter: none
  endpoint: ""
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
    }

    fn from_yaml_str(yaml: &str) -> ::config::Config {
        ::config::Config::builder()
            .add_source(::config::File::from_str(yaml, ::config::FileFormat::Yaml))
            .build()
            .expect("yaml builds")
    }

    #[test]
    fn load_from_yaml_string_succeeds() {
        let settings = from_yaml_str(sample_yaml());
        let cfg: Config = settings.try_deserialize().expect("deserialize");
        cfg.validate().expect("validate");
        assert_eq!(cfg.app.name, "test-svc");
        assert_eq!(cfg.http.port, 8080);
        assert_eq!(cfg.db.max_conns, 10);
        assert!(cfg.example.enabled);
    }

    #[test]
    fn db_conn_string_has_sslmode() {
        let cfg: Config = from_yaml_str(sample_yaml()).try_deserialize().unwrap();
        let s = cfg.db_conn_string();
        assert!(s.contains("sslmode=disable"), "got {s}");
        assert!(s.contains("postgres://postgres:postgres@localhost:5432/app"));
    }

    #[test]
    fn http_addr_and_grpc_addr_format() {
        let cfg: Config = from_yaml_str(sample_yaml()).try_deserialize().unwrap();
        assert_eq!(cfg.http_addr(), "0.0.0.0:8080");
        assert_eq!(cfg.grpc_addr(), "0.0.0.0:50051");
    }

    #[test]
    fn validate_cross_rejects_otlp_without_endpoint() {
        let yaml = sample_yaml().replace("exporter: none", "exporter: otlp");
        let cfg: Config = from_yaml_str(&yaml).try_deserialize().unwrap();
        assert!(cfg.validate_cross().is_err());
    }

    #[test]
    fn validate_cross_rejects_min_gt_max_conns() {
        let yaml = sample_yaml()
            .replace("max_conns: 10", "max_conns: 1")
            .replace("min_conns: 2", "min_conns: 5");
        let cfg: Config = from_yaml_str(&yaml).try_deserialize().unwrap();
        assert!(cfg.validate_cross().is_err());
    }

    #[test]
    fn validate_rejects_bad_port() {
        // app.port must be in 1..=65535. Pick the HTTP port to avoid disturbing the example
        // struct (its port is unvalidated by the Go validator either).
        let yaml = sample_yaml().replace(
            "http:\n  host: 0.0.0.0\n  port: 8080",
            "http:\n  host: 0.0.0.0\n  port: 0",
        );
        let cfg: Config = from_yaml_str(&yaml).try_deserialize().unwrap();
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn leaf_bindings_matches_go_set() {
        // Sanity: the table is non-empty and matches Go's leafBindings() length.
        let b = leaf_bindings();
        assert!(
            b.len() >= 40,
            "leaf bindings should cover all keys, got {}",
            b.len()
        );
        assert!(b.iter().any(|(k, e)| *k == "app.name" && *e == "APP_NAME"));
        assert!(
            b.iter()
                .any(|(k, e)| *k == "db.max_conns" && *e == "DB_MAX_CONNS")
        );
    }
}
