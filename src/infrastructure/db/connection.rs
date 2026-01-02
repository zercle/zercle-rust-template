use crate::config::Settings;
use anyhow::{Context, Result};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{Pool, Postgres};
use std::time::Duration;

/// Database connection pool type alias
pub type DbPool = Pool<Postgres>;

/// Database connection manager
///
/// This struct manages the PostgreSQL connection pool and provides
/// methods for connecting, health checking, and accessing the pool.
pub struct Database {
    pool: DbPool,
}

impl Database {
    /// Create a new database connection manager
    ///
    /// # Arguments
    /// * `settings` - Application settings containing database configuration
    ///
    /// # Returns
    /// `Result<Self>` - The database manager or an error if connection fails
    ///
    /// # Example
    /// ```no_run
    /// use zercle_rust_template::infrastructure::db::connection::Database;
    /// use zercle_rust_template::config::Settings;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let settings = Settings::load()?;
    /// let db = Database::connect(&settings).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect(settings: &Settings) -> Result<Self> {
        let database_url = settings.database_url();
        
        // Parse the database URL into connection options
        let options: PgConnectOptions = database_url
            .parse()
            .context("Failed to parse database URL")?;

        // Configure connection pool
        let pool = PgPoolOptions::new()
            .max_connections(settings.database.pool_size as u32)
            .acquire_timeout(Duration::from_secs(30))
            .idle_timeout(Duration::from_secs(600))
            .max_lifetime(Duration::from_secs(3600))
            .test_before_acquire(true)
            .connect_with(options)
            .await
            .context("Failed to create database connection pool")?;

        Ok(Self { pool })
    }

    /// Create a new database connection manager from a database URL
    ///
    /// # Arguments
    /// * `database_url` - PostgreSQL connection URL
    ///
    /// # Returns
    /// `Result<Self>` - The database manager or an error if connection fails
    ///
    /// # Example
    /// ```no_run
    /// use zercle_rust_template::infrastructure::db::connection::Database;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let db = Database::from_url("postgres://user:pass@localhost/db").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn from_url(database_url: &str) -> Result<Self> {
        let options: PgConnectOptions = database_url
            .parse()
            .context("Failed to parse database URL")?;

        let pool = PgPoolOptions::new()
            .max_connections(10)
            .acquire_timeout(Duration::from_secs(30))
            .idle_timeout(Duration::from_secs(600))
            .max_lifetime(Duration::from_secs(3600))
            .test_before_acquire(true)
            .connect_with(options)
            .await
            .context("Failed to create database connection pool")?;

        Ok(Self { pool })
    }

    /// Get a reference to the connection pool
    ///
    /// # Returns
    /// Reference to the database connection pool
    pub fn pool(&self) -> &DbPool {
        &self.pool
    }

    /// Get a mutable reference to the connection pool
    ///
    /// # Returns
    /// Mutable reference to the database connection pool
    pub fn pool_mut(&mut self) -> &mut DbPool {
        &mut self.pool
    }

    /// Perform a health check on the database connection
    ///
    /// This method executes a simple query to verify that the database
    /// is accessible and responding correctly.
    ///
    /// # Returns
    /// `Result<()>` - Ok if healthy, error otherwise
    ///
    /// # Example
    /// ```no_run
    /// use zercle_rust_template::infrastructure::db::connection::Database;
    ///
    /// # async fn example(db: &Database) -> anyhow::Result<()> {
    /// if db.health_check().await.is_ok() {
    ///     println!("Database is healthy");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .context("Database health check failed")?;

        Ok(())
    }

    /// Get connection pool size information
    ///
    /// # Returns
    /// A tuple of (size, idle) where:
    /// - size: Total number of connections in the pool
    /// - idle: Number of idle connections
    pub fn pool_size(&self) -> (u32, u32) {
        (
            self.pool.size() as u32,
            self.pool.num_idle() as u32
        )
    }

    /// Close the database connection pool
    ///
    /// This method gracefully closes all connections in the pool.
    /// It should be called when shutting down the application.
    ///
    /// # Example
    /// ```no_run
    /// use zercle_rust_template::infrastructure::db::connection::Database;
    ///
    /// # async fn example(mut db: Database) -> anyhow::Result<()> {
    /// db.close().await;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn close(self) {
        self.pool.close().await;
    }
}

/// Create a database connection pool from settings
///
/// This is a convenience function that creates a Database instance
/// from application settings.
///
/// # Arguments
/// * `settings` - Application settings containing database configuration
///
/// # Returns
/// `Result<DbPool>` - The connection pool or an error if connection fails
///
/// # Example
/// ```no_run
/// use zercle_rust_template::infrastructure::db::connection::connect;
/// use zercle_rust_template::config::Settings;
///
/// # async fn example() -> anyhow::Result<()> {
/// let settings = Settings::load()?;
/// let pool = connect(&settings).await?;
/// # Ok(())
/// # }
/// ```
pub async fn connect(settings: &Settings) -> Result<DbPool> {
    let database_url = settings.database_url();
    
    let options: PgConnectOptions = database_url
        .parse()
        .context("Failed to parse database URL")?;

    let pool = PgPoolOptions::new()
        .max_connections(settings.database.pool_size as u32)
        .acquire_timeout(Duration::from_secs(30))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(3600))
        .test_before_acquire(true)
        .connect_with(options)
        .await
        .context("Failed to create database connection pool")?;

    Ok(pool)
}

/// Perform a health check on a database connection pool
///
/// This is a convenience function that checks if a database pool is healthy.
///
/// # Arguments
/// * `pool` - The database connection pool to check
///
/// # Returns
/// `Result<()>` - Ok if healthy, error otherwise
///
/// # Example
/// ```no_run
/// use zercle_rust_template::infrastructure::db::connection::{connect, health_check};
/// use zercle_rust_template::config::Settings;
///
/// # async fn example() -> anyhow::Result<()> {
/// let settings = Settings::load()?;
/// let pool = connect(&settings).await?;
/// health_check(&pool).await?;
/// # Ok(())
/// # }
/// ```
pub async fn health_check(pool: &DbPool) -> Result<()> {
    sqlx::query("SELECT 1")
        .fetch_one(pool)
        .await
        .context("Database health check failed")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::settings::{Settings, ServerConfig, DatabaseConfig, JwtConfig, LoggingConfig, CorsConfig, RateLimitConfig, Argon2idConfig};

    #[test]
    fn test_database_url_construction() {
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
                user: "testuser".to_string(),
                password: "testpass".to_string(),
                name: "testdb".to_string(),
                pool_size: 10,
                ssl_mode: "disable".to_string(),
            },
            jwt: JwtConfig {
                secret: "a".repeat(32),
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
        };

        let url = settings.database_url();
        assert_eq!(
            url,
            "postgres://testuser:testpass@localhost:5432/testdb?sslmode=disable"
        );
    }

    #[test]
    fn test_parse_database_url() {
        let url = "postgres://user:pass@localhost:5432/dbname?sslmode=disable";
        let result: Result<PgConnectOptions, _> = url.parse();
        assert!(result.is_ok());
    }
}
