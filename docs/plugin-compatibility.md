# Plugin Compatibility Standard

`sdkwork-local-router` exposes model API conversions as standard plugins. The
public package boundary is `sdkwork-lr-plugin`; built-in conversion
implementations live in `sdkwork-lr-transform`.

## Package Boundaries

- `sdkwork-lr-plugin`: shared plugin API, manifests, canonical names, registry,
  path mapping, standard API surface metadata, standard client API metadata,
  fallback/effective plugin decisions, and model compatibility resolver.
- `sdkwork-lr-transform`: built-in JSON transform plugins for OpenAI Chat
  Completions, OpenAI Responses, Anthropic Messages, and Gemini GenerateContent.
- `sdkwork-local-router`: runtime loader that combines route surface, upstream
  account, model metadata, and plugin registry.
- `sdkwork-models`: git dependency sourced from
  `https://github.com/Sdkwork-Cloud/sdkwork-models.git`, used for model,
  vendor, protocol, and client API compatibility metadata.

## SDK Generation Standard

Any generated SDK for `sdkwork-local-router` open, app, or backend APIs must
follow the SDKWork standards in
`../sdkwork-specs/SDK_SPEC.md` and
`../sdkwork-specs/SDK_WORKSPACE_GENERATION_SPEC.md`.
The only standard HTTP SDK generator is
`..\sdkwork-sdk-generator`
(`@sdkwork/sdk-generator` / `sdkgen`), using OpenAPI authority documents and
derived `*.sdkgen.yaml` inputs. Local-router SDK generation commands,
manifests, and release notes must record that generator path or resolved
package location, generator version or commit, input spec, output path,
language, SDK type, SDK family name, package name, and standard profile.

`sdkwork-code-generator`, local stub generators, copied generator source,
generic OpenAPI client generators, and handwritten SDK forks are not accepted
as production SDK sources for this project. A project-local script may use one
of those names only when it is a thin documented wrapper around the canonical
`sdkwork-sdk-generator`.

## Server API Groups

The server is split into three explicit API groups. The group code is part of
the runtime request context and is recorded in audit metadata as
`audit.api_group`.

| API group | Runtime paths | Audience | Auth and user source |
| --- | --- | --- | --- |
| `local-router-open-api` | Provider-compatible proxy paths from `[base_paths]`, for example `/v1/{*path}`, `/anthropic/{*path}`, and `/google/{*path}` | Codex, Claude Code, Gemini CLI, OpenAI-compatible SDKs, and model clients | Database-backed `client_api_key`; `user_id` comes from `local_router_client_api_keys.user_id` |
| `local-router-app-api` | `/app/v3/api/local_router/*` | SDKWork app clients and local/private app integrations | SDKWork auth/access tokens, JWT claims, or trusted subject headers |
| `local-router-backend-api` | `/backend/v3/api/local_router/*` | Backend SDKs, admin consoles, operators, and control-plane services | SDKWork auth/access tokens, JWT claims, or trusted subject headers |

`local-router-open-api` preserves provider-compatible URL shapes because
existing tools expect OpenAI, Anthropic, or Gemini-compatible paths. Its group
name is therefore a server-side contract for auth, routing, audit, monitoring,
and SDK manifest purposes rather than a URL prefix.

The machine-readable group manifest is available at:

```text
GET /backend/v3/api/local_router/api_groups
```

The same `api_groups` array is also embedded in
`/backend/v3/api/local_router/plugins` so third-party SDKs and admin tooling can
read API grouping, plugin standards, interceptor stages, and routing strategies
from one endpoint.

## Canonical Plugin Names

Plugin IDs are uppercase and use concrete API surface names:

- `OPENAI_CHAT_COMPLETIONS_TO_OPENAI_RESPONSES_API`
- `OPENAI_RESPONSES_TO_OPENAI_CHAT_COMPLETIONS_API`
- `OPENAI_BATCH_TO_OPENAI_CHAT_COMPLETIONS_API`
- `OPENAI_BATCH_TO_OPENAI_RESPONSES_API`
- `OPENAI_RESPONSES_TO_ANTHROPIC_MESSAGES_API`
- `OPENAI_RESPONSES_TO_GEMINI_GENERATE_CONTENT_API`
- `ANTHROPIC_MESSAGES_TO_OPENAI_CHAT_COMPLETIONS_API`
- `GEMINI_GENERATE_CONTENT_TO_OPENAI_CHAT_COMPLETIONS_API`
- `ANTHROPIC_MESSAGES_TO_OPENAI_RESPONSES_API`
- `GEMINI_GENERATE_CONTENT_TO_OPENAI_RESPONSES_API`
- `ANTHROPIC_MESSAGES_TO_GEMINI_GENERATE_CONTENT_API`
- `GEMINI_GENERATE_CONTENT_TO_ANTHROPIC_MESSAGES_API`

