//! Settings Tests
//!
//! This module contains unit tests for configuration settings loading,
//! validation, and database URL generation.

use std::path::Path;
use tempfile::TempDir;
use zercle_rust_template::config::Settings;

// Helper function to clear all known env vars that could affect Settings
fn clear_all_settings_env_vars() {
    let vars = [
        "SERVER_HOST", "SERVER_PORT", "SERVER_ENV",
        "DB_DRIVER", "DB_HOST", "DB_PORT", "DB_USER", "DB_PASSWORD", "DB_NAME", "DB_POOL_SIZE", "DB_SSL_MODE",
        "JWT_SECRET", "JWT_EXPIRATION_HOURS",
        "LOG_LEVEL", "LOG_FORMAT",
        "CORS_ALLOWED_ORIGINS",
        "RATE_LIMIT_REQUESTS_PER_MINUTE",
        "ARGON2ID_MEMORY_KB", "ARGON2ID_ITERATIONS", "ARGON2ID_PARALLELISM",
    ];
    for var in &vars {
        std::env::remove_var(var);
    }
}

// ============================================================================
// Settings from Environment Tests
// ============================================================================

mod settings_from_env_tests {
    use super::*;

    /// Test loading settings from environment variables
    #[test]
    fn test_settings_from_env() {
        // Clear all env vars first
        clear_all_settings_env_vars();
        
        // Set environment variables
        std::env::set_var("SERVER_HOST", "127.0.0.1");
        std::env::set_var("SERVER_PORT", "8080");
        std::env::set_var("SERVER_ENV", "test");
        std::env::set_var("DB_HOST", "testdb.example.com");
        std::env::set_var("DB_PORT", "5433");
        std::env::set_var("DB_USER", "testuser");
        std::env::set_var("DB_PASSWORD", "testpass");
        std::env::set_var("DB_NAME", "testdb");
        std::env::set_var("JWT_SECRET", "test-jwt-secret-key");
        std::env::set_var("RATE_LIMIT_REQUESTS_PER_MINUTE", "200");

        let settings = Settings::from_env();

        assert!(settings.is_ok(), "Settings should load from environment");
        let settings = settings.unwrap();

        assert_eq!(settings.server.host, "127.0.0.1");
        assert_eq!(settings.server.port, 8080);
        assert_eq!(settings.server.env, "test");
        assert_eq!(settings.database.host, "testdb.example.com");
        assert_eq!(settings.database.port, 5433);
        assert_eq!(settings.database.user, "testuser");
        assert_eq!(settings.database.password, "testpass");
        assert_eq!(settings.database.name, "testdb");
        assert_eq!(settings.jwt.secret, "test-jwt-secret-key");
        assert_eq!(settings.rate_limit.requests_per_minute, 200);

        // Clean up
        clear_all_settings_env_vars();
    }

    /// Test default values are used when env vars are not set
    #[test]
    fn test_settings_default_values() {
        // Ensure env vars are cleared - this is critical for the test
        clear_all_settings_env_vars();

        let settings = Settings::from_env();

        assert!(settings.is_ok());
        let settings = settings.unwrap();

        // Check default values
        assert_eq!(settings.server.host, "0.0.0.0");
        assert_eq!(settings.server.port, 3000);
        assert_eq!(settings.server.env, "local");
        assert_eq!(settings.database.host, "localhost");
        assert_eq!(settings.database.port, 5432);
        assert_eq!(settings.database.user, "postgres");
        assert_eq!(settings.database.password, "postgres");
        assert_eq!(settings.database.name, "postgres");
        assert!(!settings.jwt.secret.is_empty());
        assert_eq!(settings.rate_limit.requests_per_minute, 100);
        
        // Clean up
        clear_all_settings_env_vars();
    }

    /// Test invalid port number in environment
    #[test]
    fn test_settings_invalid_port() {
        clear_all_settings_env_vars();
        std::env::set_var("SERVER_PORT", "invalid");
        
        let settings = Settings::from_env();
        
        assert!(settings.is_err(), "Settings should fail with invalid port");
        
        clear_all_settings_env_vars();
    }

