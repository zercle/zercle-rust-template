use crate::config::Settings;
use anyhow::{Context, Result};
use config::{Config, Environment, File};
use std::path::Path;

/// Load configuration from a YAML file
///
/// This function reads configuration from the specified YAML file path.
/// The file must contain all required configuration fields.
///
/// # Arguments
/// * `path` - Path to the YAML configuration file
///
/// # Returns
/// * `Result<Settings>` - The loaded settings or an error
///
/// # Example
/// ```no_run
/// use zercle_rust_template::infrastructure::config::load_from_file;
///
/// let settings = load_from_file("configs/dev.yaml").unwrap();
/// ```
pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Settings> {
    let config = Config::builder()
        .add_source(File::from(path.as_ref()))
        .build()
        .context("Failed to build configuration from file")?;

    let settings: Settings = config
        .try_deserialize()
        .context("Failed to deserialize configuration")?;

    Ok(settings)
}

/// Load configuration from environment variables
///
/// This function reads configuration from environment variables.
/// Environment variables take precedence over file-based configuration.
/// Uses dotenv to load .env file if present.
///
/// # Returns
/// * `Result<Settings>` - The loaded settings or an error
///
/// # Example
/// ```no_run
/// use zercle_rust_template::infrastructure::config::load_from_env;
///
/// let settings = load_from_env().unwrap();
/// ```
pub fn load_from_env() -> Result<Settings> {
    // Load .env file if it exists
    dotenv::dotenv().ok();

    let config = Config::builder()
        .add_source(Environment::default().separator("__"))
        .build()
        .context("Failed to build configuration from environment")?;

    let settings: Settings = config
        .try_deserialize()
        .context("Failed to deserialize configuration")?;

    Ok(settings)
}

/// Load configuration from a YAML file with environment variable overrides
///
/// This function reads configuration from a YAML file and then overrides
/// any values with environment variables. Environment variables use double
/// underscore as separator (e.g., `SERVER__HOST` for `server.host`).
///
/// # Arguments
/// * `path` - Path to the YAML configuration file
///
/// # Returns
/// * `Result<Settings>` - The loaded settings or an error
///
/// # Example
/// ```no_run
/// use zercle_rust_template::infrastructure::config::load_from_file_with_env;
///
/// let settings = load_from_file_with_env("configs/dev.yaml").unwrap();
/// ```
pub fn load_from_file_with_env<P: AsRef<Path>>(path: P) -> Result<Settings> {
    // Load .env file if it exists
    dotenv::dotenv().ok();

    let config = Config::builder()
        .add_source(File::from(path.as_ref()))
        .add_source(Environment::default().separator("__"))
        .build()
        .context("Failed to build configuration from file and environment")?;

    let settings: Settings = config
        .try_deserialize()
        .context("Failed to deserialize configuration")?;

    Ok(settings)
}

/// Load configuration automatically based on environment
///
/// This function determines the current environment from the SERVER_ENV
/// environment variable (defaults to "local") and loads the corresponding
/// configuration file from the configs/ directory.
///
/// Configuration loading priority:
/// 1. Environment variables (SERVER_ENV)
/// 2. YAML file (configs/{env}.yaml)
/// 3. Environment variable overrides
/// 4. Default values
///
/// # Returns
/// * `Result<Settings>` - The loaded settings or an error
///
/// # Example
/// ```no_run
/// use zercle_rust_template::infrastructure::config::load_config;
///
/// let settings = load_config().unwrap();
/// ```
pub fn load_config() -> Result<Settings> {
    // Get environment from SERVER_ENV variable or default to "local"
    let env = std::env::var("SERVER_ENV").unwrap_or_else(|_| "local".to_string());

    // Construct config file path
    let config_path = format!("configs/{}.yaml", env);
    let path = Path::new(&config_path);

    // Load configuration based on whether file exists
    if path.exists() {
        load_from_file_with_env(path)
    } else {
        // Fallback to environment-only configuration
        load_from_env()
    }
}

