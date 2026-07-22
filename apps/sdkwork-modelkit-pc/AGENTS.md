# Repository Guidelines

<!-- SDKWORK-AGENTS-GENERATED: v2 -->

## SDKWORK Soul

Read `../../../sdkwork-specs/SOUL.md` before executing tasks in this application root. Apply specs
before memory, dictionary before context, exact sources before inference, and evidence before
completion.

## SDKWORK Standards

The canonical standards entrypoint is `../../../sdkwork-specs/README.md`. This entrypoint follows
`../../../sdkwork-specs/AGENTS_SPEC.md` and narrows the parent `../../AGENTS.md` contract.

## Application Identity

- Application id: `sdkwork-local-router-sdkwork-modelkit-pc`
- Runtime family: React/Vite PC application
- Application declaration: `sdkwork.app.config.json`
- Source configuration entrypoint: `etc/sdkwork.deployment.config.json`
- Component contract: `specs/component.spec.json`

The parent Local Router application owns API routes, runtime composition, persistence, and the
provider-compatible ingress contract. This UI consumes those capabilities through approved SDK or
host boundaries and must not create a second HTTP API authority.

## Local Dictionary Structure

- `AGENTS.md`: PC application execution entrypoint.
- `sdkwork.app.config.json`: application and release identity.
- `etc/`: source configuration governed by `SOURCE_CONFIG_SPEC.md`.
- `packages/`: focused application packages.
- `sdks/`: app-local SDK composition surfaces when declared.
- `src/`: application bootstrap and shell entrypoint.
- `package.json`: scripts and dependency authority.
- `vite.config.ts`, `tsconfig.json`: Vite and TypeScript build authority.

## Spec Resolution Order

1. Read this `AGENTS.md`.
2. Read `../../AGENTS.md` for repository-wide boundaries.
3. Read this root's `sdkwork.app.config.json` for application identity or runtime work.
4. Read the nearest package manifest and `specs/` only for the affected package.
5. Resolve the task row in `../../../sdkwork-specs/README.md` and read only selected specs.
6. Inspect implementation files.

## Required Specs By Task Type

- Agent/workflow: `SOUL.md`, `AGENTS_SPEC.md`, `SDKWORK_WORKSPACE_SPEC.md`, and `TEST_SPEC.md`.
- Any code: `CODE_STYLE_SPEC.md`, `NAMING_SPEC.md`, `TYPESCRIPT_CODE_SPEC.md`, and `TEST_SPEC.md`.
- Package scripts: `PNPM_SCRIPT_SPEC.md`, `CONFIG_SPEC.md`, and `TEST_SPEC.md`.
- Frontend/UI: `FRONTEND_CODE_SPEC.md`, `FRONTEND_SPEC.md`, `UI_ARCHITECTURE_SPEC.md`,
  `APP_PC_ARCHITECTURE_SPEC.md`, and `APP_PC_REACT_UI_SPEC.md`.
- SDK integration: `APP_SDK_INTEGRATION_SPEC.md`, `SDK_SPEC.md`,
  `SDK_WORKSPACE_GENERATION_SPEC.md`, and `TEST_SPEC.md`.
- List/search: add `PAGINATION_SPEC.md`.
- Source config/runtime: `SOURCE_CONFIG_SPEC.md`, `CONFIG_SPEC.md`, `ENVIRONMENT_SPEC.md`,
  `DEPLOYMENT_SPEC.md`, and `TEST_SPEC.md`.
- Packaging/workflows: `PNPM_SCRIPT_SPEC.md`, `GITHUB_WORKFLOW_SPEC.md`, `RELEASE_SPEC.md`, and
  `SUPPLY_CHAIN_SECURITY_SPEC.md`.

Language-specific specs are on-demand; do not load unrelated language or framework specs.

## Code Style Rules

- UI and feature packages consume injected generated SDK clients or approved composed facades.
- Do not add raw HTTP, manual auth headers, local SDK forks, duplicate wire DTOs, or imports into
  generator-owned internals.
- Bootstrap owns runtime configuration and SDK client construction; UI components do not.
- Use approved `sdkwork-utils` implementations for shared utility behavior.

## Agent Execution Rules

Use dynamic progressive loading before implementation files. Do not replace SDK integration with
raw HTTP, hand-edit generated SDK transport, or change Local Router API ownership from this UI root.

## Build, Test, And Verification

Run `pnpm lint` and `pnpm build` from this application root. Run `pnpm check` and `pnpm verify` from
the parent root for cross-surface verification.

For list/search changes, run
`node ../../../sdkwork-specs/tools/check-pagination.mjs --workspace ../..` from this application
root.

## Human Review Rules

Human review is required for breaking API/SDK behavior, security or auth changes, runtime config
semantics, generated ownership changes, and release or deployment governance changes.
