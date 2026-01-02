use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub jwt: JwtConfig,
    pub logging: LoggingConfig,
    pub cors: CorsConfig,
    pub rate_limit: RateLimitConfig,
    pub argon2id: Argon2idConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub env: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub driver: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub name: String,
    pub pool_size: u32,
    pub ssl_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    pub secret: String,
    pub expiration_hours: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Argon2idConfig {
    pub memory_kb: u32,
    pub iterations: u32,
    pub parallelism: u32,
}

impl Settings {
    pub fn from_env() -> anyhow::Result<Self> {
        dotenv::dotenv().ok();

        Ok(Settings {
            server: ServerConfig {
                host: std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
                port: std::env::var("SERVER_PORT")
                    .unwrap_or_else(|_| "3000".to_string())
                    .parse()?,
                env: std::env::var("SERVER_ENV").unwrap_or_else(|_| "local".to_string()),
            },
            database: DatabaseConfig {
                driver: std::env::var("DB_DRIVER").unwrap_or_else(|_| "postgres".to_string()),
                host: std::env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_string()),
                port: std::env::var("DB_PORT")
                    .unwrap_or_else(|_| "5432".to_string())
                    .parse()?,
                user: std::env::var("DB_USER").unwrap_or_else(|_| "postgres".to_string()),
                password: std::env::var("DB_PASSWORD").unwrap_or_else(|_| "postgres".to_string()),
                name: std::env::var("DB_NAME").unwrap_or_else(|_| "postgres".to_string()),
                pool_size: std::env::var("DB_POOL_SIZE")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse()?,
                ssl_mode: std::env::var("DB_SSL_MODE").unwrap_or_else(|_| "disable".to_string()),
            },
            jwt: JwtConfig {
                secret: std::env::var("JWT_SECRET").unwrap_or_else(|_| {
                    "your-super-secret-jwt-key-for-development-only-123".to_string()
                }),
                expiration_hours: std::env::var("JWT_EXPIRATION_HOURS")
                    .unwrap_or_else(|_| "24".to_string())
                    .parse()?,
            },
            logging: LoggingConfig {
                level: std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
                format: std::env::var("LOG_FORMAT").unwrap_or_else(|_| "json".to_string()),
            },
            cors: CorsConfig {
                allowed_origins: std::env::var("CORS_ALLOWED_ORIGINS")
                    .unwrap_or_else(|_| "http://localhost:3000,http://localhost:8080".to_string())
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect(),
            },
            rate_limit: RateLimitConfig {
                requests_per_minute: std::env::var("RATE_LIMIT_REQUESTS_PER_MINUTE")
                    .unwrap_or_else(|_| "100".to_string())
                    .parse()?,
            },
            argon2id: Argon2idConfig {
                memory_kb: std::env::var("ARGON2ID_MEMORY_KB")
                    .unwrap_or_else(|_| "19456".to_string())
                    .parse()?,
                iterations: std::env::var("ARGON2ID_ITERATIONS")
                    .unwrap_or_else(|_| "2".to_string())
                    .parse()?,
                parallelism: std::env::var("ARGON2ID_PARALLELISM")
                    .unwrap_or_else(|_| "1".to_string())
                    .parse()?,
            },
        })
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let config = config::Config::builder()
            .add_source(config::File::from(path.as_ref()))
            .build()?;

        let settings: Settings = config.try_deserialize()?;
        Ok(settings)
    }

    pub fn from_file_with_env<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        dotenv::dotenv().ok();

        let config = config::Config::builder()
            .add_source(config::File::from(path.as_ref()))
            .add_source(config::Environment::default().separator("__"))
            .build()?;

        let settings: Settings = config.try_deserialize()?;
        Ok(settings)
    }

    pub fn load() -> anyhow::Result<Self> {
        let env = std::env::var("SERVER_ENV").unwrap_or_else(|_| "local".to_string());
        let config_path = format!("configs/{}.yaml", env);

        let path = Path::new(&config_path);
        if path.exists() {
            Self::from_file_with_env(path)
        } else {
            Self::from_env()
        }
    }

    pub fn database_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}?sslmode={}",
            self.database.user,
            self.database.password,
            self.database.host,
            self.database.port,
            self.database.name,
            self.database.ssl_mode
        )
    }

    pub fn is_dev(&self) -> bool {
        self.server.env == "dev" || self.server.env == "local"
    }

    pub fn is_prod(&self) -> bool {
        self.server.env == "prod"
    }
}
