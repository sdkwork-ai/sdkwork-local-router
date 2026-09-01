# Repository Guidelines

<!-- SDKWORK-AGENTS-GENERATED: v2 -->

## SDKWORK Soul

Read `../sdkwork-specs/SOUL.md` before executing tasks in this root. Apply specs before memory,
dictionary before context, exact sources before inference, and evidence before completion.

## SDKWORK Standards

The canonical standards entrypoint is `../sdkwork-specs/README.md`. Agent entrypoints follow
`../sdkwork-specs/AGENTS_SPEC.md`; repository layout follows
`../sdkwork-specs/SDKWORK_WORKSPACE_SPEC.md`. Do not copy global normative bodies into this file.

## Application Identity

- Application id: `sdkwork-local-router`
- Domain and capability: `intelligence/local-router`
- Runtime family: Rust/Axum application with cloud and standalone profiles
- API assembly: `crates/sdkwork-api-local-router-assembly`
- Standalone host: `crates/sdkwork-api-local-router-standalone-gateway`
- Application declaration: `sdkwork.app.config.json`
- Source configuration entrypoint: `etc/sdkwork.deployment.config.json`
- Runtime TOML example: `config.example.toml`
- Component contract: `specs/component.spec.json`

The provider-compatible open-api ingress namespace is locked to `/local-router`: OpenAI-compatible
traffic uses `/local-router/v1`, Anthropic-compatible traffic uses `/local-router/anthropic`, and
Google-compatible traffic uses `/local-router/google`. Standalone and cloud deployments expose the
same paths.

## Local Dictionary Structure

- `AGENTS.md`: repository execution entrypoint.
- `CLAUDE.md`, `GEMINI.md`, `CODEX.md`: compatibility shims that point back here.
- `sdkwork.app.config.json`: application and release identity.
- `etc/`: source-controlled deployment profiles governed by `SOURCE_CONFIG_SPEC.md`.
- `specs/`: application-wide machine contracts.
- `sdks/_route-manifests/`: materialized app-api, backend-api, and open-api route contracts.
- `crates/`: Rust config, core, proxy, route, assembly, database-host, and standalone-host crates.
- `database/`: Local Router-owned database baseline and lifecycle contracts.
- `apps/`: desktop and web application surfaces.
- `.sdkwork/`: source-controlled AI metadata only; runtime state and secrets stay ignored.

Documentation Canon:

- [docs/README.md](docs/README.md)
- [docs/product/prd/PRD.md](docs/product/prd/PRD.md)
- [docs/architecture/tech/TECH_ARCHITECTURE.md](docs/architecture/tech/TECH_ARCHITECTURE.md)

## Spec Resolution Order

1. Read the nearest `AGENTS.md`.
2. Read `sdkwork.app.config.json` when application identity, runtime, SDK wiring, release,
   packaging, or app-owned capabilities are in scope.
3. Read the nearest module `specs/` and root `specs/` only when their contracts are touched.
4. Read `.sdkwork/README.md` and only relevant local skill/plugin metadata when applicable.
5. Resolve the task row in `../sdkwork-specs/README.md`.
6. Read only the global specs selected by that task row or the touched component contract.
7. Inspect implementation files.

## Required Specs By Task Type

- Agent/workflow changes: `SOUL.md`, `AGENTS_SPEC.md`, `SDKWORK_WORKSPACE_SPEC.md`,
  `DOCUMENTATION_SPEC.md`, and `TEST_SPEC.md`.
- Any authored code: `CODE_STYLE_SPEC.md`, `NAMING_SPEC.md`, and only the touched language spec.
- Rust/Cargo: `RUST_CODE_SPEC.md` and `TEST_SPEC.md`.
- TypeScript/Node: `TYPESCRIPT_CODE_SPEC.md`, `PNPM_SCRIPT_SPEC.md`, and `TEST_SPEC.md`.
- API/SDK: `API_SPEC.md`, `SDK_SPEC.md`, `SDK_WORKSPACE_GENERATION_SPEC.md`,
  `APP_SDK_INTEGRATION_SPEC.md`, `WEB_FRAMEWORK_SPEC.md`, and `TEST_SPEC.md`.
- List/search: add `PAGINATION_SPEC.md`.
- Component/gateway composition: `COMPONENT_SPEC.md`, `COMPOSABLE_ARCHITECTURE_SPEC.md`,
  `APPLICATION_GATEWAY_SPEC.md`, `APP_RUNTIME_TOPOLOGY_SPEC.md`, `APP_COMPOSITION_SPEC.md`, and
  `TEST_SPEC.md`.
