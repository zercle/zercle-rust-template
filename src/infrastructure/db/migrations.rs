use crate::config::Settings;
use anyhow::{Context, Result};
use sqlx::{Pool, Postgres, Row};
use std::path::Path;

/// Migration SQL for creating the users table
#[allow(dead_code)]
const CREATE_USERS_TABLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    full_name VARCHAR(255),
    phone VARCHAR(20),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_created_at ON users(created_at);
"#;

/// Migration SQL for creating the tasks table
#[allow(dead_code)]
const CREATE_TASKS_TABLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS tasks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    priority VARCHAR(20) NOT NULL DEFAULT 'medium',
    due_date TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_tasks_user_id ON tasks(user_id);
CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
CREATE INDEX IF NOT EXISTS idx_tasks_priority ON tasks(priority);
CREATE INDEX IF NOT EXISTS idx_tasks_due_date ON tasks(due_date);
CREATE INDEX IF NOT EXISTS idx_tasks_created_at ON tasks(created_at);
"#;

/// Migration SQL for creating updated_at trigger function
const CREATE_UPDATED_AT_TRIGGER_SQL: &str = r#"
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';
"#;

/// Migration SQL for creating updated_at trigger for users table
#[allow(dead_code)]
const CREATE_USERS_TRIGGER_SQL: &str = r#"
DROP TRIGGER IF EXISTS update_users_updated_at ON users;
CREATE TRIGGER update_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
"#;

/// Migration SQL for creating updated_at trigger for tasks table
#[allow(dead_code)]
const CREATE_TASKS_TRIGGER_SQL: &str = r#"
DROP TRIGGER IF EXISTS update_tasks_updated_at ON tasks;
CREATE TRIGGER update_tasks_updated_at
    BEFORE UPDATE ON tasks
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
"#;

/// Migration manager for database schema changes
pub struct Migrations;

impl Migrations {
    /// Run all pending migrations
    ///
    /// This method executes all necessary migrations to bring the database
    /// schema up to the latest version.
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    ///
    /// # Returns
    /// `Result<()>` - Ok if migrations succeeded, error otherwise
    ///
    /// # Example
    /// ```no_run
    /// use zercle_rust_template::infrastructure::db::migrations::Migrations;
    /// use sqlx::PgPool;
    ///
    /// # async fn example(pool: &PgPool) -> anyhow::Result<()> {
    /// Migrations::run(pool).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn run(pool: &Pool<Postgres>) -> Result<()> {
        // Run migrations in order
        Self::create_users_table(pool).await?;
        Self::create_tasks_table(pool).await?;
        Self::create_updated_at_trigger(pool).await?;
        Self::create_users_trigger(pool).await?;
        Self::create_tasks_trigger(pool).await?;