    /// Test invalid rate limit value in environment
    #[test]
    fn test_settings_invalid_rate_limit() {
        clear_all_settings_env_vars();
        std::env::set_var("RATE_LIMIT_REQUESTS_PER_MINUTE", "not-a-number");
        
        let settings = Settings::from_env();
        
        assert!(settings.is_err(), "Settings should fail with invalid rate limit");
        
        clear_all_settings_env_vars();
    }
}

// ============================================================================
// Settings Database URL Tests
// ============================================================================

mod settings_database_url_tests {
    use super::*;

    /// Test database URL generation
    #[test]
    fn test_settings_database_url() {
        let settings = Settings {
            server: zercle_rust_template::config::ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 3000,
                env: "test".to_string(),
            },
            database: zercle_rust_template::config::DatabaseConfig {
                driver: "postgres".to_string(),
                host: "localhost".to_string(),
                port: 5432,
                user: "postgres".to_string(),
                password: "postgres".to_string(),
                name: "mydb".to_string(),
                pool_size: 10,
                ssl_mode: "disable".to_string(),
            },
            jwt: zercle_rust_template::config::JwtConfig {
                secret: "secret".to_string(),
                expiration_hours: 24,
            },
            logging: zercle_rust_template::config::LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
            },
            cors: zercle_rust_template::config::CorsConfig {
                allowed_origins: vec!["http://localhost:3000".to_string()],
            },
            rate_limit: zercle_rust_template::config::RateLimitConfig {
                requests_per_minute: 100,
            },
            argon2id: zercle_rust_template::config::Argon2idConfig {
                memory_kb: 19456,
                iterations: 2,
                parallelism: 1,
            },
        };

        let url = settings.database_url();
        
        assert!(url.contains("postgres://"));
        assert!(url.contains("postgres:postgres@localhost:5432/mydb"));
        assert!(url.contains("sslmode=disable"));
    }

    /// Test database URL with special characters in password
    #[test]
    fn test_settings_database_url_special_chars() {
        let settings = Settings {
            server: zercle_rust_template::config::ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 3000,
                env: "test".to_string(),
            },
            database: zercle_rust_template::config::DatabaseConfig {
                driver: "postgres".to_string(),
                host: "db.example.com".to_string(),
                port: 5432,
                user: "admin".to_string(),
                password: "p@ss:word/123".to_string(),
                name: "production".to_string(),
                pool_size: 20,
                ssl_mode: "require".to_string(),
            },
            jwt: zercle_rust_template::config::JwtConfig {
                secret: "secret".to_string(),
                expiration_hours: 24,
            },
            logging: zercle_rust_template::config::LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
            },
            cors: zercle_rust_template::config::CorsConfig {
                allowed_origins: vec!["http://localhost:3000".to_string()],
            },
            rate_limit: zercle_rust_template::config::RateLimitConfig {
                requests_per_minute: 100,
            },
            argon2id: zercle_rust_template::config::Argon2idConfig {
                memory_kb: 19456,
                iterations: 2,
                parallelism: 1,
            },
        };

        let url = settings.database_url();
        
        assert!(url.contains("admin:p@ss:word/123@db.example.com:5432/production"));
        assert!(url.contains("sslmode=require"));
    }

    /// Test database URL format consistency
    #[test]
    fn test_settings_database_url_format() {
        let settings = Settings {
            server: zercle_rust_template::config::ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 3000,
                env: "test".to_string(),
            },
            database: zercle_rust_template::config::DatabaseConfig {
                driver: "postgres".to_string(),
                host: "localhost".to_string(),
                port: 5432,
                user: "user".to_string(),
                password: "pass".to_string(),
                name: "db".to_string(),
                pool_size: 5,
                ssl_mode: "disable".to_string(),
            },
            jwt: zercle_rust_template::config::JwtConfig {
                secret: "secret".to_string(),
                expiration_hours: 24,
            },
            logging: zercle_rust_template::config::LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
            },
            cors: zercle_rust_template::config::CorsConfig {
                allowed_origins: vec!["http://localhost:3000".to_string()],
            },
            rate_limit: zercle_rust_template::config::RateLimitConfig {
                requests_per_minute: 100,
            },
            argon2id: zercle_rust_template::config::Argon2idConfig {
                memory_kb: 19456,
                iterations: 2,
                parallelism: 1,
            },
        };

        let url = settings.database_url();
        
        // Expected format: postgres://user:pass@host:port/dbname?sslmode=value
        assert!(url.starts_with("postgres://"));
        assert!(url.contains("@"));
        assert!(url.contains(":5432/"));
        assert!(url.contains("?sslmode="));
    }
}

