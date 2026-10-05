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

// Feature defaults mirror Go `setDefaults` (port-spec §4): applied when the
// yaml/env key is absent so a section may appear partially (or not at all).
fn default_page_size() -> u32 {
    20
}
fn default_max_page_size() -> u32 {
    100
}
fn default_max_name_length() -> u32 {
    255
}
fn default_max_label_length() -> u32 {
    255
}
fn default_top_machines() -> u32 {
    5
}
fn default_max_top_machines() -> u32 {
    20
}
// Go `setDefaults` valkey.ttl=30s (port-spec §4).
fn default_valkey_ttl() -> String {
    "30s".to_string()
}

// Upper bounds mirroring Go `config` (port-spec §4).
const MAX_PAGE_SIZE_UPPER_BOUND: u32 = 1000;
const MAX_NAME_LENGTH_UPPER_BOUND: u32 = 4096;
const MAX_LABEL_LENGTH_UPPER_BOUND: u32 = 4096;
const MAX_TOP_MACHINES_UPPER_BOUND: u32 = 100;

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
    #[serde(default)]
    #[validate(nested)]
    pub catalog: CatalogConfig,
    #[serde(default)]
    #[validate(nested)]
    pub machines: MachinesConfig,
    #[serde(default)]
    #[validate(nested)]
    pub sales: SalesConfig,
    #[serde(default)]
    #[validate(nested)]
    pub reporting: ReportingConfig,
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
    /// Cache-entry TTL (Go `ValkeyConfig.TTL`, port-spec §4). Human-friendly
    /// duration string ("30s", "5m"), consumed by the catalog cache-aside
    /// decorator; absent falls back to `setDefaults`' `30s`.
    #[serde(default = "default_valkey_ttl")]
    pub ttl: String,
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

/// catalog feature settings (Go `CatalogConfig`, port-spec §4). `enabled`
/// defaults to false; the yaml `catalog:` section is owned by a later wave, so
/// today's `config.yaml` omits it and these defaults apply.
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CatalogConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_page_size")]
    pub default_page_size: u32,
    #[serde(default = "default_max_page_size")]
    pub max_page_size: u32,
    #[serde(default = "default_max_name_length")]
    pub max_name_length: u32,
}

/// machines feature settings (Go `MachinesConfig`, port-spec §4).
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct MachinesConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_page_size")]
    pub default_page_size: u32,
    #[serde(default = "default_max_page_size")]
    pub max_page_size: u32,
    #[serde(default = "default_max_label_length")]
    pub max_label_length: u32,
}

/// sales feature settings (Go `SalesConfig`, port-spec §4) — toggle only.
#[derive(Debug, Clone, Default, Deserialize, Validate)]
pub struct SalesConfig {
    #[serde(default)]
    pub enabled: bool,
}

/// reporting feature settings (Go `ReportingConfig`, port-spec §4).
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct ReportingConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_top_machines")]
    pub default_top_machines: u32,
    #[serde(default = "default_max_top_machines")]
    pub max_top_machines: u32,
}

// Defaults for a wholly absent yaml section, matching the per-field serde
// defaults above and Go `setDefaults` (port-spec §4). Feature `enabled` is
// false by default, so absent sections register nothing.
impl Default for CatalogConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            default_page_size: default_page_size(),
            max_page_size: default_max_page_size(),
            max_name_length: default_max_name_length(),
        }
    }
}

impl Default for MachinesConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            default_page_size: default_page_size(),
            max_page_size: default_max_page_size(),
            max_label_length: default_max_label_length(),
        }
    }
}

