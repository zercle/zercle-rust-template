//! Postgres repository adapter for catalog (Go
//! `repository/postgres/repository.go` parity, port-spec §5.1f). Next wave:
//! implement `port::Repository` over `sqlx::PgPool`.
//!
//! Migration SQL is embedded from `postgres/migrations/` via
//! `catalog::di::migrations` (Go `//go:embed *.sql` parity).