- Database: `DATABASE_SPEC.md`, `DATABASE_FRAMEWORK_SPEC.md`, and `TEST_SPEC.md`.
- Source config/runtime: `SOURCE_CONFIG_SPEC.md`, `CONFIG_SPEC.md`, `ENVIRONMENT_SPEC.md`,
  `RUNTIME_DIRECTORY_SPEC.md`, `DEPLOYMENT_SPEC.md`, and `TEST_SPEC.md`.
- Security/auth: `IAM_SPEC.md`, `IAM_LOGIN_INTEGRATION_SPEC.md`, `SECURITY_SPEC.md`, and
  `PRIVACY_SPEC.md`.
- Packaging/workflows: `PNPM_SCRIPT_SPEC.md`, `GITHUB_WORKFLOW_SPEC.md`, `RELEASE_SPEC.md`, and
  `SUPPLY_CHAIN_SECURITY_SPEC.md`.

Language-specific specs are on-demand; do not load unrelated language or framework specs.

## Code Style Rules

- Keep `src/lib.rs` limited to module declarations, re-exports, light docs, and wiring.
- Keep provider credentials separate from client API keys and never log either secret.
- Route crates own business routes; assembly crates compose them; process hosts own listeners,
  readiness, metrics, and shutdown.
- Keep standalone and cloud paths, methods, auth, and response behavior identical.
- Fix source contracts and regenerate approved SDK output; never hand-edit
  `generated/server-openapi`.
- Use `sdkwork-utils` for approved shared utilities rather than duplicating generic behavior.
- Build-critical source handling follows `CODE_STYLE_SPEC.md` section 7; `pnpm clean` must not
  remove tracked build sources.

## Agent Execution Rules

Use dynamic progressive loading before implementation files: nearest dictionary, relevant local
contract, task-specific global specs, then affected source. Do not replace generated SDK integration
with raw HTTP or report completion without recorded verification evidence. Database schema or
migration changes require explicit user approval.

## Task-Specific Standards

- App SDK consumer work: run
  `node ../sdkwork-specs/tools/check-app-sdk-consumer-imports.mjs --workspace .`.
- HTTP API contracts: run `check-api-operation-patterns.mjs` and
  `check-api-response-envelope.mjs` from `../sdkwork-specs/tools/`.
- List/search changes: run `node ../sdkwork-specs/tools/check-pagination.mjs --workspace .`.
- Source configuration changes: run
  `node ../sdkwork-specs/tools/check-source-config-standard.mjs --root .`.
- Agent entrypoint changes: run
  `node ../sdkwork-specs/tools/check-agent-workflow-standard.mjs --root .`.

## Int64 Wire Contract (API_SPEC §13.6)

- OpenAPI `int64` fields and parameters `MUST` be `type: string`, `format: int64`,
  a decimal `pattern` such as `^-?[0-9]+$`, and `x-sdkwork-int64-string: true`.
  `type: integer, format: int64` is a contract violation: generated TypeScript
  SDKs then emit `number`, and browsers silently round ids past
  `Number.MAX_SAFE_INTEGER` (2^53), replaying wrong ids into lookups.
- Rust response DTOs `MUST` serialize `i64` wire fields with
  `#[serde(with = "sdkwork_utils_rust::serde_int64")]` (or `::option`); request
  boundaries parse inbound strings with the same helper.
- Generated TypeScript SDKs keep `int64` as `string`; frontend code `MUST NOT`
  convert ids/snowflake ids/sequence ids to `number` for storage, comparison,
  or submission.
- Verification: `node <sdkwork-specs>/tools/check-api-operation-patterns.mjs --workspace .`

## Build, Test, And Verification

Run the narrowest relevant check first, then the root aggregate:

```powershell
cargo fmt -- --check
cargo test --workspace
cargo clippy --workspace --tests -- -D warnings
pnpm check
pnpm verify
```

Do not run `cargo fmt --all`; optional workspace paths may cross application authority boundaries.

## Human Review Rules

Human review is required for breaking public API/SDK changes, security exceptions, database schema
or migration changes, destructive filesystem work, generated SDK ownership changes, and release or
deployment governance changes.

<!-- SDKWORK-NAMING-STANDARD: v1 -->
## Rust Naming And Dependency Declaration

Authority: `../sdkwork-specs/NAMING_SPEC.md` section 3.1 and section 3.2.

Two identifier planes exist in every Rust crate and they MUST NOT be mixed: the package plane
(Cargo, filesystem, lock file) uses kebab-case, and the crate plane (lib target, modules, source
imports) uses snake_case.

- `[package].name`, the crate directory, `[features]` keys, and `[[bin]].name` use kebab-case.
- `[lib].name`, module files, module directories, and Rust imports use snake_case.
- A crate whose `[package].name` contains a hyphen SHOULD declare `[lib].name` explicitly
  (default: package name with every `-` replaced by `_`). A shorter lib name is allowed only
  when declared explicitly and used consistently by every consumer.
