//! Database migration runner.
//!
//! Mirrors `cmd/migrate/main.go` (structure.md §16). Subcommands:
//!
//! * `up`            — apply all pending migrations (default).
//! * `down [N]`      — roll back the last `N` migrations (default N=1).
//! * `force VERSION` — mark the migration table at `VERSION` dirty=false.
//! * `version`       — print the current applied version.
//!
//! Migrations are embedded at compile time via `sqlx::migrate!`.

use std::process::ExitCode;

use anyhow::{Context, Result};
use sqlx::{Executor, PgPool, postgres::PgPoolOptions};
use zercle_rust_template::platform::config::Config;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(args).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("migrate: {e:#}");
            ExitCode::FAILURE
        }
    }
}

async fn run(args: Vec<String>) -> Result<()> {
    let cfg = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("failed to load config: {e:#}");
            std::process::exit(1);
        }
    };
    if let Err(e) = validator::Validate::validate(&cfg).map_err(|e| anyhow::anyhow!(e)) {
        eprintln!("invalid config: {e:#}");
        std::process::exit(1);
    }
    if let Err(e) = cfg.validate_cross() {
        eprintln!("invalid config: {e:#}");
        std::process::exit(1);
    }

    let cmd = parse_command(&args)?;

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .min_connections(0)
        .acquire_timeout(cfg.db_connect_timeout())
        .connect(&cfg.db_conn_string())
        .await
        .context("connect postgres for migration")?;

    let result = match cmd {
        Command::Up => run_up(&pool).await,
        Command::Down { count } => run_down(&pool, count).await,
        Command::Force { version } => run_force(&pool, version).await,
        Command::Version => run_version(&pool).await,
    };

    pool.close().await;
    result
}

enum Command {
    Up,
    Down { count: i64 },
    Force { version: i64 },
    Version,
}

fn parse_command(args: &[String]) -> Result<Command> {
    let cmd = args.first().map(String::as_str).unwrap_or("up");
    match cmd {
        "up" => Ok(Command::Up),
        "down" => {
            let count = if let Some(arg) = args.get(1) {
                let n: i64 = arg.parse().with_context(|| {
                    format!("invalid down count {arg:?}: must be a positive integer")
                })?;
                if n <= 0 {
                    return Err(anyhow::anyhow!(
                        "invalid down count {arg:?}: must be a positive integer"
                    ));
                }
                n
            } else {
                1
            };
            Ok(Command::Down { count })
        }
        "force" => {
            let arg = args
                .get(1)
                .ok_or_else(|| anyhow::anyhow!("force requires a version argument\n\n{}", USAGE))?;
            let version: i64 = arg
                .parse()
                .with_context(|| format!("invalid force version {arg:?}: must be an integer"))?;
            Ok(Command::Force { version })
        }
        "version" => Ok(Command::Version),
        other => Err(anyhow::anyhow!("unknown command: {other}\n\n{USAGE}")),
    }
}

const USAGE: &str = "usage: migrate [up | down [N] | force VERSION | version]";

async fn run_up(pool: &PgPool) -> Result<()> {
    // sqlx's Migrator::run returns Ok even if every migration is already
    // applied, so a no-op "up" prints "migration complete" with the current
    // version — matching Go's `migrate.ErrNoChange` behaviour.
    MIGRATOR.run(pool).await.context("apply migrations")?;
    print_version(pool).await
}

async fn run_down(pool: &PgPool, count: i64) -> Result<()> {
    // Resolve the target version from the known migration set, then issue a
    // single `undo` call. sqlx's `Migrator::undo(pool, target)` reverts every
    // applied migration with `version > target`, so computing the version
    // `count` steps below the current one lets us roll back in one transaction
    // instead of an N+1 query/undo loop.
    let current = current_version(pool).await?;
    let Some(current_v) = current else {
        println!("no migrations to undo");
        return Ok(());
    };

    let mut known_versions: Vec<i64> = MIGRATOR.iter().map(|m| m.version).collect();
    known_versions.sort_unstable();

    let target = if let Some(idx) = known_versions.iter().position(|&v| v == current_v) {
        if (idx as i64) < count {
            0
        } else {
            known_versions[idx - count as usize]
        }
    } else {
        // Current version isn't in the known set (e.g. a migration file was
        // removed); fall back to arithmetic on the version number.
        current_v.saturating_sub(count)
    };

    MIGRATOR
        .undo(pool, target)
        .await
        .with_context(|| format!("undo migration to version {target}"))?;

    print_version(pool).await
}