Documented aliases such as
`OPENAI_COMPATIBLE_CHAT_TO_RESPONSE_API`,
`OPENAI_COMPATIBLE_RESPONSE_TO_CLAUDE_MESSAGE_API`,
`CLAUDE_MESSAGE_TO_OPENAI_CHAT_API`, and
`GEMINI_MESSAGE_TO_OPENAI_RESPONSE_API` are normalized to canonical IDs.
Misspelled plugin IDs are rejected; plugin manifests advertise only canonical
IDs and documented aliases.

## API Surface Standard

`sdkwork-lr-plugin::standard_api_surfaces()` exports the machine-readable API
surface contract used by the router and by third-party applications that want
to preflight compatibility before sending traffic. Each entry includes:

- `code`: the protocol code used by `sdkwork-models`, such as
  `openai_responses`, `openai_compatible`, `openai_batch`,
  `anthropic_messages`, or `google_gemini`.
- `token`: the canonical uppercase token used in plugin IDs.
- `protocol`: router protocol family: `openai`, `anthropic`, or `google`.
- `request_path` and optional `stream_path`: the normalized generation route.
- `required_request_fields`, `optional_request_fields`, `response_fields`, and
  `streaming_event_types`: the compact field/event contract used to validate
  conversion support.

The current standard covers these official-compatible surfaces:

- OpenAI Chat Completions: `/v1/chat/completions`; requires `model` and
  `messages`; supports `tools`, `tool_choice`, `tool_calls`, `stream`,
  `stream_options`, `response_format`, `response_format.json_schema`,
  `parallel_tool_calls`, `max_completion_tokens`, `metadata`, `store`,
  `service_tier`, `safety_identifier`, prompt cache hints, and `top_logprobs`
  in the compatibility schema.
- OpenAI Responses: `/v1/responses`; requires `model` and `input`; supports
  `instructions`, `tools`, `tool_choice`, `max_output_tokens`, `reasoning`,
  `text`, `text.format`, `stream`, `stream_options`, `metadata`, `store`,
  `service_tier`, `safety_identifier`, prompt cache hints, `top_logprobs`,
  `truncation`, `output`, `output_text`, Codex-oriented `shell`/`local_shell`
  tools, and Responses event names such as `response.output_text.delta`.
- OpenAI Batch: `/v1/batches`; requires `input_file_id`, `endpoint`, and
  `completion_window`. Batch execution plugins are reserved until work-item
  expansion and result collation are implemented.
- Anthropic Messages: `/v1/messages`; requires `model`, `max_tokens`, and
  `messages`; supports `system`, `tools`, `tool_choice`, `stop_sequences`,
  `stream`, `content`, `stop_reason`, `usage`, and Anthropic SSE events such
  as `message_start`, `content_block_delta`, and `message_stop`.
- Gemini GenerateContent: `/v1/models/{model}:generateContent` and
  `/v1/models/{model}:streamGenerateContent?alt=sse`; requires `contents`;
  supports `systemInstruction`, `tools`, `tools.functionDeclarations`,
  `toolConfig`, `toolConfig.functionCallingConfig`, `generationConfig`,
  `generationConfig.maxOutputTokens`, `generationConfig.temperature`,
  `generationConfig.topP`, `generationConfig.candidateCount`,
  `generationConfig.presencePenalty`, `generationConfig.frequencyPenalty`,
  `generationConfig.seed`, `generationConfig.stopSequences`,
  `generationConfig.responseMimeType`, `generationConfig.responseSchema`,
  `candidates`, `finishReason`, and `usageMetadata`.

Non-generation resource paths under the provider-compatible prefixes, such as
OpenAI `/v1/models`, `/v1/files`, or provider-native model listing endpoints,
are not treated as generation API surfaces. They are filtered to same-protocol
upstream accounts and forwarded without body conversion. This prevents Codex,
Claude Code, Gemini CLI, and OpenAI-compatible SDK initialization calls from
being accidentally routed into `/v1/messages`, `/v1/responses`, or
`generateContent`.

`sdkwork-lr-plugin::standard_client_apis()` exports the client tool standard:
`codex` defaults to OpenAI Responses, `claude_code` defaults to Anthropic
Messages, and `gemini_cli` defaults to Gemini GenerateContent. The alias
normalizer accepts common spellings such as `claude-code`, `gemini-cli`, and
`openai-codex`.

