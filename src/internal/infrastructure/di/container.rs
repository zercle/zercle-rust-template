use std::sync::Arc;

use crate::internal::{
    domain::error::DomainError,
    domain::task::service::TaskServiceImpl,
    domain::task::traits::TaskService,
    domain::user::service::UserServiceImpl,
    domain::user::traits::{JwtGenerator, PasswordHasher, RefreshTokenRepository, TaskRepository, UserRepository, UserService},
    infrastructure::{
        config::Config,
        db::connection::DbPool,
        http::middleware::rate_limit::RateLimiter,
        logger::init_logging,
        repository::{RefreshTokenRepositoryImpl, TaskRepositoryImpl, UserRepositoryImpl},
        security::{jwt::JwtGeneratorImpl, password::Argon2PasswordHasher},
    },
};

/// Application dependency injection container
/// Holds all application dependencies and provides them to handlers
#[derive(Clone)]
pub struct Container {
    /// Application configuration
    pub config: Arc<Config>,

    /// Database connection pool
    pub db_pool: Arc<DbPool>,

    /// Password hashing service
    pub password_hasher: Arc<dyn PasswordHasher>,

    /// JWT token generator
    pub jwt_generator: Arc<dyn JwtGenerator>,

    /// User repository
    pub user_repository: Arc<dyn UserRepository>,

    /// Refresh token repository
    pub refresh_token_repository: Arc<dyn RefreshTokenRepository>,

    /// Task repository
    pub task_repository: Arc<dyn TaskRepository>,

    /// User service
    pub user_service: Arc<dyn UserService>,

    /// Task service
    pub task_service: Arc<dyn TaskService>,

    /// Rate limiter
    pub rate_limiter: Arc<RateLimiter>,
}

impl Container {
    /// Create a new container with all dependencies
    pub async fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        // 1. Initialize logging
        init_logging(&config.logging.level, &config.logging.format)?;

        // 2. Create database pool
        let db_pool = create_db_pool(&config).await?;

        // 3. Run migrations
        run_migrations(&db_pool).await?;

        // 4. Create security components
        let password_hasher: Arc<dyn PasswordHasher> = Arc::new(Argon2PasswordHasher::new());
        
        // Convert seconds from config to minutes/days for JWT generator
        let access_token_ttl_minutes = (config.jwt.access_token_expiry / 60) as i64;
        let refresh_token_ttl_days = (config.jwt.refresh_token_expiry / 86400) as i64;
        
        let jwt_generator: Arc<dyn JwtGenerator> = Arc::new(JwtGeneratorImpl::with_durations(
            config.jwt.secret.as_bytes(),
            access_token_ttl_minutes,
            refresh_token_ttl_days,
        ));

        // 5. Create repositories
        let user_repository: Arc<dyn UserRepository> = Arc::new(UserRepositoryImpl::new(db_pool.clone()));
        let refresh_token_repository: Arc<dyn RefreshTokenRepository> = Arc::new(RefreshTokenRepositoryImpl::new(db_pool.clone()));
        let task_repository: Arc<dyn TaskRepository> = Arc::new(TaskRepositoryImpl::new(db_pool.clone()));

        // 6. Create services
        let user_service: Arc<dyn UserService> = Arc::new(UserServiceImpl::new(
            user_repository.clone(),
            refresh_token_repository.clone(),
            task_repository.clone(),
            password_hasher.clone(),
            jwt_generator.clone(),
        ));

        let task_service: Arc<dyn TaskService> = Arc::new(TaskServiceImpl::new(task_repository.clone()));

        // 7. Create rate limiter (default to 60 requests per minute if not configured)
        let rate_limit_config = std::env::var("RATE_LIMIT_REQUESTS_PER_MINUTE")
            .unwrap_or_else(|_| "60".to_string())
            .parse()
            .unwrap_or(60);
        let rate_limiter = Arc::new(RateLimiter::new(rate_limit_config, 60));

        Ok(Self {
            config: Arc::new(config),
            db_pool,
            password_hasher,
            jwt_generator,
            user_repository,
            refresh_token_repository,
            task_repository,
            user_service,
            task_service,
            rate_limiter,
        })
    }
}

/// Create database pool from configuration
async fn create_db_pool(config: &Config) -> Result<Arc<DbPool>, DomainError> {
    use sqlx::postgres::PgPoolOptions;
    
    let pool = PgPoolOptions::new()
        .max_connections(config.database.max_open_conns)
        .min_connections(config.database.max_idle_conns)
        .connect(&config.connection_string())
        .await
        .map_err(|e| DomainError::Database(e.to_string()))?;

    Ok(Arc::new(pool))
}

/// Run database migrations
async fn run_migrations(pool: &DbPool) -> Result<(), DomainError> {
    use std::path::Path;
    
    let migrations_path = std::env::var("MIGRATIONS_PATH").unwrap_or_else(|_| "migrations".to_string());
    
    sqlx::migrate::Migrator::new(Path::new(&migrations_path))
        .await
        .map_err(|e| DomainError::Database(e.to_string()))?
        .run(pool)
        .await
        .map_err(|e| DomainError::Database(e.to_string()))?;

    Ok(())
}
