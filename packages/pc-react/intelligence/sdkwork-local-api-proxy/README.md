# @sdkwork/local-api-proxy

## Purpose

`@sdkwork/local-api-proxy` is the canonical local gateway capability for SDKWORK PC React applications.
It owns one authority model, one runtime-plan compiler, one storage standard, one host bridge surface, and one reusable UI surface set.

## Standard Model

- Package: `@sdkwork/local-api-proxy`
- Architecture: `pc-react`
- Domain: `intelligence`
 - Capability: `local-api-proxy`
 - Status: `ready`
- Authority model: `LocalApiProxyConfig`
- Runtime compilation artifact: `LocalApiProxyRuntimePlan`
- Table prefix: `lap_`

This package does not split config authority across UI, runtime snapshots, or consumer-owned files.
Consumer projections are outputs of the runtime plan, not peer configuration systems.

## Storage Topology

- Desktop-local mode uses one SQLite file.
- Server-managed mode uses one PostgreSQL schema.
- Both modes share the same domain model, repository ports, and table set.

The canonical table set is:

- `lap_schema_migrations`
- `lap_config`
- `lap_routes`
- `lap_route_capabilities`
- `lap_route_models`
- `lap_route_exposures`
- `lap_runtime_settings`
- `lap_probe_records`
- `lap_credentials`
- `lap_request_logs`
- `lap_message_logs`
- `lap_capture_settings`
- `lap_runtime_events`

## Package Boundary

This package owns:

- authority config and normalization
- runtime-plan compilation
- capability-first gateway operation catalogs
- SQLite and PostgreSQL schema builders
- host bridge contracts and Tauri adapter shell
- package-owned services for config, runtime, and observability
- reusable PC React components and page shell
- native Rust runtime scaffold under `native/tauri-rust`

This package does not own:

- application shell routing
- usage or quota policy
- application-specific secrets backends
- application-owned settings wrappers

## Public API

Current public surfaces include:

- package metadata via `localApiProxyPackageMeta`
- authority model and config helpers
- runtime-plan compiler and route selection helpers
- provider-route compatibility helpers for application-side route editors and migrations, including legacy provider id/model-ref normalization
- provider runtime config contracts and normalizers
- provider request-override draft format/parse helpers for application-side editors
- provider routing catalog contracts for presets, persisted routes, and configured-provider summaries
- projection helpers for consumer-facing route selection, model catalog normalization, and runtime endpoint resolution
- projected provider-model catalog helpers for consumer kernels that need normalized roles and streaming metadata
- shared JSON5 parse/stringify helpers used by package-owned draft editors and config tooling
- gateway operation catalog and route groups
- repository ports and dialect-specific schema builders
- Tauri bridge contracts and host service helpers
- config, runtime, and observability services
- logs workspace service helpers for request/message capture consoles
- `LocalApiProxyRuntimeSummary`
- `LocalApiProxyRouteList`
- `LocalApiProxyPage`

## Native Boundary

The embedded Rust implementation lives under `native/tauri-rust`.
It is an internal owned boundary of the package, not a separate workspace package or architecture tree.
That keeps the operational surface small while preserving a clean split between TypeScript package APIs and native runtime internals.

The native boundary is also the canonical source for desktop runtime constants such as:

- default port `21281`
- provider id `sdkwork-local-proxy`
- token env var `SDKWORK_LOCAL_PROXY_TOKEN`
- Sdkwork-facing provider API identifiers

## Integration Direction

Applications such as `sdkwork-studio` should consume this package as an assembly dependency.
They should not re-implement config models, route normalization, schema shape, or host command catalogs in application code.
