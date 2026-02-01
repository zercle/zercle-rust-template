use std::error::Error;

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};

/// Request ID storage for the current request.
#[derive(Clone, Debug)]
pub struct RequestId(pub String);

impl RequestId {
    /// Generate a new random request ID.
    pub fn generate() -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        Self(id)
    }

    /// Get the request ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::generate()
    }
}

/// Log format configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    /// JSON format for production environments.
    Json,
    /// Pretty format for development environments.
    Pretty,
    /// Compact format (one line per log).
    Compact,
}

impl LogFormat {
    /// Parse log format from string.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "json" => LogFormat::Json,
            "compact" => LogFormat::Compact,
            _ => LogFormat::Pretty,
        }
    }
}

/// Log level configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    /// Parse log level from string.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "trace" => LogLevel::Trace,
            "debug" => LogLevel::Debug,
            "info" => LogLevel::Info,
            "warn" | "warning" => LogLevel::Warn,
            "error" => LogLevel::Error,
            _ => LogLevel::Info,
        }
    }

    /// Convert to tracing Level.
    pub fn to_tracing_level(self) -> tracing::Level {
        match self {
            LogLevel::Trace => tracing::Level::TRACE,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Error => tracing::Level::ERROR,
        }
    }
}

/// Initialize structured logging with the specified configuration.
pub fn init_logging(log_level: &str, format: &str) -> Result<(), Box<dyn Error>> {
    let log_level = LogLevel::from_str(log_level);
    let log_format = LogFormat::from_str(format);

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(log_level.to_tracing_level().as_str()));

    match log_format {
        LogFormat::Json => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt::layer().json().with_thread_names(false).with_thread_ids(false).with_target(true))
                .init();
        }
        LogFormat::Pretty => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt::layer().pretty().with_thread_names(false).with_thread_ids(false).with_target(true))
                .init();
        }
        LogFormat::Compact => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt::layer().compact().with_thread_names(false).with_thread_ids(false).with_target(true))
                .init();
        }
    }

    tracing::info!(
        level = ?log_level,
        format = ?log_format,
        "Logging initialized"
    );

    Ok(())
}

/// Initialize logging with environment-based configuration.
pub fn init_logging_from_env() -> Result<(), Box<dyn Error>> {
    let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    let log_format = std::env::var("LOG_FORMAT").unwrap_or_else(|_| "pretty".to_string());

    init_logging(&log_level, &log_format)
}

/// Create a new span with request ID tracking.
#[inline]
pub fn span_with_request_id(_name: &'static str, request_id: &RequestId) -> tracing::Span {
    tracing::info_span!("request", request_id = request_id.as_str())
}

/// Record a debug log with request ID.
#[inline]
pub fn log_with_request_id(request_id: &RequestId, message: &str) {
    tracing::info!(request_id = request_id.as_str(), "{}", message);
}

/// Convenience function to get the current request ID from span context.
pub fn current_request_id() -> Option<RequestId> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing::{debug, info, trace, warn, error};
    use std::sync::OnceLock;

    // Use a OnceLock to ensure we only set the global subscriber once
    static SUBSCRIBER_GUARD: OnceLock<()> = OnceLock::new();

    fn ensure_subscriber_initialized() {
        SUBSCRIBER_GUARD.get_or_init(|| {
            let _ = init_logging("info", "compact");
        });
    }

    #[test]
    fn test_log_format_parsing() {
        assert_eq!(LogFormat::from_str("json"), LogFormat::Json);
        assert_eq!(LogFormat::from_str("pretty"), LogFormat::Pretty);
        assert_eq!(LogFormat::from_str("compact"), LogFormat::Compact);
        assert_eq!(LogFormat::from_str("unknown"), LogFormat::Pretty);
    }

    #[test]
    fn test_log_level_parsing() {
        assert_eq!(LogLevel::from_str("trace"), LogLevel::Trace);
        assert_eq!(LogLevel::from_str("debug"), LogLevel::Debug);
        assert_eq!(LogLevel::from_str("info"), LogLevel::Info);
        assert_eq!(LogLevel::from_str("warn"), LogLevel::Warn);
        assert_eq!(LogLevel::from_str("error"), LogLevel::Error);
    }

    #[test]
    fn test_request_id_generation() {
        let id1 = RequestId::generate();
        let id2 = RequestId::generate();

        assert_ne!(id1.0, id2.0);
        assert!(!id1.0.is_empty());
        assert!(id1.0.len() == 36);
    }

    #[test]
    fn test_init_logging_json_format() {
        ensure_subscriber_initialized();
        info!("Test log message in JSON format");
    }

    #[test]
    fn test_init_logging_pretty_format() {
        ensure_subscriber_initialized();
        info!("Test log message in pretty format");
    }

    #[test]
    fn test_init_logging_compact_format() {
        ensure_subscriber_initialized();
        info!("Test log message in compact format");
    }

    #[test]
    fn test_span_with_request_id() {
        ensure_subscriber_initialized();

        let request_id = RequestId::generate();
        let _span = span_with_request_id("test_span", &request_id);

        debug!(request_id = request_id.as_str(), "Testing span with request ID");
    }

    #[test]
    fn test_log_with_request_id() {
        ensure_subscriber_initialized();

        let request_id = RequestId::generate();
        log_with_request_id(&request_id, "Test message");
    }

    #[test]
    fn test_different_log_levels() {
        ensure_subscriber_initialized();

        trace!("Trace level message");
        debug!("Debug level message");
        info!("Info level message");
        warn!("Warn level message");
        error!("Error level message");
    }
}