## Vendor Compatibility Data

The router reads `sdkwork-models` vendor and model data:

- `ModelInfo.apiFormat` selects the vendor model API surface, such as
  `openai_compatible`, `openai_responses`, `anthropic_messages`, or
  `google_gemini`.
- `ModelVendor.supportedProtocols` declares vendor-supported standard
  protocols.
- `ModelVendor.clientApiCompatibility` declares client tool support for:
  `codex`, `claude_code`, and `gemini_cli`.

If a vendor omits a client API compatibility entry, the router treats that
support as `unspecified` and falls back to model `apiFormat` plus request route
surface to select a conversion plugin. The standard for new vendor data is:

- `supportStatus`: `supported`, `partial`, or `unsupported`.
- `protocolCodes`: standard protocol codes that the client can use natively.
- `apiCodes`: specific client API surfaces, if vendor-native.
- `resourceCodes`: vendor-specific API resources.
- `notes` and `source`: evidence for the compatibility claim.

Resolver behavior is conservative and account-aware:

- If `supportStatus` is `supported` or `partial`, `protocolCodes` contains a
  known API surface, and the incoming request route already uses that surface,
  and the selected upstream account also uses that surface, the router treats
  the vendor as natively compatible for that client API and does not load a
  conversion plugin.
- If the selected upstream account uses a standard protocol declared in
  `ModelVendor.supportedProtocols`, the router may use that upstream surface
  directly even when the model's default `apiFormat` is different. This covers
  vendors such as DeepSeek and Zhipu/GLM where a model may default to
  `openai_compatible` while the vendor also exposes an Anthropic-compatible
  Messages endpoint for Claude Code. `apiFormat` is the default model surface,
  not the only legal upstream protocol.
- OpenAI Chat Completions and OpenAI Responses are both OpenAI-generation
  surfaces. When choosing between those two, the router keeps the model
  `apiFormat` preference so OpenAI Responses-native models are not
  accidentally downgraded to Chat Completions just because the selected
  upstream provider is OpenAI-compatible.
- If support is `unsupported`, `unspecified`, or the incoming route does not
  match the declared client surface, the router uses the model's `apiFormat` as
  the target surface unless the selected upstream surface is explicitly listed
  in the vendor's `supportedProtocols`. It loads the canonical conversion
  plugin when source and target differ.

For example, DeepSeek `deepseek-v4-pro` and Zhipu/GLM `glm-5.1` currently use
`apiFormat = openai_compatible` in `sdkwork-models`, and their
`clientApiCompatibility.claude_code` entries are `unsupported` because they do
not expose the Claude Code client API as a vendor-native client product. If the
selected upstream account is OpenAI-compatible, Claude Code `/v1/messages`
requests are converted with
`ANTHROPIC_MESSAGES_TO_OPENAI_CHAT_COMPLETIONS_API`. If the selected upstream
account is Anthropic Messages-compatible and that protocol is declared in
`supportedProtocols`, the same Claude Code request is passed through without a
plugin because the upstream surface already matches the client request surface.

## Runtime Decision Flow

For each proxied request:

1. The route path determines the client API surface.
2. The optional `x-sdkwork-client-api` request header declares the client tool
   API code. Supported aliases such as `claude-code`, `claude_code`,
   `gemini-cli`, and `gemini_cli` are normalized to the standard codes used by
   `sdkwork-models`.
3. When the header is absent, the router infers the client API code from the
   route surface: OpenAI Responses routes infer `codex`, Anthropic Messages
   routes infer `claude_code`, and Gemini GenerateContent routes infer
   `gemini_cli`.
4. The selected upstream account determines the provider protocol fallback.
5. The model catalog maps the requested model to vendor and `apiFormat`.
6. The compatibility resolver decides the target API surface.
7. The router builds an `EffectivePluginDecision` from the model decision and
   runtime policy, then preflights the matching route in the plugin registry.
8. The plugin maps path, request body, non-streaming response body, and
   streaming SSE events when its route declares `capabilities.stream = true`.

Provider-compatible `key=` query parameters are accepted only as local-router
client API key input on proxy routes. The parameter name is matched
case-insensitively after URL decoding, so `key`, `Key`, and encoded key names
are treated as local-router credentials. Before forwarding upstream, the router
removes those `key` parameters from the upstream query string and injects the
configured `upstream_api_key` through the provider-specific upstream auth
mechanism instead. This keeps local client API keys from leaking to
Google/Gemini upstreams and avoids colliding with upstream API key auth.

