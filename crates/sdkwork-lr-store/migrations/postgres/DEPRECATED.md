# Deprecated legacy migrations

PostgreSQL schema is consolidated in `database/ddl/baseline/postgres/0001_localrouter_legacy_baseline.sql`.

Production PostgreSQL bootstrap MUST use `sdkwork-local-router-database-host` via `bootstrap_local_router_database()`.

SQLite tests and local runtimes continue to use `sqlx::migrate!("./migrations/sqlite")`.
