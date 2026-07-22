# Source Configuration

`sdkwork.deployment.config.json` is the Local Router source configuration entrypoint. It selects one
typed `<deployment-profile>.<environment>` profile and resolves the corresponding safe environment
file under `topology/`.

Supported source profiles are:

- `standalone.development`
- `standalone.production`
- `cloud.development`
- `cloud.production`

`standalone` profiles bind the application-owned gateway on loopback and expose the same app-api,
backend-api, and open-api contract as cloud. `cloud` profiles point clients at the deployed SDKWork
platform ingress and do not start remote services locally. The provider-compatible Local Router
paths remain `/local-router/v1`, `/local-router/anthropic`, and `/local-router/google` in every
profile.

The profile index and environment files are the source-controlled configuration authority. The
schemas and invariants are defined by `../specs/component.spec.json`, `SOURCE_CONFIG_SPEC.md`,
`CONFIG_SPEC.md`, and `ENVIRONMENT_SPEC.md`. Runtime materialization selects exactly one profile;
process environment variables and command-line values are explicit overrides only.

Committed files contain no client API keys, upstream provider credentials, tokens, passwords,
private keys, or machine-specific absolute paths. Inject secrets through the platform secret
manager, protected secret files, or process environment. Keep private overrides in ignored
`*.local.*` files; never commit them.

Installed server configuration is materialized under the operating-system path defined by
`RUNTIME_DIRECTORY_SPEC.md`. Browser-visible materialization contains safe public URLs only and no
secrets.

Validate this source configuration from the repository root:

```powershell
node ..\sdkwork-specs\tools\check-source-config-standard.mjs --root .
```
