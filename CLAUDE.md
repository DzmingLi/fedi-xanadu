# Fedi-Xanadu Development Guidelines

## Critical: No Parallel Agents for Cross-Codebase Atomic Migrations

When performing atomic changes that span many files (e.g. database engine migration from SQLite to PostgreSQL), **never use parallel agents**. Each agent independently runs `cargo check`, sees compilation errors from files not yet updated by other agents, and "fixes" them by reverting changes — choosing the path of least resistance rather than the migration direction.

**Instead:** Process files serially, or batch all changes without intermediate compilation, then verify once at the end.

## Build Commands
- `nix develop --command cargo check` — type check
- `nix develop --command cargo test` — run tests
- `nix develop --command cargo clippy -- -D warnings` — lint
- `nix develop --command bash -c "cd frontend && npm run build"` — frontend

## Database
- **PostgreSQL** (migrated from SQLite)
- Migrations in `migrations_pg/` using `sqlx::migrate!`
- Use `$1, $2, ...` placeholders (not `?`)
- Use `NOW()` (not `datetime('now')`)
- Use `ON CONFLICT ... DO NOTHING` (not `INSERT OR IGNORE`)
- Use `ON CONFLICT ... DO UPDATE SET ...` (not `INSERT OR REPLACE`)
- Timestamps use `chrono::DateTime<Utc>` in Rust structs
- `SQLX_OFFLINE=true` for compile-time (no live DB needed)
