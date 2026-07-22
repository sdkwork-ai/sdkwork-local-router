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