- Cargo dependency keys, `[workspace.dependencies]` keys, and `Cargo.lock` entries use the
  dependency package name. Use `package = "..."` when an alias is required.
- Every external crate referenced by `src/` MUST be declared in that crate's `[dependencies]`.
  Test-only crates belong in `[dev-dependencies]`; `build.rs` crates belong in
  `[build-dependencies]`.
- Never delete a dependency line, and never demote one from `[dependencies]` to
  `[dev-dependencies]`, while `src/` still imports it. Verify manifest cleanups with the
  command below before committing them.
- Regenerate and commit `Cargo.lock` in the same change as any dependency table edit.

Verification:

```bash
node ../sdkwork-specs/tools/check-rust-crate-naming-standard.mjs --root .
```
<!-- /SDKWORK-NAMING-STANDARD: v1 -->

<!-- SDKWORK-RUST-CODE-STANDARD: v1 -->
## Rust Code Standard

Authority: `../sdkwork-specs/RUST_CODE_SPEC.md` (v2, industry-best baseline); package/crate
naming and dependency declaration are normative in `../sdkwork-specs/NAMING_SPEC.md` section 3.1
and 3.2.

- Crates are responsibility-shaped: service, repository-sqlx, routes, service-host, native-host,
  worker, assembly, gateway. No generic `core`/`common`/`backend`/`runtime` suffixes.
- Errors are typed enums (`thiserror`) implementing `std::error::Error` with a `source` chain.
  `anyhow` only at binary/CLI/test boundaries, never in lib `[dependencies]`.
- No `unsafe` without a `// SAFETY:` comment; crates default to `unsafe_code = "forbid"`.
  No `unwrap`/`expect`/`panic!`/`todo!`/`dbg!` in library code reachable from public API.
- No lock guard held across `.await`; every external await has a timeout; spawned tasks are
  awaited/detached with a documented owner; retries are bounded, jittered, and idempotent.
- Public API is minimal, documented, `#[must_use]` where applicable, and semver-clean. Leaking
  framework types (`sqlx::Row`, axum extractors) through public signatures is forbidden.
- Workspace root declares `[workspace.package]` (edition, rust-version) and `[workspace.lints]`
  (RUST_CODE_SPEC.md section 13 baseline); every member inherits both with
  `edition.workspace = true` and `[lints] workspace = true`.

Verification:

```bash
node ../sdkwork-specs/tools/check-rust-crate-naming-standard.mjs --root .
node ../sdkwork-specs/tools/check-rust-manifest-standard.mjs --root .
# when service/repository/route/gateway dependencies change:
node ../sdkwork-specs/tools/check-rust-backend-composition.mjs --root .
```
<!-- /SDKWORK-RUST-CODE-STANDARD: v1 -->

<!-- SDKWORK-TYPESCRIPT-CODE-STANDARD: v1 -->
## TypeScript Code Standard

Authority: `../sdkwork-specs/TYPESCRIPT_CODE_SPEC.md` (v2, industry-best baseline).

- `tsconfig` runs `strict: true` and the strict family; public APIs are typed and `any`-free.
  `import type` is required for type-only imports (`verbatimModuleSyntax`).
- Errors are typed at package/service boundaries; no empty catches, no swallowed promise
  rejections, no bare `throw new Error('...')` for business failures.
- Async: every promise is settled; external awaits have timeouts; `AbortSignal` accepted for
  cancellable work; bounded concurrency; no unbounded `Promise.all`.
- Public API is minimal, JSDoc-documented, `@deprecated` where applicable, and semver-clean.
- Discriminated unions model closed variant sets; no `as`/`@ts-ignore` bypasses without a guard.
- Node/build runners verify build-critical sources and self-heal from git (CODE_STYLE_SPEC §7);
  `pnpm clean` never deletes git-tracked build-critical files.

Verification:

```bash
pnpm typecheck && pnpm test && pnpm lint
node ../sdkwork-specs/tools/check-application-layering.mjs --root .
```
<!-- /SDKWORK-TYPESCRIPT-CODE-STANDARD: v1 -->

<!-- SDKWORK-FRONTEND-CODE-STANDARD: v1 -->
## Frontend Code Standard

Authority: `../sdkwork-specs/FRONTEND_CODE_SPEC.md` (v2); language rules follow
`../sdkwork-specs/TYPESCRIPT_CODE_SPEC.md` (React/TS) or `../sdkwork-specs/DART_CODE_SPEC.md` (Flutter).

- UI -> service -> injected SDK flow is preserved; components never construct SDK clients or
  assemble raw HTTP/auth headers.
- React: hooks rules clean (`react-hooks`), `useEffect` with full deps and cleanup, stable
  list keys, error boundaries at route/page level, derived state during render (not in effects).
- State: server state behind services/query layer; client state local or minimal typed store;
  no duplication of server state in client stores.
