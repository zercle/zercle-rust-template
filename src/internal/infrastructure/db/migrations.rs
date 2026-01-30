use sqlx::PgPool;
use std::path::Path;

pub struct Migrations;

impl Migrations {
    pub async fn run(pool: &PgPool) -> anyhow::Result<()> {
        let migrations_path = std::env::var("MIGRATIONS_PATH").unwrap_or_else(|_| "migrations".to_string());

        sqlx::migrate::Migrator::new(Path::new(&migrations_path))
            .await?
            .run(pool)
            .await?;

        Ok(())
    }
}