async fn run_force(pool: &PgPool, version: i64) -> Result<()> {
    // `force VERSION` marks the migration table at `VERSION` as
    // `success = true` (sqlx 0.8 replaced the legacy `dirty` text column with
    // a boolean `success` column; we update the equivalent semantic). We
    // execute the SQL directly because sqlx::Migrator doesn't expose a
    // public `force` helper.
    //
    // PostgreSQL forbids `ORDER BY` / `LIMIT` directly inside an `UPDATE`,
    // so the highest-applied version `<= $1` is computed in a scalar
    // subquery. The matched row already carries that version, so we only
    // flip `success = TRUE` and leave `version` untouched.
    pool.execute(
        sqlx::query(
            "UPDATE _sqlx_migrations SET success = TRUE \
             WHERE version = (SELECT version FROM _sqlx_migrations WHERE version <= $1 ORDER BY version DESC LIMIT 1)",
        )
        .bind(version),
    )
    .await
    .context("force migration version")?;
    println!("forced migration version {version} success=true");
    print_version(pool).await
}

async fn run_version(pool: &PgPool) -> Result<()> {
    print_version(pool).await
}

async fn current_version(pool: &PgPool) -> Result<Option<i64>> {
    // sqlx 0.8 stores the latest applied version in `_sqlx_migrations` as
    // the row with the maximum version and `success = TRUE`.
    let row: Option<(Option<i64>, bool)> = sqlx::query_as(
        "SELECT version, success FROM _sqlx_migrations ORDER BY version DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .context("read current migration version")?;
    Ok(row.and_then(|(v, success)| if success { v } else { None }))
}

async fn print_version(pool: &PgPool) -> Result<()> {
    let row: Option<(Option<i64>, bool)> = sqlx::query_as(
        "SELECT version, success FROM _sqlx_migrations ORDER BY version DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .context("read migration version after operation")?;
    match row {
        None => {
            println!("no migrations applied");
            Ok(())
        }
        Some((None, _)) => {
            println!("no migrations applied");
            Ok(())
        }
        Some((Some(version), success)) => {
            println!("migration complete: version {version} success={success}");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_command_defaults_to_up() {
        let cmd = parse_command(&[]).unwrap();
        assert!(matches!(cmd, Command::Up));
    }

    #[test]
    fn parse_command_down_default_count_is_one() {
        let cmd = parse_command(&["down".into()]).unwrap();
        assert!(matches!(cmd, Command::Down { count: 1 }));
    }

    #[test]
    fn parse_command_down_with_count() {
        let cmd = parse_command(&["down".into(), "3".into()]).unwrap();
        assert!(matches!(cmd, Command::Down { count: 3 }));
    }

    #[test]
    fn parse_command_down_rejects_non_positive() {
        assert!(parse_command(&["down".into(), "0".into()]).is_err());
        assert!(parse_command(&["down".into(), "-1".into()]).is_err());
        assert!(parse_command(&["down".into(), "abc".into()]).is_err());
    }

    #[test]
    fn parse_command_force_requires_version() {
        assert!(parse_command(&["force".into()]).is_err());
        let cmd = parse_command(&["force".into(), "2".into()]).unwrap();
        assert!(matches!(cmd, Command::Force { version: 2 }));
    }

    #[test]
    fn parse_command_force_rejects_non_integer() {
        assert!(parse_command(&["force".into(), "x".into()]).is_err());
    }

    #[test]
    fn parse_command_version() {
        let cmd = parse_command(&["version".into()]).unwrap();
        assert!(matches!(cmd, Command::Version));
    }

    #[test]
    fn parse_command_rejects_unknown() {
        assert!(parse_command(&["nope".into()]).is_err());
    }
}