impl Default for ReportingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            default_top_machines: default_top_machines(),
            max_top_machines: default_max_top_machines(),
        }
    }
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
        // SCREAMING_SNAKE names win when set. List leaves are comma-split because the Rust `config`
        // crate does not weak-type a scalar into a `Vec<String>` the way Go's viper does.
        for (key, env_name) in leaf_bindings() {
            if let Ok(val) = std::env::var(env_name) {
                builder = builder
                    .set_override(key, env_override_value(key, &val))
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
        let mut url = url::Url::parse("postgres://localhost").expect("valid base url");
        let _ = url.set_username(&self.db.user);
        let _ = url.set_password(Some(&self.db.password));
        let _ = url.set_host(Some(&self.db.host));
        let _ = url.set_port(Some(self.db.port));
        url.set_path(&self.db.name);
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

    /// Cache-entry TTL (Go `cfg.Valkey.TTL`). An empty value falls back to the
    /// `setDefaults` default of 30 seconds (Go `omitempty`).
    pub fn valkey_ttl(&self) -> Duration {
        if self.valkey.ttl.is_empty() {
            return Duration::from_secs(30);
        }
        parse_humantime("valkey.ttl", &self.valkey.ttl).expect("validated in validate_cross")
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
        // `valkey.ttl` is `omitempty,min=1s` (Go `ValkeyConfig.TTL`, §4): an
        // empty value is allowed and the accessor falls back to 30s; a present
        // value must parse to a positive duration.
        if !self.valkey.ttl.is_empty() {
            let d = parse_humantime("valkey.ttl", &self.valkey.ttl)?;
            if d.is_zero() {
                return Err(anyhow!("valkey.ttl must be > 0"));
            }
        }
        // Per-feature checks run only when the feature is enabled (Go
        // `Config.Validate`, port-spec §4).
        if self.catalog.enabled {
            validate_catalog(&self.catalog)?;
        }
        if self.machines.enabled {
            validate_machines(&self.machines)?;
        }
        if self.reporting.enabled {
            validate_reporting(&self.reporting)?;
        }
        Ok(())
    }
}

/// catalog limit checks (Go `validateCatalog`, port-spec §4).
fn validate_catalog(c: &CatalogConfig) -> anyhow::Result<()> {
    if c.default_page_size < 1 {
        return Err(anyhow!("CATALOG_DEFAULT_PAGE_SIZE must be >= 1"));
    }
    if c.max_page_size < 1 {
        return Err(anyhow!("CATALOG_MAX_PAGE_SIZE must be >= 1"));
    }
    if c.max_name_length < 1 {
        return Err(anyhow!("CATALOG_MAX_NAME_LENGTH must be >= 1"));
    }
    if c.default_page_size > c.max_page_size {
        return Err(anyhow!(
            "CATALOG_DEFAULT_PAGE_SIZE must be <= CATALOG_MAX_PAGE_SIZE"
        ));
    }
    if c.max_page_size > MAX_PAGE_SIZE_UPPER_BOUND {
        return Err(anyhow!(
            "CATALOG_MAX_PAGE_SIZE exceeds maximum allowed value {MAX_PAGE_SIZE_UPPER_BOUND}"
        ));
    }
    if c.max_name_length > MAX_NAME_LENGTH_UPPER_BOUND {
        return Err(anyhow!(
            "CATALOG_MAX_NAME_LENGTH exceeds maximum allowed value {MAX_NAME_LENGTH_UPPER_BOUND}"
        ));
    }
    Ok(())
}

/// machines limit checks (Go `validateMachines`, port-spec §4).
fn validate_machines(m: &MachinesConfig) -> anyhow::Result<()> {
    if m.default_page_size < 1 {
        return Err(anyhow!("MACHINES_DEFAULT_PAGE_SIZE must be >= 1"));
    }
    if m.max_page_size < 1 {
        return Err(anyhow!("MACHINES_MAX_PAGE_SIZE must be >= 1"));
    }
    if m.max_label_length < 1 {
        return Err(anyhow!("MACHINES_MAX_LABEL_LENGTH must be >= 1"));
    }
    if m.default_page_size > m.max_page_size {
        return Err(anyhow!(
            "MACHINES_DEFAULT_PAGE_SIZE must be <= MACHINES_MAX_PAGE_SIZE"
        ));
    }
    if m.max_page_size > MAX_PAGE_SIZE_UPPER_BOUND {
        return Err(anyhow!(
            "MACHINES_MAX_PAGE_SIZE exceeds maximum allowed value {MAX_PAGE_SIZE_UPPER_BOUND}"
        ));
    }
    if m.max_label_length > MAX_LABEL_LENGTH_UPPER_BOUND {
        return Err(anyhow!(
            "MACHINES_MAX_LABEL_LENGTH exceeds maximum allowed value {MAX_LABEL_LENGTH_UPPER_BOUND}"
        ));
    }
    Ok(())
}

/// reporting limit checks (Go `validateReporting`, port-spec §4).
fn validate_reporting(r: &ReportingConfig) -> anyhow::Result<()> {
    if r.default_top_machines < 1 {
        return Err(anyhow!("REPORTING_DEFAULT_TOP_MACHINES must be >= 1"));
    }
    if r.max_top_machines < 1 {
        return Err(anyhow!("REPORTING_MAX_TOP_MACHINES must be >= 1"));
    }
    if r.default_top_machines > r.max_top_machines {
        return Err(anyhow!(
            "REPORTING_DEFAULT_TOP_MACHINES must be <= REPORTING_MAX_TOP_MACHINES"
        ));
    }
    if r.max_top_machines > MAX_TOP_MACHINES_UPPER_BOUND {
        return Err(anyhow!(
            "REPORTING_MAX_TOP_MACHINES exceeds maximum allowed value {MAX_TOP_MACHINES_UPPER_BOUND}"
        ));
    }
    Ok(())
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
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf))
    {
        out.push(dir.join("config.yaml").to_string_lossy().into_owned());
    }
    out
}