        Ok(())
    }

    /// Create the users table
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    ///
    /// # Returns
    /// `Result<()>` - Ok if successful, error otherwise
    pub async fn create_users_table(pool: &Pool<Postgres>) -> Result<()> {
        // Create users table
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS users (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                email VARCHAR(255) UNIQUE NOT NULL,
                password_hash VARCHAR(255) NOT NULL,
                full_name VARCHAR(255),
                phone VARCHAR(20),
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )",
        )
        .execute(pool)
        .await
        .context("Failed to create users table")?;

        // Create users indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_email ON users(email)")
            .execute(pool)
            .await
            .context("Failed to create users email index")?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_created_at ON users(created_at)")
            .execute(pool)
            .await
            .context("Failed to create users created_at index")?;

        Ok(())
    }

    /// Create the tasks table
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    ///
    /// # Returns
    /// `Result<()>` - Ok if successful, error otherwise
    pub async fn create_tasks_table(pool: &Pool<Postgres>) -> Result<()> {
        // Create tasks table
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS tasks (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                title VARCHAR(255) NOT NULL,
                description TEXT,
                status VARCHAR(20) NOT NULL DEFAULT 'pending',
                priority VARCHAR(20) NOT NULL DEFAULT 'medium',
                due_date TIMESTAMPTZ,
                completed_at TIMESTAMPTZ,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )",
        )
        .execute(pool)
        .await
        .context("Failed to create tasks table")?;

        // Create tasks indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tasks_user_id ON tasks(user_id)")
            .execute(pool)
            .await
            .context("Failed to create tasks user_id index")?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status)")
            .execute(pool)
            .await
            .context("Failed to create tasks status index")?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tasks_priority ON tasks(priority)")
            .execute(pool)
            .await
            .context("Failed to create tasks priority index")?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tasks_due_date ON tasks(due_date)")
            .execute(pool)
            .await
            .context("Failed to create tasks due_date index")?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tasks_created_at ON tasks(created_at)")
            .execute(pool)
            .await
            .context("Failed to create tasks created_at index")?;

        Ok(())
    }

    /// Create the updated_at trigger function
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    ///
    /// # Returns
    /// `Result<()>` - Ok if successful, error otherwise
    pub async fn create_updated_at_trigger(pool: &Pool<Postgres>) -> Result<()> {
        sqlx::query(CREATE_UPDATED_AT_TRIGGER_SQL)
            .execute(pool)
            .await
            .context("Failed to create updated_at trigger function")?;

        Ok(())
    }

    /// Create the updated_at trigger for users table
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    ///
    /// # Returns
    /// `Result<()>` - Ok if successful, error otherwise
    pub async fn create_users_trigger(pool: &Pool<Postgres>) -> Result<()> {
        sqlx::query("DROP TRIGGER IF EXISTS update_users_updated_at ON users")
            .execute(pool)
            .await
            .context("Failed to drop users updated_at trigger")?;

        sqlx::query(
            "CREATE TRIGGER update_users_updated_at
            BEFORE UPDATE ON users
            FOR EACH ROW
            EXECUTE FUNCTION update_updated_at_column()",
        )
        .execute(pool)
        .await
        .context("Failed to create users updated_at trigger")?;

        Ok(())
    }

    /// Create the updated_at trigger for tasks table
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    ///
    /// # Returns
    /// `Result<()>` - Ok if successful, error otherwise
    pub async fn create_tasks_trigger(pool: &Pool<Postgres>) -> Result<()> {
        sqlx::query("DROP TRIGGER IF EXISTS update_tasks_updated_at ON tasks")
            .execute(pool)
            .await
            .context("Failed to drop tasks updated_at trigger")?;

        sqlx::query(
            "CREATE TRIGGER update_tasks_updated_at
            BEFORE UPDATE ON tasks
            FOR EACH ROW
            EXECUTE FUNCTION update_updated_at_column()",
        )
        .execute(pool)
        .await
        .context("Failed to create tasks updated_at trigger")?;

        Ok(())
    }

    /// Run migrations from SQL files
    ///
    /// This method reads migration SQL files from the specified directory
    /// and executes them in order.
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `migrations_dir` - Path to the directory containing migration files
    ///
    /// # Returns
    /// `Result<()>` - Ok if migrations succeeded, error otherwise
    ///
    /// # Example
    /// ```no_run
    /// use zercle_rust_template::infrastructure::db::migrations::Migrations;
    /// use sqlx::PgPool;
    ///
    /// # async fn example(pool: &PgPool) -> anyhow::Result<()> {
    /// Migrations::run_from_files(pool, "migrations").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn run_from_files<P: AsRef<Path>>(
        pool: &Pool<Postgres>,
        migrations_dir: P,
    ) -> Result<()> {
        let dir = migrations_dir.as_ref();

        // Read all .sql files from the migrations directory
        let mut migration_files: Vec<_> = std::fs::read_dir(dir)
            .context("Failed to read migrations directory")?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .map(|ext| ext == "sql")
                    .unwrap_or(false)
            })
            .collect();

        // Sort migration files by name to ensure correct order
        migration_files.sort_by_key(|entry| entry.path());

        // Execute each migration file
        for entry in migration_files {
            let path = entry.path();
            let sql_content = std::fs::read_to_string(&path)
                .with_context(|| format!("Failed to read migration file: {path:?}"))?;

            sqlx::query(&sql_content)
                .execute(pool)
                .await
                .with_context(|| format!("Failed to execute migration: {path:?}"))?;
        }

        Ok(())
    }

    /// Drop all tables (use with caution!)
    ///
    /// This method drops all tables created by the migration system.
    /// This is primarily useful for testing purposes.
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    ///
    /// # Returns
    /// `Result<()>` - Ok if successful, error otherwise
    ///
    /// # Warning
    /// This will delete all data in the tables!
    pub async fn drop_all(pool: &Pool<Postgres>) -> Result<()> {
        sqlx::query("DROP TABLE IF EXISTS tasks CASCADE")
            .execute(pool)
            .await
            .context("Failed to drop tasks table")?;

        sqlx::query("DROP TABLE IF EXISTS users CASCADE")
            .execute(pool)
            .await
            .context("Failed to drop users table")?;

        sqlx::query("DROP FUNCTION IF EXISTS update_updated_at_column")
            .execute(pool)
            .await
            .context("Failed to drop update_updated_at_column function")?;

        Ok(())
    }

    /// Check if migrations have been run
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    ///
    /// # Returns
    /// `Result<bool>` - true if migrations have been run, false otherwise
    pub async fn check_migrations(pool: &Pool<Postgres>) -> Result<bool> {
        let result = sqlx::query(
            "SELECT EXISTS (
                SELECT FROM information_schema.tables
                WHERE table_schema = 'public'
                AND table_name = 'users'
            )",
        )
        .fetch_one(pool)
        .await
        .context("Failed to check migrations")?;

        let exists: bool = result.try_get("exists")?;
        Ok(exists)
    }
}

