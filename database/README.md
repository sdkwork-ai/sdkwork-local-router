# LOCAL_ROUTER Database Module

Canonical lifecycle assets for `sdkwork-local-router` per `DATABASE_FRAMEWORK_SPEC.md`.

- moduleId: `localrouter`
- serviceCode: `LOCAL_ROUTER`
- tablePrefix: `lr_`

## Commands

```bash
pnpm run db:validate
pnpm run db:plan
pnpm run db:init
pnpm run db:migrate
pnpm run db:seed
pnpm run db:status
pnpm run db:drift:check
```

## Migration status

Legacy SQL was consolidated into `ddl/baseline/postgres/0001_*_legacy_baseline.sql` for bootstrap review.
Author contract-first tables in `contract/schema.yaml`, then split baseline into versioned `migrations/` pairs.

Imported legacy sources:
- `crates/sdkwork-lr-store/migrations/postgres/20260522000001_init.sql`
- `crates/sdkwork-lr-store/migrations/postgres/20260523000001_add_retry_config.sql`
- `crates/sdkwork-lr-store/migrations/postgres/20260604000001_local_router_user_isolation.sql`
- `crates/sdkwork-lr-store/migrations/postgres/20260604000002_add_invocation_metadata.sql`
- `crates/sdkwork-lr-store/migrations/postgres/20260604000003_add_upstream_auth_scheme.sql`
- `crates/sdkwork-lr-store/migrations/postgres/20260604000004_add_model_route_mappings_table.sql`
- `crates/sdkwork-lr-store/migrations/postgres/20260605000001_use_runtime_snowflake_ids.sql`
- `crates/sdkwork-lr-store/migrations/sqlite/20260522000001_init.sql`
- `crates/sdkwork-lr-store/migrations/sqlite/20260523000001_add_retry_config.sql`
- `crates/sdkwork-lr-store/migrations/sqlite/20260604000001_local_router_user_isolation.sql`
- `crates/sdkwork-lr-store/migrations/sqlite/20260604000002_add_invocation_metadata.sql`
- `crates/sdkwork-lr-store/migrations/sqlite/20260604000003_add_upstream_auth_scheme.sql`
- `crates/sdkwork-lr-store/migrations/sqlite/20260604000004_add_model_route_mappings_table.sql`
- `crates/sdkwork-lr-store/migrations/sqlite/20260605000001_use_runtime_snowflake_ids.sql`

Runtime services MUST create pools through `sdkwork-database-sqlx` and register `DefaultDatabaseModule` at bootstrap.
