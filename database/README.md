# LOCAL_ROUTER Database Module

Canonical lifecycle assets for `sdkwork-local-router` per `DATABASE_FRAMEWORK_SPEC.md`.

- moduleId: `localrouter`
- serviceCode: `LOCAL_ROUTER`
- databaseRole: `authoritative-server`
- engine: `postgres`
- tablePrefix: `local_router_`

## Initialization state

This module is in **initialization state** for greenfield deployments:

1. **Baseline** - `database/ddl/baseline/postgres/0001_localrouter_baseline.sql` contains the full authoritative PostgreSQL DDL snapshot.
2. **Migrations** - `database/migrations/postgres/` is reserved for post-GA incremental schema changes only. It is intentionally empty at initialization.
3. **Drift** - run `pnpm db:drift:check` before release.

The legacy SQLite migrations under `crates/sdkwork-lr-store/migrations/sqlite/` support isolated compatibility tests and are not part of the application-root authoritative database lifecycle.

## Commands

```bash
pnpm run db:validate
pnpm run db:materialize:contract
pnpm run db:plan
pnpm run db:init
pnpm run db:migrate
pnpm run db:seed
pnpm run db:status
pnpm run db:drift:check
```