// ============================================================================
// Settings Validation Tests
// ============================================================================

mod settings_validation_tests {
    use super::*;

    /// Test settings is_dev returns true for dev environment
    #[test]
    fn test_settings_is_dev() {
        let settings = Settings {
            server: zercle_rust_template::config::ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 3000,
                env: "dev".to_string(),
            },
            database: zercle_rust_template::config::DatabaseConfig {
                driver: "postgres".to_string(),
                host: "localhost".to_string(),
                port: 5432,
                user: "postgres".to_string(),
                password: "postgres".to_string(),
                name: "postgres".to_string(),
                pool_size: 10,
                ssl_mode: "disable".to_string(),
            },
            jwt: zercle_rust_template::config::JwtConfig {
                secret: "secret".to_string(),
                expiration_hours: 24,
            },
            logging: zercle_rust_template::config::LoggingConfig {
                level: "debug".to_string(),
                format: "json".to_string(),
            },
            cors: zercle_rust_template::config::CorsConfig {
                allowed_origins: vec!["*".to_string()],
            },
            rate_limit: zercle_rust_template::config::RateLimitConfig {
                requests_per_minute: 100,
            },
            argon2id: zercle_rust_template::config::Argon2idConfig {
                memory_kb: 19456,
                iterations: 2,
                parallelism: 1,
            },
        };

        assert!(settings.is_dev());
        assert!(!settings.is_prod());
    }

    /// Test settings is_dev returns true for local environment
    #[test]
    fn test_settings_is_local() {
        let settings = Settings {
            server: zercle_rust_template::config::ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 3000,
                env: "local".to_string(),
            },
            database: zercle_rust_template::config::DatabaseConfig {
                driver: "postgres".to_string(),
                host: "localhost".to_string(),
                port: 5432,
                user: "postgres".to_string(),
                password: "postgres".to_string(),
                name: "postgres".to_string(),
                pool_size: 10,
                ssl_mode: "disable".to_string(),
            },
            jwt: zercle_rust_template::config::JwtConfig {
                secret: "secret".to_string(),
                expiration_hours: 24,
            },
            logging: zercle_rust_template::config::LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
            },
            cors: zercle_rust_template::config::CorsConfig {
                allowed_origins: vec![],
            },
            rate_limit: zercle_rust_template::config::RateLimitConfig {
                requests_per_minute: 100,
            },
            argon2id: zercle_rust_template::config::Argon2idConfig {
                memory_kb: 19456,
                iterations: 2,
                parallelism: 1,
            },
        };

        assert!(settings.is_dev());
        assert!(!settings.is_prod());
    }

    /// Test settings is_prod returns true for prod environment
    #[test]
    fn test_settings_is_prod() {
        let settings = Settings {
            server: zercle_rust_template::config::ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 3000,
                env: "prod".to_string(),
            },
            database: zercle_rust_template::config::DatabaseConfig {
                driver: "postgres".to_string(),
                host: "prod-db.example.com".to_string(),
                port: 5432,
                user: "prod_user".to_string(),
                password: "secure_password".to_string(),
                name: "prod_db".to_string(),
                pool_size: 20,
                ssl_mode: "require".to_string(),
            },
            jwt: zercle_rust_template::config::JwtConfig {
                secret: "production-secret".to_string(),
                expiration_hours: 24,
            },
            logging: zercle_rust_template::config::LoggingConfig {
                level: "warn".to_string(),
                format: "json".to_string(),
            },
            cors: zercle_rust_template::config::CorsConfig {
                allowed_origins: vec!["https://example.com".to_string()],
            },
            rate_limit: zercle_rust_template::config::RateLimitConfig {
                requests_per_minute: 200,
            },
            argon2id: zercle_rust_template::config::Argon2idConfig {
                memory_kb: 65536,
                iterations: 3,
                parallelism: 2,
            },
        };

        assert!(!settings.is_dev());
        assert!(settings.is_prod());
    }
}

