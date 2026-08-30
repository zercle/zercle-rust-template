//! CORS middleware.
//!
//! Mirrors `internal/shared/middleware/cors.go` (structure.md §9). Builds
//! `tower_http::cors::CorsLayer` from `cfg.http.cors_*`. Defaults when the corresponding list is
//! empty:
//!
//! * origins: `["*"]`
//! * methods: `GET,HEAD,PUT,PATCH,POST,DELETE`
//! * headers: `Origin,Content-Type,Accept,Authorization`
//! * expose: `Content-Length`
//! * max age: `86400` seconds (24h)

use std::time::Duration;

use axum::http::{HeaderName, HeaderValue, Method};
use tower_http::cors::{AllowOrigin, Any, CorsLayer};

use crate::platform::config::Config;

const DEFAULT_METHODS: &[&str] = &["GET", "HEAD", "PUT", "PATCH", "POST", "DELETE"];
const DEFAULT_HEADERS: &[&str] = &["Origin", "Content-Type", "Accept", "Authorization"];
const EXPOSE_HEADERS: [HeaderName; 1] = [HeaderName::from_static("content-length")];
const MAX_AGE_SECS: u64 = 86_400;

/// Build the [`CorsLayer`] from `cfg.http.cors_*`.
pub fn layer(cfg: &Config) -> CorsLayer {
    let mut layer = CorsLayer::new();

    // Origins: empty or single "*" → Any; explicit list → AllowOrigin::list.
    let any_wildcard = cfg.http.cors_allow_origins.is_empty()
        || (cfg.http.cors_allow_origins.len() == 1 && cfg.http.cors_allow_origins[0] == "*");
    if any_wildcard {
        layer = layer.allow_origin(Any);
    } else {
        let parsed: Vec<HeaderValue> = cfg
            .http
            .cors_allow_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        layer = layer.allow_origin(AllowOrigin::list(parsed));
    }

    let methods: Vec<Method> = if cfg.http.cors_allow_methods.is_empty() {
        DEFAULT_METHODS
            .iter()
            .filter_map(|m| m.parse().ok())
            .collect()
    } else {
        cfg.http
            .cors_allow_methods
            .iter()
            .filter_map(|m| m.parse().ok())
            .collect()
    };
    layer = layer.allow_methods(methods);

    let headers: Vec<HeaderName> = if cfg.http.cors_allow_headers.is_empty() {
        DEFAULT_HEADERS
            .iter()
            .filter_map(|h| h.parse().ok())
            .collect()
    } else {
        cfg.http
            .cors_allow_headers
            .iter()
            .filter_map(|h| h.parse().ok())
            .collect()
    };
    layer = layer.allow_headers(headers);

    layer
        .expose_headers(EXPOSE_HEADERS)
        .max_age(Duration::from_secs(MAX_AGE_SECS))
}

/// Build the default [`CorsLayer`] (no config): same defaults as when `cors_*` is empty.
pub fn default_layer() -> CorsLayer {
    let methods: Vec<Method> = DEFAULT_METHODS
        .iter()
        .filter_map(|m| m.parse().ok())
        .collect();
    let headers: Vec<HeaderName> = DEFAULT_HEADERS
        .iter()
        .filter_map(|h| h.parse().ok())
        .collect();
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(methods)
        .allow_headers(headers)
        .expose_headers(EXPOSE_HEADERS)
        .max_age(Duration::from_secs(MAX_AGE_SECS))
}

#[cfg(test)]
mod tests {
    use axum::{Router, body::Body, http::Request, http::header, routing::get};
    use tower::ServiceExt;

    use super::*;
    use crate::platform::config::Config;

    fn sample_cfg_with(origins: Vec<String>, methods: Vec<String>, headers: Vec<String>) -> Config {
        let origins_str = if origins.is_empty() {
            "  cors_allow_origins: []\n".to_string()
        } else {
            let mut s = String::new();
            s.push_str("  cors_allow_origins:\n");
            for o in &origins {
                s.push_str(&format!("    - \"{o}\"\n"));
            }
            s
        };
        let methods_str = if methods.is_empty() {
            "  cors_allow_methods: []\n".to_string()
        } else {
            let mut s = String::new();
            s.push_str("  cors_allow_methods:\n");
            for m in &methods {
                s.push_str(&format!("    - \"{m}\"\n"));
            }
            s
        };
        let headers_str = if headers.is_empty() {
            "  cors_allow_headers: []\n".to_string()
        } else {
            let mut s = String::new();
            s.push_str("  cors_allow_headers:\n");
            for h in &headers {
                s.push_str(&format!("    - \"{h}\"\n"));
            }
            s
        };
        let yaml = format!(
            r#"
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
{origins_str}{methods_str}{headers_str}grpc:
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
"#
        );
        let settings = ::config::Config::builder()
            .add_source(::config::File::from_str(&yaml, ::config::FileFormat::Yaml))
            .build()
            .expect("yaml builds");
        settings.try_deserialize::<Config>().expect("deserialize")
    }

    async fn ok() -> &'static str {
        "ok"
    }

    #[tokio::test]
    async fn preflight_with_default_config_is_allowed() {
        let cfg = sample_cfg_with(vec![], vec![], vec![]);
        let layer = layer(&cfg);
        let app = Router::new().route("/", get(ok)).layer(layer);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("OPTIONS")
                    .uri("/")
                    .header(header::ORIGIN, "https://example.com")
                    .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                    .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "Content-Type")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = resp.status().as_u16();
        assert!(
            status == 200 || status == 204,
            "preflight should be allowed, got {status}"
        );
        assert!(
            resp.headers()
                .contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN),
            "ACAO header should be present"
        );
    }

    #[tokio::test]
    async fn preflight_with_explicit_origin_is_allowed() {
        let cfg = sample_cfg_with(vec!["https://example.com".to_string()], vec![], vec![]);
        let layer = layer(&cfg);
        let app = Router::new().route("/", get(ok)).layer(layer);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("OPTIONS")
                    .uri("/")
                    .header(header::ORIGIN, "https://example.com")
                    .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = resp.status().as_u16();
        assert!(
            status == 200 || status == 204,
            "preflight should be allowed, got {status}"
        );
        assert!(
            resp.headers()
                .contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN),
            "ACAO header should be present for explicit origin"
        );
    }

    #[tokio::test]
    async fn preflight_with_explicit_methods_and_headers() {
        let cfg = sample_cfg_with(
            vec!["*".to_string()],
            vec!["GET".to_string(), "POST".to_string()],
            vec!["X-Custom".to_string()],
        );
        let layer = layer(&cfg);
        let app = Router::new().route("/", get(ok)).layer(layer);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("OPTIONS")
                    .uri("/")
                    .header(header::ORIGIN, "https://example.com")
                    .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                    .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "X-Custom")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = resp.status().as_u16();
        assert!(
            status == 200 || status == 204,
            "preflight should be allowed, got {status}"
        );
    }

    #[test]
    fn default_layer_returns_a_layer() {
        let _ = default_layer();
    }
}