/// CORS leaves backed by `Vec<String>` in [`HttpConfig`]; env values comma-split into
/// a sequence to match Go's viper weak-typing. Exactly these three keys are list-valued.
const LIST_LEAVES: [&str; 3] = [
    "http.cors_allow_origins",
    "http.cors_allow_methods",
    "http.cors_allow_headers",
];

/// Convert a scalar env override to the type its config leaf expects. The CORS list
/// leaves (see [`LIST_LEAVES`]) comma-split; every other leaf stays a scalar string.
/// Splitting trims each item and drops empties, so `""` -> `[]` and `"*"` -> `["*"]`.
fn env_override_value(key: &str, raw: &str) -> ::config::Value {
    if LIST_LEAVES.contains(&key) {
        let items: Vec<String> = raw
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        items.into()
    } else {
        raw.into()
    }
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
        ("valkey.ttl", "VALKEY_TTL"),
        ("valkey.connect_timeout", "VALKEY_CONNECT_TIMEOUT"),
        ("log.level", "LOG_LEVEL"),
        ("log.format", "LOG_FORMAT"),
        ("otel.exporter", "OTEL_EXPORTER"),
        ("otel.endpoint", "OTEL_EXPORTER_OTLP_ENDPOINT"),
        ("otel.service_name", "OTEL_SERVICE_NAME"),
        ("otel.sampling", "OTEL_TRACES_SAMPLER_ARG"),
        ("catalog.enabled", "CATALOG_ENABLED"),
        ("catalog.default_page_size", "CATALOG_DEFAULT_PAGE_SIZE"),
        ("catalog.max_page_size", "CATALOG_MAX_PAGE_SIZE"),
        ("catalog.max_name_length", "CATALOG_MAX_NAME_LENGTH"),
        ("machines.enabled", "MACHINES_ENABLED"),
        ("machines.default_page_size", "MACHINES_DEFAULT_PAGE_SIZE"),
        ("machines.max_page_size", "MACHINES_MAX_PAGE_SIZE"),
        ("machines.max_label_length", "MACHINES_MAX_LABEL_LENGTH"),
        ("sales.enabled", "SALES_ENABLED"),
        ("reporting.enabled", "REPORTING_ENABLED"),
        (
            "reporting.default_top_machines",
            "REPORTING_DEFAULT_TOP_MACHINES",
        ),
        ("reporting.max_top_machines", "REPORTING_MAX_TOP_MACHINES"),
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
"#
    }

    fn from_yaml_str(yaml: &str) -> ::config::Config {
        ::config::Config::builder()
            .add_source(::config::File::from_str(yaml, ::config::FileFormat::Yaml))
            .build()
            .expect("yaml builds")
    }

    /// Deserialize `sample_yaml()` with env-style scalar overrides applied through the
    /// same `env_override_value` conversion `Config::load` uses, without touching the
    /// process environment (so cases stay deterministic and parallel-safe).
    fn from_yaml_with_env(overrides: &[(&str, &str)]) -> Config {
        let mut builder = ::config::Config::builder().add_source(::config::File::from_str(
            sample_yaml(),
            ::config::FileFormat::Yaml,
        ));
        for (key, raw) in overrides {
            builder = builder
                .set_override(*key, env_override_value(key, raw))
                .expect("valid config key");
        }
        builder
            .build()
            .expect("config builds")
            .try_deserialize()
            .expect("deserialize")
    }

    #[test]
    fn cors_lists_load_from_yaml_only() {
        let yaml = sample_yaml().replace(
            "  health_probe_timeout: 5s\n",
            "  health_probe_timeout: 5s\n  cors_allow_origins:\n    - https://a.example\n    - https://b.example\n  cors_allow_methods:\n    - GET\n    - POST\n  cors_allow_headers:\n    - Authorization\n",
        );
        let cfg: Config = from_yaml_str(&yaml).try_deserialize().unwrap();
        assert_eq!(
            cfg.http.cors_allow_origins,
            vec!["https://a.example", "https://b.example"]
        );
        assert_eq!(cfg.http.cors_allow_methods, vec!["GET", "POST"]);
        assert_eq!(cfg.http.cors_allow_headers, vec!["Authorization"]);
    }

    #[test]
    fn cors_lists_split_comma_env_values_and_trim_items() {
        let cfg = from_yaml_with_env(&[
            (
                "http.cors_allow_methods",
                "GET,POST,PUT,PATCH,DELETE,OPTIONS",
            ),
            (
                "http.cors_allow_headers",
                "Authorization,Content-Type,X-Request-ID",
            ),
            (
                "http.cors_allow_origins",
                " https://a.example , https://b.example ",
            ),
        ]);
        assert_eq!(
            cfg.http.cors_allow_methods,
            vec!["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"]
        );
        assert_eq!(
            cfg.http.cors_allow_headers,
            vec!["Authorization", "Content-Type", "X-Request-ID"]
        );
        assert_eq!(
            cfg.http.cors_allow_origins,
            vec!["https://a.example", "https://b.example"]
        );
    }

    #[test]
    fn cors_single_star_env_value_becomes_one_element() {
        let cfg = from_yaml_with_env(&[("http.cors_allow_origins", "*")]);
        assert_eq!(cfg.http.cors_allow_origins, vec!["*"]);
    }

    #[test]
    fn cors_empty_env_value_becomes_empty_list() {
        let cfg = from_yaml_with_env(&[("http.cors_allow_origins", "")]);
        assert!(cfg.http.cors_allow_origins.is_empty());
    }

    #[test]
    fn non_list_leaf_env_override_stays_scalar() {
        let cfg = from_yaml_with_env(&[("http.body_limit", "2M")]);
        assert_eq!(cfg.http.body_limit, "2M");
    }

    #[test]
    fn load_from_yaml_string_succeeds() {
        let settings = from_yaml_str(sample_yaml());
        let cfg: Config = settings.try_deserialize().expect("deserialize");
        cfg.validate().expect("validate");
        assert_eq!(cfg.app.name, "test-svc");
        assert_eq!(cfg.http.port, 8080);
        assert_eq!(cfg.db.max_conns, 10);
    }

    #[test]
    fn db_conn_string_has_sslmode() {
        let cfg: Config = from_yaml_str(sample_yaml()).try_deserialize().unwrap();
        let s = cfg.db_conn_string();
        assert!(s.contains("sslmode=disable"), "got {s}");
        assert!(s.contains("postgres://postgres:postgres@localhost:5432/app"));
    }

    #[test]
    fn db_conn_string_percent_encodes_special_chars() {
        // Override db user/password in the sample yaml to contain characters that
        // must be percent-encoded per RFC 3986 in the URI userinfo component.
        // The `url` crate's `set_username`/`set_password` use RFC-3986 encoding
        // (so space -> %20), unlike the form-urlencoded `+` encoding we replaced.
        let yaml = sample_yaml()
            .replace("user: postgres", "user: \"u ser\"")
            .replace("password: postgres", "password: \"p@ss/w:ord\"");
        let cfg: Config = from_yaml_str(&yaml).try_deserialize().unwrap();
        let s = cfg.db_conn_string();
        assert!(s.contains("u%20ser"), "got {s}");
        assert!(s.contains("p%40ss%2Fw%3Aord"), "got {s}");
        assert!(s.contains("sslmode="), "got {s}");
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
    fn feature_sections_default_to_disabled_when_absent() {
        // A yaml with no catalog/machines/sales/reporting sections still
        // deserializes; the sections default to Go `setDefaults` values.
        let cfg: Config = from_yaml_str(sample_yaml()).try_deserialize().unwrap();
        assert!(!cfg.catalog.enabled);
        assert!(!cfg.machines.enabled);
        assert!(!cfg.sales.enabled);
        assert!(!cfg.reporting.enabled);
        assert_eq!(cfg.catalog.default_page_size, 20);
        assert_eq!(cfg.catalog.max_page_size, 100);
        assert_eq!(cfg.catalog.max_name_length, 255);
        assert_eq!(cfg.machines.max_label_length, 255);
        assert_eq!(cfg.reporting.default_top_machines, 5);
        assert_eq!(cfg.reporting.max_top_machines, 20);
        cfg.validate_cross()
            .expect("absent feature sections are valid");
    }

    #[test]
    fn validate_cross_checks_enabled_catalog_limits() {
        let yaml = format!(
            "{}\ncatalog:\n  enabled: true\n  default_page_size: 50\n  max_page_size: 10\n",
            sample_yaml()
        );
        let cfg: Config = from_yaml_str(&yaml).try_deserialize().unwrap();
        let err = cfg.validate_cross().unwrap_err().to_string();
        assert!(
            err.contains("CATALOG_DEFAULT_PAGE_SIZE must be <= CATALOG_MAX_PAGE_SIZE"),
            "got {err}"
        );
    }

    #[test]
    fn validate_cross_ignores_disabled_feature_limits() {
        // Same invalid limits as above, but disabled -> not checked (Go §4).
        let yaml = format!(
            "{}\ncatalog:\n  enabled: false\n  default_page_size: 50\n  max_page_size: 10\n",
            sample_yaml()
        );
        let cfg: Config = from_yaml_str(&yaml).try_deserialize().unwrap();
        cfg.validate_cross()
            .expect("disabled feature limits are not checked");
    }

    #[test]
    fn valkey_ttl_defaults_to_30s_when_absent() {
        let cfg: Config = from_yaml_str(sample_yaml()).try_deserialize().unwrap();
        assert_eq!(cfg.valkey.ttl, "30s");
        assert_eq!(cfg.valkey_ttl(), Duration::from_secs(30));
    }

    #[test]
    fn valkey_ttl_parses_an_override_and_rejects_zero() {
        let yaml = sample_yaml().replace("  db: 0\n", "  db: 0\n  ttl: 45s\n");
        let cfg: Config = from_yaml_str(&yaml).try_deserialize().unwrap();
        cfg.validate_cross().expect("45s is valid");
        assert_eq!(cfg.valkey_ttl(), Duration::from_secs(45));

        let bad = sample_yaml().replace("  db: 0\n", "  db: 0\n  ttl: 0s\n");
        let cfg: Config = from_yaml_str(&bad).try_deserialize().unwrap();
        assert!(cfg.validate_cross().is_err(), "a 0s TTL is rejected");
    }

    #[test]
    fn leaf_bindings_cover_valkey_ttl() {
        let b = leaf_bindings();
        assert!(
            b.iter()
                .any(|(k, e)| *k == "valkey.ttl" && *e == "VALKEY_TTL"),
            "missing leaf binding valkey.ttl -> VALKEY_TTL"
        );
    }

    #[test]
    fn leaf_bindings_cover_feature_sections() {
        let b = leaf_bindings();
        for (key, env) in [
            ("catalog.enabled", "CATALOG_ENABLED"),
            ("catalog.max_name_length", "CATALOG_MAX_NAME_LENGTH"),
            ("machines.max_label_length", "MACHINES_MAX_LABEL_LENGTH"),
            ("sales.enabled", "SALES_ENABLED"),
            (
                "reporting.default_top_machines",
                "REPORTING_DEFAULT_TOP_MACHINES",
            ),
            ("reporting.max_top_machines", "REPORTING_MAX_TOP_MACHINES"),
        ] {
            assert!(
                b.iter().any(|(k, e)| *k == key && *e == env),
                "missing leaf binding {key} -> {env}"
            );
        }
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