// ============================================================================
// Settings File Loading Tests
// ============================================================================

mod settings_file_tests {
    use super::*;

    /// Test loading settings from YAML file
    #[test]
    fn test_settings_from_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test.yaml");
        
        let yaml_content = r#"
server:
  host: "192.168.1.1"
  port: 9090
  env: "staging"

database:
  driver: "postgres"
  host: "staging-db.example.com"
  port: 5432
  user: "staging_user"
  password: "staging_pass"
  name: "staging_db"
  pool_size: 15
  ssl_mode: "require"

jwt:
  secret: "staging-jwt-secret"
  expiration_hours: 12

logging:
  level: "debug"
  format: "json"

cors:
  allowed_origins:
    - "http://staging.example.com"

rate_limit:
  requests_per_minute: 150

argon2id:
  memory_kb: 32768
  iterations: 3
  parallelism: 2
"#;

        std::fs::write(&config_path, yaml_content).unwrap();

        let settings = Settings::from_file(&config_path);

        assert!(settings.is_ok());
        let settings = settings.unwrap();

        assert_eq!(settings.server.host, "192.168.1.1");
        assert_eq!(settings.server.port, 9090);
        assert_eq!(settings.server.env, "staging");
        assert_eq!(settings.database.host, "staging-db.example.com");
        assert_eq!(settings.database.user, "staging_user");
        assert_eq!(settings.database.pool_size, 15);
        assert_eq!(settings.jwt.secret, "staging-jwt-secret");
        assert_eq!(settings.jwt.expiration_hours, 12);
        assert_eq!(settings.logging.level, "debug");
        assert_eq!(settings.rate_limit.requests_per_minute, 150);
        assert_eq!(settings.argon2id.memory_kb, 32768);
    }

    /// Test loading settings from non-existent file
    #[test]
    fn test_settings_from_nonexistent_file() {
        let non_existent_path = Path::new("/nonexistent/config.yaml");
        
        let settings = Settings::from_file(non_existent_path);
        
        assert!(settings.is_err());
    }

    /// Test loading settings from invalid YAML
    #[test]
    fn test_settings_from_invalid_yaml() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("invalid.yaml");
        
        let invalid_yaml = r#"
server:
  host: "localhost"
  port: [invalid
"#;

        std::fs::write(&config_path, invalid_yaml).unwrap();

        let settings = Settings::from_file(&config_path);
        
        assert!(settings.is_err());
    }

    /// Test loading settings with environment overrides
    #[test]
    fn test_settings_from_file_with_env() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");
        
        let yaml_content = r#"
server:
  host: "0.0.0.0"
  port: 3000
  env: "test"

database:
  driver: "postgres"
  host: "localhost"
  port: 5432
  user: "postgres"
  password: "postgres"
  name: "postgres"
  pool_size: 10
  ssl_mode: "disable"

jwt:
  secret: "file-secret"
  expiration_hours: 24

logging:
  level: "info"
  format: "json"

cors:
  allowed_origins:
    - "*"

rate_limit:
  requests_per_minute: 100

argon2id:
  memory_kb: 19456
  iterations: 2
  parallelism: 1
"#;

        std::fs::write(&config_path, yaml_content).unwrap();

        // Clear env vars first, then set only what we need for override
        clear_all_settings_env_vars();
        
        // Set environment variable to override - config crate uses double underscore for nesting
        // so SERVER__PORT maps to server.port
        std::env::set_var("SERVER__PORT", "9999");
        std::env::set_var("JWT__SECRET", "env-override-secret");

        let settings = Settings::from_file_with_env(&config_path);

        assert!(settings.is_ok());
        let settings = settings.unwrap();

        // File value for host
        assert_eq!(settings.server.host, "0.0.0.0");
        // Environment override for port
        assert_eq!(settings.server.port, 9999);
        // Environment override for JWT secret
        assert_eq!(settings.jwt.secret, "env-override-secret");

        clear_all_settings_env_vars();
    }
}