If the model catalog cannot be loaded, the router falls back to provider-based
protocol routing by using
`sdkwork_lr_plugin::fallback_compatibility_decision(...)`. The decision
endpoint still returns a fallback payload instead of failing.

Preflight blocks requests with HTTP 501 before contacting upstreams when a
conversion is required but the plugin is missing, reserved, or lacks required
capabilities for path mapping, request body mapping, response body mapping, or
streaming.

## Runtime Policy

The `[plugins]` TOML section controls runtime selection:

```toml
[plugins]
enabled = true
policy = "auto"
expose_decision_headers = true
```

Supported policies:

- `auto`: default. Use `sdkwork-models` model/vendor metadata to choose the
  target API surface. If no catalog data exists, fall back to the selected
  upstream provider surface.
- `strict`: same target selection as `auto`, but request transformation errors
  fail the request with HTTP 400 instead of forwarding the original body to an
  incompatible upstream API.
- `passthrough`: disable API body/path conversion and keep the client API
  surface. Model route mappings configured on the upstream account still apply.
- `force_transform`: ignore the model catalog target surface and convert to the
  selected upstream provider's default surface.

`enabled = false` is equivalent to `policy = "passthrough"` at runtime.

## Model Route Mapping

Each upstream account can declare model route mappings as a client-model to
upstream-model routing map. The durable runtime source is the
`local_router_model_route_mappings` table, with the account-level
`model_route_mappings` field used as the account create/update import surface.
This lets a client tool keep a stable local model name such as `gpt-5.5` while
the router sends the request to the provider's real model identifier, for
example `gemini-2.5-pro`, `claude-sonnet-4-20250514`, or a dated OpenAI model id.

The route mapping is applied consistently across the routing chain:

- Account selection matches either the requested client model or the mapped
  upstream model against the account's `models` patterns.
- `sdkwork-models` compatibility decisions use the mapped upstream model, so
  vendor support and plugin selection are based on the real provider model.
- Upstream request paths use the mapped model for Gemini
  `models/{model}:generateContent` routes.
- Upstream JSON bodies use the mapped model for OpenAI Chat Completions,
  OpenAI Responses, and Anthropic Messages targets when those bodies require a
  `model` field.
- Account create/update requests can synchronize the mapping rows from the
  `model_route_mappings` object. Direct route-mapping management APIs can
  update rows without replacing unrelated account configuration.

Example:

```toml
[[upstreams]]
name = "gemini-for-codex"
provider = "google"
base_url = "https://generativelanguage.googleapis.com"
upstream_api_key = ""
models = ["gemini-*"]

[upstreams.model_route_mappings]
"gpt-5.5" = "gemini-2.5-pro"
```

With that configuration, a Codex/OpenAI-compatible client can call
`/v1/responses` or `/v1/chat/completions` with `"model": "gpt-5.5"`. The router
selects the Gemini account, transforms the request when needed, and forwards it
to `/v1/models/gemini-2.5-pro:generateContent` or
`/v1/models/gemini-2.5-pro:streamGenerateContent?alt=sse`.

Backend management APIs for real-time changes:

```text
GET    /backend/v3/api/local_router/model_route_mappings
POST   /backend/v3/api/local_router/model_route_mappings
DELETE /backend/v3/api/local_router/model_route_mappings/{modelRouteMappingId}
```

`POST /backend/v3/api/local_router/model_route_mappings` accepts:

```json
{
  "account_id": 1,
  "client_model": "gpt-5.5",
  "upstream_model": "gemini-2.5-pro",
  "enabled": true,
  "notes": "Codex local GPT name to Gemini official model"
}
```

The route upserts on `(user_id, account_id, client_model)`, increments the row
`version` on updates, reloads the resolved user's account pool immediately, and
never crosses user boundaries.

## Routing Strategy Standard

The router supports account routing at the user account-pool level. Every
request first resolves `user_id`, then loads that user's enabled upstream
accounts, filters by requested model and account health, and finally orders the
candidate list by the configured routing strategy. If `routing.strategy` is
omitted or set to `auto`, the router uses `priority` for a single available
account and `round_robin` for multiple accounts. This keeps single-account
deployments deterministic while making multi-account local-router deployments
distribute the first attempt across equivalent accounts by default. An explicit
non-`auto` `routing.strategy` value always overrides the automatic default.

Supported routing strategies:

- `auto`: clears any per-user runtime override and applies the account-count
  default, which is `priority` for one account and `round_robin` for multiple
  accounts.