/// Run migrations from application settings
///
/// This is a convenience function that creates a connection pool
/// from settings and runs migrations.
///
/// # Arguments
/// * `settings` - Application settings
///
/// # Returns
/// `Result<()>` - Ok if migrations succeeded, error otherwise
///
/// # Example
/// ```no_run
/// use zercle_rust_template::infrastructure::db::migrations::migrate_from_settings;
/// use zercle_rust_template::config::Settings;
///
/// # async fn example() -> anyhow::Result<()> {
/// let settings = Settings::load()?;
/// migrate_from_settings(&settings).await?;
/// # Ok(())
/// # }
/// ```
pub async fn migrate_from_settings(settings: &Settings) -> Result<()> {
    use crate::infrastructure::db::connection::connect;

    let pool = connect(settings).await?;
    Migrations::run(&pool).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_sql_syntax() {
        // Verify that the SQL strings are syntactically valid
        // by checking they contain expected keywords
        assert!(CREATE_USERS_TABLE_SQL.contains("CREATE TABLE"));
        assert!(CREATE_USERS_TABLE_SQL.contains("users"));
        assert!(CREATE_TASKS_TABLE_SQL.contains("CREATE TABLE"));
        assert!(CREATE_TASKS_TABLE_SQL.contains("tasks"));
        assert!(CREATE_UPDATED_AT_TRIGGER_SQL.contains("CREATE OR REPLACE FUNCTION"));
        assert!(CREATE_USERS_TRIGGER_SQL.contains("CREATE TRIGGER"));
        assert!(CREATE_TASKS_TRIGGER_SQL.contains("CREATE TRIGGER"));
    }

    #[test]
    fn test_migration_sql_indexes() {
        // Verify that indexes are created
        assert!(CREATE_USERS_TABLE_SQL.contains("CREATE INDEX"));
        assert!(CREATE_USERS_TABLE_SQL.contains("idx_users_email"));
        assert!(CREATE_TASKS_TABLE_SQL.contains("CREATE INDEX"));
        assert!(CREATE_TASKS_TABLE_SQL.contains("idx_tasks_user_id"));
    }

    #[test]
    fn test_migration_sql_foreign_key() {
        // Verify that foreign key constraint is created
        assert!(CREATE_TASKS_TABLE_SQL.contains("REFERENCES users(id)"));
        assert!(CREATE_TASKS_TABLE_SQL.contains("ON DELETE CASCADE"));
    }
}