// ============================================================================
// Settings CORS Tests
// ============================================================================

mod settings_cors_tests {
    use super::*;

    /// Test CORS allowed origins parsing from comma-separated string
    #[test]
    fn test_cors_allowed_origins_parsing() {
        clear_all_settings_env_vars();
        std::env::set_var("CORS_ALLOWED_ORIGINS", "http://localhost:3000,http://localhost:8080,https://example.com");

        let settings = Settings::from_env().unwrap();

        assert_eq!(settings.cors.allowed_origins.len(), 3);
        assert!(settings.cors.allowed_origins.contains(&"http://localhost:3000".to_string()));
        assert!(settings.cors.allowed_origins.contains(&"http://localhost:8080".to_string()));
        assert!(settings.cors.allowed_origins.contains(&"https://example.com".to_string()));

        clear_all_settings_env_vars();
    }

    /// Test CORS with single origin
    #[test]
    fn test_cors_single_origin() {
        clear_all_settings_env_vars();
        std::env::set_var("CORS_ALLOWED_ORIGINS", "https://api.example.com");

        let settings = Settings::from_env().unwrap();

        assert_eq!(settings.cors.allowed_origins.len(), 1);
        assert_eq!(settings.cors.allowed_origins[0], "https://api.example.com");

        clear_all_settings_env_vars();
    }

    /// Test CORS with whitespace in origins
    #[test]
    fn test_cors_origins_with_whitespace() {
        clear_all_settings_env_vars();
        std::env::set_var("CORS_ALLOWED_ORIGINS", " http://localhost:3000 , http://localhost:8080 ");

        let settings = Settings::from_env().unwrap();

        assert_eq!(settings.cors.allowed_origins.len(), 2);
        assert!(settings.cors.allowed_origins.contains(&"http://localhost:3000".to_string()));
        assert!(settings.cors.allowed_origins.contains(&"http://localhost:8080".to_string()));

        clear_all_settings_env_vars();
    }
}

// ============================================================================
// Settings Argon2id Tests
// ============================================================================

mod settings_argon2id_tests {
    use super::*;

    /// Test Argon2id configuration
    #[test]
    fn test_argon2id_config() {
        let settings = Settings {
            server: zercle_rust_template::config::ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 3000,
                env: "test".to_string(),
            },
            database: zercle_rust_template::config::DatabaseConfig {
                driver: "postgres".to_string(),
                host: "localhost".to_string(),
                port: 5432,
                user: "postgres".to_string(),
                password: "postgres".to_string(),
                name: "postgres".to_string(),
                pool_size: 10,
                ssl_mode: "disable".to_string(),
            },
            jwt: zercle_rust_template::config::JwtConfig {
                secret: "secret".to_string(),
                expiration_hours: 24,
            },
            logging: zercle_rust_template::config::LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
            },
            cors: zercle_rust_template::config::CorsConfig {
                allowed_origins: vec![],
            },
            rate_limit: zercle_rust_template::config::RateLimitConfig {
                requests_per_minute: 100,
            },
            argon2id: zercle_rust_template::config::Argon2idConfig {
                memory_kb: 65536,
                iterations: 4,
                parallelism: 2,
            },
        };

        assert_eq!(settings.argon2id.memory_kb, 65536);
        assert_eq!(settings.argon2id.iterations, 4);
        assert_eq!(settings.argon2id.parallelism, 2);
    }

    /// Test Argon2id default values
    #[test]
    fn test_argon2id_default_values() {
        clear_all_settings_env_vars();
        
        let settings = Settings::from_env().unwrap();

        // Default values from Settings::from_env()
        assert_eq!(settings.argon2id.memory_kb, 19456);
        assert_eq!(settings.argon2id.iterations, 2);
        assert_eq!(settings.argon2id.parallelism, 1);
        
        clear_all_settings_env_vars();
    }
}