- Accessibility: accessible names, keyboard behavior, visible focus, color is never the only
  signal; error states announced.
- i18n for all user-facing copy in reusable/user-facing packages (I18N_SPEC §6.1).
- PC/H5 `outDir` uses `dist/{standalone,cloud}/{dev,test,staging,prod}`.

Verification:

```bash
pnpm typecheck && pnpm test && pnpm lint
node ../sdkwork-specs/tools/check-application-layering.mjs --root .
node ../sdkwork-specs/tools/check-browser-dist-layout.mjs --root .   # PC/H5 apps
```
<!-- /SDKWORK-FRONTEND-CODE-STANDARD: v1 -->

<!-- SDKWORK-PNPM-WORKSPACE-STANDARD: v1 -->
## pnpm Workspace Dependency And Package Import

Authority: `../sdkwork-specs/PNPM_WORKSPACE_DEPENDENCY_SPEC.md` (companion to
`../sdkwork-specs/DEPENDENCY_MANAGEMENT_SPEC.md`).

Sibling SDKWork repositories are consumed through a dual-track model that MUST stay consistent:

- **Local development** (`pnpm dev`, `pnpm build`): pnpm workspace protocol. Each sibling
  package is declared ONCE in this repository root `pnpm-workspace.yaml` `packages:` as a
  `../sdkwork-*` relative path, and consumed with `workspace:*` in `package.json`. Never use
  `file:`/`link:`/git-URL specifiers for SDKWork sibling packages in any environment.
- **CI / release packaging**: git-repository dependency checkout. Every sibling referenced by the
  local workspace MUST have a matching `dependencies[]` entry in `sdkwork.workflow.json` so CI
  clones the sibling into the same `../sdkwork-*` relative layout (`GITHUB_WORKFLOW_SPEC.md`).
  `package.json` is never rewritten for CI.

Import rules for sibling SDKWork packages:

- Import by package name only: `import { X } from "@sdkwork/package-name"`. The specifier MUST
  equal the target package's `package.json` `name` exactly - no shortening, renaming, or alias.
- Forbidden: relative imports that cross a package boundary into another SDKWork repository or
  another workspace package's `src/` (for example `import ... from "../../sdkwork-appbase/.../src/..."`).
- Consume only the public `exports` surface of a package; never deep-import sibling `src/` internals.
- Every non-relative import in a workspace member MUST resolve to that member's own
  `dependencies`/`devDependencies`/`peerDependencies` (import closure).
- Vite aliases MUST NOT rename or redirect `@sdkwork/*` packages, MUST NOT be added to make a
  resolution error pass, and are allowed only for documented bootstrap/SDK-generation entrypoints.
- Fix a resolution failure by correcting the workspace declaration or the package `exports`,
  not by adding an alias.

Verification:

```bash
node ../sdkwork-specs/tools/verify-repo.mjs --root .
node ../sdkwork-specs/tools/check-workspace-member-protocol.mjs --root .
node ../sdkwork-specs/tools/check-dependency-list-completeness.mjs --target <repo-name>
```
<!-- /SDKWORK-PNPM-WORKSPACE-STANDARD: v1 -->

<!-- SDKWORK-SDK-GENERATION-STANDARD: v1 -->
## Generated SDK Output Is Generator-Owned

Authority: `../sdkwork-specs/SDK_SPEC.md` and `../sdkwork-specs/SDK_WORKSPACE_GENERATION_SPEC.md`.

Everything generated under `sdks/` — `generated/server-openapi/` trees, generated language
workspaces, `dist/` build output, generated `sdkwork-sdk.json`, generated
`.sdkwork/sdkwork-generator-*` reports, and standardizer-synced OpenAPI snapshots — is produced by
the canonical SDK generator `../sdkwork-sdk-generator/bin/sdkgen.js` (`@sdkwork/sdk-generator`).

- Do not hand-edit generated SDK files, including type definitions, dist bundles, and generated
  package metadata. Manual edits are overwritten by the next generation run and break
  reproducibility and contract audits.
- When generated or compiled SDK output does not meet a contract or standard, fix the upstream
  source — authored API contract, route manifest, OpenAPI authority, derived `*.sdkgen.*` input,
  generator profile, or `custom/` runtime build scripts — then regenerate through the standard
  generation command. Do not patch generated output in place.
- Remove stale generated files by re-running the family generation command, which owns cleanup of
  disappeared routes and models; do not hand-prune generated trees.
- The only approved handwritten surfaces are `custom/` roots inside generated workspaces and
  authored `composed/` facades outside `generated/server-openapi`.

Verification:

```bash
node ../sdkwork-specs/tools/sync-agent-sdk-generation-standard.mjs --root . --check
```
<!-- /SDKWORK-SDK-GENERATION-STANDARD: v1 -->
