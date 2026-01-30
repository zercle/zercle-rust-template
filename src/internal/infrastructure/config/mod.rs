use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub app: AppConfig,
    pub database: DatabaseConfig,
    pub jwt: JwtConfig,
    pub argon2id: Argon2idConfig,
    pub logging: LoggingConfig,
    pub cors: CorsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub env: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub name: String,
    pub sslmode: String,
    pub max_open_conns: u32,
    pub max_idle_conns: u32,
    pub conn_max_lifetime: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    pub secret: String,
    pub access_token_expiry: u64,
    pub refresh_token_expiry: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Argon2idConfig {
    pub memory: u32,
    pub iterations: u32,
    pub parallelism: u32,
    pub salt_length: u32,
    pub key_length: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
    pub allowed_methods: Vec<String>,
    pub allowed_headers: Vec<String>,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let env = std::env::var("APP_ENV").unwrap_or_else(|_| "local".to_string());
        let config_path = format!("configs/{env}.yaml");

        let path = Path::new(&config_path);
        if path.exists() {
            Self::from_file(path)
        } else {
            Self::from_env()
        }
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let _ = dotenv::dotenv().ok();

        let config = config::Config::builder()
            .add_source(config::File::from(path.as_ref()))
            .add_source(config::Environment::default().separator("__"))
            .build()?;

        let settings: Config = config.try_deserialize()?;
        Ok(settings)
    }

    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Config {
            app: AppConfig {
                host: std::env::var("APP_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
                port: std::env::var("APP_PORT")
                    .unwrap_or_else(|_| "3000".to_string())
                    .parse()?,
                env: std::env::var("APP_ENV").unwrap_or_else(|_| "local".to_string()),
            },
            database: DatabaseConfig {
                host: std::env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_string()),
                port: std::env::var("DB_PORT")
                    .unwrap_or_else(|_| "5432".to_string())
                    .parse()?,
                user: std::env::var("DB_USER").unwrap_or_else(|_| "postgres".to_string()),
                password: std::env::var("DB_PASSWORD").unwrap_or_else(|_| "postgres".to_string()),
                name: std::env::var("DB_NAME").unwrap_or_else(|_| "zercle".to_string()),
                sslmode: std::env::var("DB_SSLMODE").unwrap_or_else(|_| "disable".to_string()),
                max_open_conns: std::env::var("DB_MAX_OPEN_CONNS")
                    .unwrap_or_else(|_| "25".to_string())
                    .parse()?,
                max_idle_conns: std::env::var("DB_MAX_IDLE_CONNS")
                    .unwrap_or_else(|_| "5".to_string())
                    .parse()?,
                conn_max_lifetime: std::env::var("DB_CONN_MAX_LIFETIME")
                    .unwrap_or_else(|_| "5m".to_string()),
            },
            jwt: JwtConfig {
                secret: std::env::var("JWT_SECRET")
                    .unwrap_or_else(|_| "your-jwt-secret-key".to_string()),
                access_token_expiry: std::env::var("JWT_ACCESS_TOKEN_EXPIRY")
                    .unwrap_or_else(|_| "3600".to_string())
                    .parse()?,
                refresh_token_expiry: std::env::var("JWT_REFRESH_TOKEN_EXPIRY")
                    .unwrap_or_else(|_| "604800".to_string())
                    .parse()?,
            },
            argon2id: Argon2idConfig {
                memory: std::env::var("ARGON2ID_MEMORY")
                    .unwrap_or_else(|_| "19456".to_string())
                    .parse()?,
                iterations: std::env::var("ARGON2ID_ITERATIONS")
                    .unwrap_or_else(|_| "2".to_string())
                    .parse()?,
                parallelism: std::env::var("ARGON2ID_PARALLELISM")
                    .unwrap_or_else(|_| "1".to_string())
                    .parse()?,
                salt_length: std::env::var("ARGON2ID_SALT_LENGTH")
                    .unwrap_or_else(|_| "16".to_string())
                    .parse()?,
                key_length: std::env::var("ARGON2ID_KEY_LENGTH")
                    .unwrap_or_else(|_| "32".to_string())
                    .parse()?,
            },
            logging: LoggingConfig {
                level: std::env::var("LOG_LEVEL").unwrap_or_else(|_| "debug".to_string()),
                format: std::env::var("LOG_FORMAT").unwrap_or_else(|_| "console".to_string()),
            },
            cors: CorsConfig {
                allowed_origins: std::env::var("CORS_ALLOWED_ORIGINS")
                    .unwrap_or_else(|_| "*".to_string())
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect(),
                allowed_methods: vec![
                    "GET".to_string(),
                    "POST".to_string(),
                    "PUT".to_string(),
                    "PATCH".to_string(),
                    "DELETE".to_string(),
                    "OPTIONS".to_string(),
                ],
                allowed_headers: vec![
                    "Origin".to_string(),
                    "Content-Type".to_string(),
                    "Accept".to_string(),
                    "Authorization".to_string(),
                ],
            },
        })
    }

    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}?sslmode={}",
            self.database.user,
            self.database.password,
            self.database.host,
            self.database.port,
            self.database.name,
            self.database.sslmode
        )
    }
}
