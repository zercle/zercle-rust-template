//! Middleware Tests - Simplified
//!
//! This module contains basic unit tests for middleware components.

use std::time::Duration;
use zercle_rust_template::config::{
    Argon2idConfig, CorsConfig, DatabaseConfig, JwtConfig, LoggingConfig, RateLimitConfig,
    ServerConfig, Settings,
};
use zercle_rust_template::infrastructure::middleware::auth::AuthState;
use zercle_rust_template::infrastructure::middleware::rate_limit::InMemoryRateLimiter;

// ============================================================================
// Auth State Tests
// ============================================================================

mod auth_state_tests {
    use super::*;

    /// Create test settings
    fn create_test_settings() -> Settings {
        Settings {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 3000,
                env: "test".to_string(),
            },
            database: DatabaseConfig {
                driver: "postgres".to_string(),
                host: "localhost".to_string(),
                port: 5432,
                user: "postgres".to_string(),
                password: "postgres".to_string(),
                name: "test".to_string(),
                pool_size: 5,
                ssl_mode: "disable".to_string(),
            },
            jwt: JwtConfig {
                secret: "test-secret-key-for-testing-purposes-only-123456".to_string(),
                expiration_hours: 24,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
            },
            cors: CorsConfig {
                allowed_origins: vec!["http://localhost:3000".to_string()],
            },
            rate_limit: RateLimitConfig {
                requests_per_minute: 100,
            },
            argon2id: Argon2idConfig {
                memory_kb: 19456,
                iterations: 2,
                parallelism: 1,
            },
        }
    }

    /// Test AuthState creation
    #[test]
    fn test_auth_state_creation() {
        let settings = create_test_settings();
        let auth_state = AuthState::new(&settings);

        assert_eq!(auth_state.jwt_secret.as_str(), settings.jwt.secret);
    }

    /// Test AuthState with different secrets
    #[test]
    fn test_auth_state_different_secrets() {
        let mut settings1 = create_test_settings();
        settings1.jwt.secret = "secret1".to_string();

        let mut settings2 = create_test_settings();
        settings2.jwt.secret = "secret2".to_string();

        let auth_state1 = AuthState::new(&settings1);
        let auth_state2 = AuthState::new(&settings2);

        assert_ne!(auth_state1.jwt_secret, auth_state2.jwt_secret);
    }
}

// ============================================================================
// Rate Limiter Tests
// ============================================================================

mod rate_limiter_tests {
    use super::*;

    /// Test rate limiter within threshold
    #[tokio::test]
    async fn test_rate_limit_within_threshold() {
        let limiter = InMemoryRateLimiter::new();
        let key = "test_client";
        let max_requests = 5;
        let window_secs = 60;

        for i in 0..max_requests {
            let (allowed, remaining, _) = limiter
                .check_rate_limit(key, max_requests, window_secs)
                .await;
            assert!(allowed, "Request {} should be allowed", i + 1);
            assert_eq!(remaining, max_requests - i - 1);
        }
    }

    /// Test rate limiter exceeding threshold
    #[tokio::test]
    async fn test_rate_limit_exceeds_threshold() {
        let limiter = InMemoryRateLimiter::new();
        let key = "test_client";
        let max_requests = 3;
        let window_secs = 60;

        for _ in 0..max_requests {
            let (allowed, _, _) = limiter
                .check_rate_limit(key, max_requests, window_secs)
                .await;
            assert!(allowed);
        }

        let (allowed, remaining, _) = limiter
            .check_rate_limit(key, max_requests, window_secs)
            .await;
        assert!(!allowed);
        assert_eq!(remaining, 0);
    }

    /// Test rate limit window reset
    #[tokio::test]
    async fn test_rate_limit_window_reset() {
        let limiter = InMemoryRateLimiter::new();
        let key = "test_client";
        let max_requests = 2;
        let window_secs = 1;

        let (allowed, _, _) = limiter
            .check_rate_limit(key, max_requests, window_secs)
            .await;
        assert!(allowed);
        let (allowed, _, _) = limiter
            .check_rate_limit(key, max_requests, window_secs)
            .await;
        assert!(allowed);

        let (allowed, _, _) = limiter
            .check_rate_limit(key, max_requests, window_secs)
            .await;
        assert!(!allowed);

        tokio::time::sleep(Duration::from_secs(2)).await;

        let (allowed, remaining, _) = limiter
            .check_rate_limit(key, max_requests, window_secs)
            .await;
        assert!(allowed);
        assert_eq!(remaining, max_requests - 1);
    }

    /// Test rate limit separate limits per IP
    #[tokio::test]
    async fn test_rate_limit_different_ips() {
        let limiter = InMemoryRateLimiter::new();
        let max_requests = 2;
        let window_secs = 60;

        let (allowed, _, _) = limiter
            .check_rate_limit("192.168.1.1", max_requests, window_secs)
            .await;
        assert!(allowed);
        let (allowed, _, _) = limiter
            .check_rate_limit("192.168.1.1", max_requests, window_secs)
            .await;
        assert!(allowed);
        let (allowed, _, _) = limiter
            .check_rate_limit("192.168.1.1", max_requests, window_secs)
            .await;
        assert!(!allowed);

        let (allowed, _, _) = limiter
            .check_rate_limit("192.168.1.2", max_requests, window_secs)
            .await;
        assert!(allowed);
    }

