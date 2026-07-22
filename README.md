# sdkwork-local-router
repository-kind: application

Rust local router for model API compatibility across OpenAI-compatible tools,
Claude Code, Gemini CLI, and vendor-native upstreams.

Client tools authenticate to the local router with database-backed
`client_api_key` records in
`local_router_client_api_keys`; each row provides the `user_id` used for data
isolation. Provider credentials are separate `upstream_api_key` values on
`local_router_upstream_accounts` and are used only when forwarding upstream.

The server exposes three service-side API groups:

- `local-router-open-api`: provider-compatible model proxy routes such as
  `/local-router/v1/*`, `/local-router/anthropic/*`, and
  `/local-router/google/*`; authenticated by local-router
  `client_api_key` records.
- `local-router-app-api`: `/app/v3/api/local_router/*` app integration routes;
  authenticated from `sdkwork-iam` subject projection supplied by the
  surrounding SDKWork runtime.
- `local-router-backend-api`: `/backend/v3/api/local_router/*` operator and
  control-plane routes for accounts, client API keys, usage, health, plugins,
  routing strategy, and API group manifests.

`GET /backend/v3/api/local_router/api_groups` returns the machine-readable API group
manifest. `GET /backend/v3/api/local_router/plugins` embeds the same groups together
with plugin standards, routing strategies, pipeline stages, and standard
components.

See [docs/architecture/tech/TECH-plugin-compatibility.md](docs/architecture/tech/TECH-plugin-compatibility.md) for the plugin
standard, canonical plugin names, and `sdkwork-models` vendor compatibility
rules.

## SDKWork Documentation Contract

Domain: device
Capability: local-router
Package type: rust-crate
Status: standard

### Public API

Public exports are declared in `specs/component.spec.json` under `contracts.publicExports`.

### Required SDK Surface

- None declared in `specs/component.spec.json`.

### Configuration

Configuration keys and runtime entrypoints are declared in `specs/component.spec.json`.

### SaaS/Private/Local Behavior

This module follows the canonical standards linked from `specs/component.spec.json`, including deployment and runtime configuration rules where applicable.

### Security

Do not add secrets, live tokens, manual auth headers, or app-local credential handling to this module.

### Extension Points

Extension points are limited to declared public exports, runtime entrypoints, SDK clients, events, and config keys.

### Verification

- `cargo test`

### Owner And Status

Owner and lifecycle status are tracked in `specs/component.spec.json`.

## Documentation Canon

- [docs/README.md](docs/README.md)
- [docs/product/prd/PRD.md](docs/product/prd/PRD.md)
- [docs/architecture/tech/TECH_ARCHITECTURE.md](docs/architecture/tech/TECH_ARCHITECTURE.md)

## Application Roots

- [apps directory index](apps/README.md)