/// Validate configuration settings
///
/// This function performs validation checks on the loaded configuration
/// to ensure all required fields are present and valid.
///
/// # Arguments
/// * `settings` - The settings to validate
///
/// # Returns
/// * `Result<()>` - Ok if valid, error otherwise
///
/// # Example
/// ```no_run
/// use zercle_rust_template::infrastructure::config::{load_config, validate_config};
///
/// let settings = load_config().unwrap();
/// validate_config(&settings).unwrap();
/// ```
pub fn validate_config(settings: &Settings) -> Result<()> {
    // Validate server configuration
    if settings.server.port == 0 {
        anyhow::bail!("Server port cannot be 0");
    }

    // Validate database configuration
    if settings.database.host.is_empty() {
        anyhow::bail!("Database host cannot be empty");
    }
    if settings.database.name.is_empty() {
        anyhow::bail!("Database name cannot be empty");
    }
    if settings.database.user.is_empty() {
        anyhow::bail!("Database user cannot be empty");
    }
    if settings.database.pool_size == 0 {
        anyhow::bail!("Database pool size cannot be 0");
    }

    // Validate JWT configuration
    if settings.jwt.secret.len() < 32 {
        anyhow::bail!("JWT secret must be at least 32 characters");
    }
    if settings.jwt.expiration_hours == 0 {
        anyhow::bail!("JWT expiration hours cannot be 0");
    }

    // Validate CORS configuration
    if settings.cors.allowed_origins.is_empty() {
        anyhow::bail!("CORS allowed origins cannot be empty");
    }

    // Validate rate limit configuration
    if settings.rate_limit.requests_per_minute == 0 {
        anyhow::bail!("Rate limit requests per minute cannot be 0");
    }

    // Validate Argon2id configuration
    if settings.argon2id.memory_kb < 1024 {
        anyhow::bail!("Argon2id memory must be at least 1024 KB");
    }
    if settings.argon2id.iterations == 0 {
        anyhow::bail!("Argon2id iterations cannot be 0");
    }
    if settings.argon2id.parallelism == 0 {
        anyhow::bail!("Argon2id parallelism cannot be 0");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_validate_config_valid() {
        let settings = Settings {
            server: crate::config::ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 3000,
                env: "test".to_string(),
            },
            database: crate::config::DatabaseConfig {
                driver: "postgres".to_string(),
                host: "localhost".to_string(),
                port: 5432,
                user: "postgres".to_string(),
                password: "password".to_string(),
                name: "testdb".to_string(),
                pool_size: 10,
                ssl_mode: "disable".to_string(),
            },
            jwt: crate::config::JwtConfig {
                secret: "a".repeat(32),
                expiration_hours: 24,
            },
            logging: crate::config::LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
            },
            cors: crate::config::CorsConfig {
                allowed_origins: vec!["http://localhost:3000".to_string()],
            },
            rate_limit: crate::config::RateLimitConfig {
                requests_per_minute: 100,
            },
            argon2id: crate::config::Argon2idConfig {
                memory_kb: 19456,
                iterations: 2,
                parallelism: 1,
            },
        };

        assert!(validate_config(&settings).is_ok());
    }

    #[test]
    fn test_validate_config_invalid_jwt_secret() {
        let settings = Settings {
            server: crate::config::ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 3000,
                env: "test".to_string(),
            },
            database: crate::config::DatabaseConfig {
                driver: "postgres".to_string(),
                host: "localhost".to_string(),
                port: 5432,
                user: "postgres".to_string(),
                password: "password".to_string(),
                name: "testdb".to_string(),
                pool_size: 10,
                ssl_mode: "disable".to_string(),
            },
            jwt: crate::config::JwtConfig {
                secret: "short".to_string(),
                expiration_hours: 24,
            },
            logging: crate::config::LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
            },
            cors: crate::config::CorsConfig {
                allowed_origins: vec!["http://localhost:3000".to_string()],
            },
            rate_limit: crate::config::RateLimitConfig {
                requests_per_minute: 100,
            },
            argon2id: crate::config::Argon2idConfig {
                memory_kb: 19456,
                iterations: 2,
                parallelism: 1,
            },
        };

        assert!(validate_config(&settings).is_err());
    }

    #[test]
    fn test_load_from_file() {
        let yaml_content = r#"
server:
  host: "0.0.0.0"
  port: 3000
  env: "test"

database:
  host: "localhost"
  port: 5432
  user: "postgres"
  password: "password"
  name: "testdb"
  driver: "postgres"
  pool_size: 10
  ssl_mode: "disable"

jwt:
  secret: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  expiration_hours: 24

logging:
  level: "info"
  format: "json"

cors:
  allowed_origins:
    - "http://localhost:3000"

rate_limit:
  requests_per_minute: 100

argon2id:
  memory_kb: 19456
  iterations: 2
  parallelism: 1
"#;

        // Use a temp file with .yaml extension so config crate recognizes it
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.into_temp_path();
        let yaml_path = temp_path.with_extension("yaml");
        fs::write(&yaml_path, yaml_content).unwrap();

        let result = load_from_file(&yaml_path);
        assert!(result.is_ok(), "Failed to load config: {result:?}");
    }
}