- `priority`: deterministic order. Candidates are sorted by ascending
  `priority`. Fallback uses that same order. This is the automatic default for
  single-account pools and remains useful for active/passive routing.
- `round_robin`: recommended for multiple equivalent accounts. Candidates are
  first sorted by priority and then rotated on every request, so fallback starts
  from a different account across calls. This strategy is applied to
  `select_all`, which is the real passthrough candidate path, not only to
  single-account selection. This is the automatic default for multi-account
  pools when no explicit non-`auto` strategy is configured.
- `random`: candidates are shuffled per request for lightweight distribution.
- `least_latency`: candidates are sorted by recorded average latency, with
  `priority` as the deterministic tie-breaker.

Runtime strategy can be changed per resolved user scope with:

```text
POST /backend/v3/api/local_router/strategy
```

Request body:

```json
{"strategy":"round_robin"}
```

Valid strategy codes are `auto`, `priority`, `round_robin`, `random`, and
`least_latency`. `auto` removes the current user's runtime strategy override and
recomputes the default from that user's account count. The response includes the
resolved request context, `user_id`, strategy mode, old strategy, and new
strategy. Explicit in-memory strategy overrides are preserved when that user's
account pool is hot-reloaded from database records.

`/backend/v3/api/local_router/plugins` exposes `routing_strategies`, so
third-party management tools can read the available strategy contract without
hard-coding it.

## Server Pipeline And Extension Points

The local router uses a server-side interceptor chain in `sdkwork-lr-core`.
Interceptors are service components, not UI plugins. They are intended for
security policy, monitoring, concurrency control, billing/quota, audit,
redaction, and custom routing extensions.

The chain exposes these standard stages:

| Stage | Method | Blocking | Purpose |
| --- | --- | --- | --- |
| `request_received` | `on_request` | yes | Initialize request metadata, enforce coarse security policy, attach trace or billing context. |
| `request_body_read` | `on_request_body_read` | yes | Inspect body size and request metadata after body read and before account routing. |
| `route_candidates` | `on_route_candidates` | yes | Inspect, filter, or reorder upstream candidates for security, quota, cost, region, or policy routing. |
| `account_selected` | `on_account_selected` | yes | Validate the selected account before transform and forwarding. |
| `before_transform` | `on_before_transform` | yes | Run pre-transform validation, model policy checks, and compatibility checks. |
| `before_forward` | `on_before_forward` | yes | Apply concurrency control, final quota checks, outbound safety checks, and monitoring correlation. |
| `upstream_response` | `on_upstream_response` | yes | Observe upstream status before fallback handling and final response transformation. |
| `response_ready` | `on_response` | no | Add response metrics, usage metadata, and completion observations after final response is known. |
| `before_persist` | `on_before_persist` | no | Finalize audit, billing, usage, and redaction metadata before invocation persistence. |
| `error` | `on_error` | no | Observe failures for security events, audit, metrics, and alerting. |

Blocking stages return `InterceptorError`, which carries `http_status`, `code`,
and `message`. This lets extensions return appropriate protocol-compatible
responses, for example:

- Security policy: HTTP `403` with code `interceptor_forbidden`.
- Concurrency control or rate saturation: HTTP `429` with code
  `interceptor_too_many_requests`.
- Billing or quota exhaustion: HTTP `402` with code
  `interceptor_payment_required`.
- Invalid request policy: HTTP `400` with code `interceptor_bad_request`.

Response-ready and before-persist stages are observation/finalization stages.
Their errors are logged and recorded into invocation metadata when possible, but
they do not invalidate an upstream response that has already been produced.

Standard server components should map to the pipeline this way:

- `auth_context_resolver`: middleware. Resolves `user_id` from
  database-backed `client_api_key` on proxy routes, or SDKWork token/JWT context
  on app/backend routes. Built in and enabled by default.
- `routing_policy`: account pool plus `route_candidates` and
  `account_selected`. Applies priority, round-robin, random, least-latency,
  model, health, user, and fallback selection. Built in and enabled by default.
- `security_policy`: interceptor on `request_received`, `request_body_read`,
  `route_candidates`, `before_transform`, and `before_forward`. Blocks unsafe
  payloads, forbidden models, forbidden vendors, disallowed regions, or
  sensitive payload patterns. `SecurityPolicyInterceptor` and
  `SecurityPolicyConfig` are exported by `sdkwork-lr-core`, but the blocking
  policy component is not enabled by default.
