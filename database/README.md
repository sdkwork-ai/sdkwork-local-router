# LOCAL_ROUTER Database Module

Canonical lifecycle assets for `sdkwork-local-router` per `DATABASE_FRAMEWORK_SPEC.md`.

- moduleId: `localrouter`
- serviceCode: `LOCAL_ROUTER`
- tablePrefix: `local_router_` (physical tables; manifest module prefix remains `lr_`)

## Commands

```bash
pnpm run db:materialize:contract
pnpm run db:validate
pnpm run db:bootstrap
```

## Baseline

Legacy PostgreSQL SQL is consolidated in `database/ddl/baseline/postgres/0001_localrouter_legacy_baseline.sql`.

Legacy SQLite SQL is consolidated in `database/ddl/baseline/sqlite/0001_localrouter_legacy_baseline.sql`.

## Runtime bootstrap

PostgreSQL: `Store::run_migrations()` calls `bootstrap_local_router_database_from_env()` via `sdkwork-local-router-database-host`.

SQLite: `sqlx::migrate!("./migrations/sqlite")` unchanged for tests and local runtimes.