    /// Test rate limit new key initialization
    #[tokio::test]
    async fn test_rate_limit_new_key_initialization() {
        let limiter = InMemoryRateLimiter::new();
        let max_requests = 10;
        let window_secs = 60;
        let new_key = "new_client_12345";

        let (allowed, remaining, _) = limiter
            .check_rate_limit(new_key, max_requests, window_secs)
            .await;
        assert!(allowed);
        assert_eq!(remaining, max_requests - 1);
    }
}

// ============================================================================
// CORS Configuration Tests
// ============================================================================

mod cors_config_tests {
    use super::*;

    /// Create test settings for CORS
    fn create_test_settings() -> Settings {
        Settings {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 3000,
                env: "test".to_string(),
            },
            database: DatabaseConfig {
                driver: "postgres".to_string(),
                host: "localhost".to_string(),
                port: 5432,
                user: "postgres".to_string(),
                password: "postgres".to_string(),
                name: "test".to_string(),
                pool_size: 5,
                ssl_mode: "disable".to_string(),
            },
            jwt: JwtConfig {
                secret: "test-secret".to_string(),
                expiration_hours: 24,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
            },
            cors: CorsConfig {
                allowed_origins: vec![
                    "http://localhost:3000".to_string(),
                    "http://localhost:8080".to_string(),
                ],
            },
            rate_limit: RateLimitConfig {
                requests_per_minute: 100,
            },
            argon2id: Argon2idConfig {
                memory_kb: 19456,
                iterations: 2,
                parallelism: 1,
            },
        }
    }

    /// Test CORS layer creation
    #[test]
    fn test_cors_layer_creation() {
        let settings = create_test_settings();
        let _cors_layer =
            zercle_rust_template::infrastructure::middleware::cors::create_cors_layer(&settings);
    }

    /// Test CORS with wildcard origins
    #[test]
    fn test_cors_wildcard_origins() {
        let mut settings = create_test_settings();
        settings.cors.allowed_origins = vec!["*".to_string()];

        let _cors_layer =
            zercle_rust_template::infrastructure::middleware::cors::create_cors_layer(&settings);
    }

    /// Test CORS with empty origins
    #[test]
    fn test_cors_empty_origins() {
        let mut settings = create_test_settings();
        settings.cors.allowed_origins = vec![];

        let _cors_layer =
            zercle_rust_template::infrastructure::middleware::cors::create_cors_layer(&settings);
    }
}

// ============================================================================
// Logging Configuration Tests
// ============================================================================

mod logging_config_tests {
    use super::*;

    /// Test logging layer creation
    #[test]
    fn test_logging_layer_creation() {
        zercle_rust_template::infrastructure::middleware::logging::create_default_logging_layer();
    }

    /// Test logging layer with debug level
    #[test]
    fn test_logging_layer_debug_level() {
        let settings = Settings {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 3000,
                env: "test".to_string(),
            },
            database: DatabaseConfig {
                driver: "postgres".to_string(),
                host: "localhost".to_string(),
                port: 5432,
                user: "postgres".to_string(),
                password: "postgres".to_string(),
                name: "test".to_string(),
                pool_size: 5,
                ssl_mode: "disable".to_string(),
            },
            jwt: JwtConfig {
                secret: "test-secret".to_string(),
                expiration_hours: 24,
            },
            logging: LoggingConfig {
                level: "debug".to_string(),
                format: "json".to_string(),
            },
            cors: CorsConfig {
                allowed_origins: vec!["http://localhost:3000".to_string()],
            },
            rate_limit: RateLimitConfig {
                requests_per_minute: 100,
            },
            argon2id: Argon2idConfig {
                memory_kb: 19456,
                iterations: 2,
                parallelism: 1,
            },
        };

        zercle_rust_template::infrastructure::middleware::logging::create_logging_layer(&settings);
    }

    /// Test logging layer with error level
    #[test]
    fn test_logging_layer_error_level() {
        let settings = Settings {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 3000,
                env: "test".to_string(),
            },
            database: DatabaseConfig {
                driver: "postgres".to_string(),
                host: "localhost".to_string(),
                port: 5432,
                user: "postgres".to_string(),
                password: "postgres".to_string(),
                name: "test".to_string(),
                pool_size: 5,
                ssl_mode: "disable".to_string(),
            },
            jwt: JwtConfig {
                secret: "test-secret".to_string(),
                expiration_hours: 24,
            },
            logging: LoggingConfig {
                level: "error".to_string(),
                format: "json".to_string(),
            },
            cors: CorsConfig {
                allowed_origins: vec!["http://localhost:3000".to_string()],
            },
            rate_limit: RateLimitConfig {
                requests_per_minute: 100,
            },
            argon2id: Argon2idConfig {
                memory_kb: 19456,
                iterations: 2,
                parallelism: 1,
            },
        };

        zercle_rust_template::infrastructure::middleware::logging::create_logging_layer(&settings);
    }
}