- `concurrency_control`: interceptor on `before_forward`, `response_ready`, and
  `error`. Enforces per-user, per-model, per-account, or global in-flight
  limits. `ConcurrencyLimitInterceptor` and `ConcurrencyLimitConfig` are
  exported by `sdkwork-lr-core`, but the blocking limit component is not enabled
  by default.
- `billing_meter`: interceptor on `before_forward`, `response_ready`, and
  `before_persist`. Reserves quota before forwarding, reads token usage after
  response parsing, and finalizes usage or billing metadata before persistence.
  `BillingQuotaInterceptor`, `BillingQuotaConfig`, and `BillingQuotaScope` are
  exported by `sdkwork-lr-core`, but quota enforcement is not enabled by
  default.
- `observability`: interceptor on `request_received`, `route_candidates`,
  `upstream_response`, `response_ready`, and `error`. Emits structured logs,
  metrics, traces, fallback counters, latency, and token counters without
  logging secrets. Logging and metrics interceptors are enabled by default.
- `audit_metadata`: interceptor on `before_persist`. Attaches non-secret
  request, user, route, plugin, model, account, and status metadata. Built in
  and enabled by default through `AuditMetadataInterceptor`.
- `invocation_recorder`: persistence component using `before_persist`. Persists
  request, routing, plugin, response, error, and token usage records into
  `local_router_` tables with user isolation.

`/backend/v3/api/local_router/plugins` exposes `pipeline_stages` and
`standard_components` as machine-readable metadata for SDKs, backend admin
tools, and third-party integrations. Each standard component declares whether a
built-in implementation is available, whether it is enabled by default, the
blocking/non-blocking stages it uses, and the configuration source or exported
Rust type names.

## Decision Observability

When `expose_decision_headers = true`, proxied responses include diagnostic
headers:

- `x-sdkwork-plugin-policy`: effective policy, such as `auto` or `strict`.
- `x-sdkwork-plugin-source`: source API surface.
- `x-sdkwork-plugin-target`: target API surface.
- `x-sdkwork-plugin-id`: canonical plugin ID when conversion is required.
- `x-sdkwork-model-vendor`: vendor code from `sdkwork-models`, when available.
- `x-sdkwork-client-api`: effective client tool API code, either declared by
  request header or inferred from the route, such as `codex`, `claude_code`, or
  `gemini_cli`. This header is returned even when no model catalog decision was
  available.

The browser CORS layer accepts the `x-sdkwork-client-api` request header and
exposes these response headers together with `x-request-id`.
`/app/v3/api/local_router/status` and
`/backend/v3/api/local_router/plugins` also expose plugin enablement, policy,
registry count, decision-header setting, and model catalog load status.

## User Isolation

All local-router owned database tables use the `local_router_` prefix:

- `local_router_upstream_accounts`
- `local_router_model_route_mappings`
- `local_router_client_api_keys`
- `local_router_invocations`
- `local_router_usages`

The local router does not own or maintain a user table. User identity is stored
only as `user_id` relationship data on router-owned rows. A missing or invalid
user id is normalized to `0`, which is the default local user scope.

Every user-owned router table includes `user_id`. Upstream account names are
unique only within one user via `(user_id, name)`, so separate users can keep
accounts with the same display name or provider shape. Invocation logs and
token usage are filtered by `user_id` for every list and aggregate query.

Model route mappings are stored as first-class user-owned rows in
`local_router_model_route_mappings`:

- `user_id`: owner scope, defaulting to `0` when no user context is present.
- `account_id` and `account_name`: selected upstream account binding.
- `client_model`: model name sent by Codex, Claude Code, Gemini CLI, or another
  client.
- `upstream_model`: real provider model sent upstream.
- `enabled`, `notes`, `version`, `created_at`, and `updated_at`.
- Unique key: `(user_id, account_id, client_model)`.
- Foreign key: `(user_id, account_id)` references
  `local_router_upstream_accounts(user_id, id)`, so the database also rejects
  route mappings that point at another user's account.

Deleting an upstream account removes its model route mapping rows. Renaming an
upstream account refreshes the `account_name` snapshot without overwriting
independently managed route mappings. Runtime reloads read enabled rows from
this table and merge them with the account-level `model_route_mappings` object.

There are two distinct key concepts:

- `client_api_key`: the local-router/open-platform key accepted from client
  tools on proxy routes. It is stored only as `key_hash` plus `key_prefix` in
  `local_router_client_api_keys`, and its database row carries `user_id`.
- `upstream_api_key`: the provider credential stored on
  `local_router_upstream_accounts` and injected only when forwarding to
  OpenAI, Anthropic, Gemini, or another upstream. It must never be used to
  identify a local-router caller.
- `upstream_auth_scheme`: optional account-level upstream credential injection
  mode. Omit it for provider defaults (`openai`/custom use Bearer,
  `anthropic` uses `x-api-key`, and `google` uses `x-goog-api-key`). Set it to
  `bearer`, `x-api-key`, `x-goog-api-key`, or `query-key` when a
  provider-compatible endpoint requires a different upstream auth shape. This
  is required for some Anthropic-compatible non-Anthropic accounts, such as
  DeepSeek/Z.AI Claude Code-compatible endpoints that document
  `Authorization: Bearer`.

Proxy/OpenAI/Claude/Gemini-compatible requests resolve `user_id` by looking up
the presented `client_api_key` in `local_router_client_api_keys`. The key may
arrive through `Authorization: Bearer`, `x-api-key`, `x-goog-api-key`, or the
Gemini-compatible `key` query parameter. A missing or invalid client API key
returns `401 invalid_client_api_key`.

App and backend APIs do not use client API keys for local identity. They resolve
`user_id` from SDKWork auth/access tokens, JWT claims, or trusted subject
headers used by the surrounding SDKWork runtime. If no user context is present,
the request runs in user scope `0`.

Local authentication is not configured through TOML. The local-router contract
uses database-backed client API key records for proxy callers and SDKWork token
context for app/backend callers.

`/app/v3/api/local_router/status`, model listing, account APIs, health APIs,
usage summary, and logs all include the resolved request context and read/write
only data owned by that `user_id`. Runtime proxy traffic selects the same
user's account pool after client API key authentication.

## Invocation Records

Each proxied request is persisted to `local_router_invocations` with enough
metadata to audit routing and plugin decisions:

- `user_id`, `request_id`, `account_name`, `protocol`, `method`, `path`,
  `query`, and `model`.
- `status`, `status_code`, `latency_ms`, and `error_message`.
- Optional `request_body` and `response_body` when recording is enabled, plus
  `request_body_size` and `response_body_size`.
- `upstream_protocol`, `upstream_path`, `client_api`, `request_surface`,
  `target_surface`, `plugin_policy`, `plugin_id`, `model_vendor`, `streaming`,
  and `attempt_count`.
- `metadata`, a JSON object for non-secret interceptor and audit metadata such
  as `audit.user_id`, `audit.api_group`, `audit.account_name`,
  `route_candidate_count`, `upstream_status_code`, `latency_ms`, and optional
  security/concurrency/quota markers. Core query fields must stay in
  first-class columns instead of being hidden only in this JSON object.

Usage rows in `local_router_usages` also include `user_id`, so token totals and
per-model aggregations remain user-isolated.

`/backend/v3/api/local_router/plugins` additionally exposes:

- `api_groups`: the three local-router server API groups:
  `local-router-open-api`, `local-router-app-api`, and
  `local-router-backend-api`.
- `api_surfaces`: the machine-readable API surface standard described above.
- `client_apis`: the machine-readable client tool standard.
- `route_capabilities`: every standard source/target route with `registered`,
  `ready`, `reason`, and declared `capabilities`.
- `routing_strategies`: `auto`, `priority`, `round_robin`, `random`, and
  `least_latency`.
- `pipeline_stages`: the stable interceptor extension points listed above.
- `standard_components`: the built-in availability and default enablement
  contract for auth, routing, security, concurrency, billing, observability,
  audit metadata, and invocation recording.
- `capability_summary`: counts for `ready`, `missing`, `reserved`, `partial`,
  and total standard routes.

Third-party tools can preflight the exact model/client decision with:

```text
GET /backend/v3/api/local_router/plugins/decision?model=<model>&client_api=<client>&source=<surface>&upstream=<surface>
```

Parameters:

- `model`: requested model id as it appears in the client request.
- `client_api`: client tool API code or alias, such as `codex`,
  `claude-code`, or `gemini-cli`.
- `source`: incoming client API surface code, such as `openai_responses`,
  `openai_compatible`, `anthropic_messages`, or `google_gemini`.
- `upstream`: selected upstream fallback surface using the same surface codes.

The response includes the normalized `client_api`, whether the model catalog is
loaded, the runtime `policy`, the catalog decision target, the runtime
`effective_target`, `needs_plugin`, catalog `plugin_id`, runtime
`effective_plugin_id`, and the matching `route_capability` so clients can see
whether that conversion is ready, reserved, partial, or missing before sending
generation traffic.

## Tool Calling Mapping

The built-in JSON plugins preserve the mainstream tool-calling shapes across
the standard surfaces:

- OpenAI Chat request `tool_calls` and `tool` messages map to OpenAI Responses
  `function_call` and `function_call_output` input items, and back.
- OpenAI Chat response `tool_calls` map to OpenAI Responses `function_call`
  output items; Responses `function_call` output maps back to Chat
  `message.tool_calls` with `finish_reason = "tool_calls"`.
- OpenAI Chat function tools map to Responses `tools[]` entries and back.
- OpenAI Chat `response_format` maps to OpenAI Responses `text.format`, and
  Responses `text.format` maps back to Chat `response_format`.
- OpenAI Chat `tool_choice` maps to OpenAI Responses `tool_choice`,
  including named function choices. `parallel_tool_calls` is preserved between
  the Chat and Responses surfaces.
- Codex-oriented OpenAI Responses `shell` and `local_shell` tools map to a
  Chat function-tool carrier named `shell` or `local_shell`. `shell` uses a
  `commands[]` carrier schema, while `local_shell` uses an exec-style
  `command[]`, `env`, `working_directory`, and `timeout_ms` carrier schema.
  Chat shell function calls map back to Responses `shell_call` or
  `local_shell_call`; tool outputs map back as `shell_call_output` or
  `local_shell_call_output`. Official Responses shell output arrays are
  flattened into readable Chat tool-message content by concatenating
  `stdout`/`stderr` chunks.
- OpenAI Chat `tool_calls` map to Anthropic `tool_use`; Chat `tool` messages
  map to Anthropic `tool_result`.
- Anthropic `tool_use` and `tool_result` map to OpenAI Chat `tool_calls` and
  `tool` messages.
- OpenAI Chat `tool_choice` maps to Anthropic `tool_choice`:
  `auto -> auto`, `none -> none`, `required -> any`, and named function choice
  to Anthropic `{type:"tool", name}`. Anthropic `tool_choice` maps back to the
  equivalent OpenAI Chat value.
- OpenAI Chat `tool_calls` map to Gemini `functionCall`; Chat `tool` messages
  map to Gemini `functionResponse`.
- Gemini `functionCall`, `functionResponse`, and `tools.functionDeclarations`
  map to OpenAI Chat `tool_calls`, `tool` messages, and function tools.
- OpenAI Chat `tool_choice` maps to Gemini
  `toolConfig.functionCallingConfig`: `none -> NONE`, `auto -> AUTO`,
  `required -> ANY`, and named function choice to `ANY` with
  `allowedFunctionNames`. Gemini `NONE`, `AUTO`, and `ANY` modes map back to
  OpenAI Chat `none`, `auto`, `required`, or a named function choice when
  `allowedFunctionNames` contains a single selected function.

These mappings are covered by focused tests in `sdkwork-lr-transform`. The
transformers still intentionally keep provider-specific advanced fields narrow
unless they are part of the shared compatibility surface.

## Streaming Scope

Existing event-level streaming support remains available for OpenAI Chat
Completions with Anthropic Messages and Gemini GenerateContent protocol pairs.
OpenAI Responses streaming is also supported for Codex-to-Claude and
Codex-to-Gemini routing through
`OPENAI_RESPONSES_TO_ANTHROPIC_MESSAGES_API` and
`OPENAI_RESPONSES_TO_GEMINI_GENERATE_CONTENT_API`: Anthropic Messages SSE events
and Gemini `streamGenerateContent` SSE chunks are converted back into OpenAI
Responses events such as `response.created`, `response.in_progress`,
`response.output_text.delta`, `response.function_call_arguments.delta`, and
`response.completed`. Claude Code streaming to OpenAI Responses-native models
and Gemini-native models is supported through
`ANTHROPIC_MESSAGES_TO_OPENAI_RESPONSES_API` and
`ANTHROPIC_MESSAGES_TO_GEMINI_GENERATE_CONTENT_API`, with upstream Responses or
Gemini SSE chunks converted back to Anthropic Messages SSE events. Direct
Gemini/Anthropic streaming conversion is also advertised for
`GEMINI_GENERATE_CONTENT_TO_ANTHROPIC_MESSAGES_API` where event-level
conversion is implemented. Batch work-item expansion remains represented in
the standard but reserved; it does not advertise `capabilities.stream = true`.
Runtime preflight returns HTTP 501 before upstream forwarding when a request
requires streaming through a plugin that does not declare stream support.

Anthropic upstream forwarding always sends an `anthropic-version` header for
Messages API calls. Account-specific `anthropic_version` values take
precedence; when omitted, the router uses the stable Messages API default
`2023-06-01` for both real proxy requests and health probes.
