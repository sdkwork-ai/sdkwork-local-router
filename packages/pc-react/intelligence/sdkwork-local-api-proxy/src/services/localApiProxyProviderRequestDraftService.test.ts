import assert from "node:assert/strict";
import type { LocalApiProxyRequestOverridesRecord } from "../types/localApiProxyProviderRuntimeConfig.ts";

async function runTest(name: string, callback: () => Promise<void> | void) {
  try {
    await callback();
    console.log(`ok - ${name}`);
  } catch (error) {
    console.error(`not ok - ${name}`);
    throw error;
  }
}

let localApiProxyProviderRequestDraftServiceModule:
  | typeof import("./localApiProxyProviderRequestDraftService.ts")
  | undefined;

try {
  localApiProxyProviderRequestDraftServiceModule = await import(
    "./localApiProxyProviderRequestDraftService.ts"
  );
} catch {
  localApiProxyProviderRequestDraftServiceModule = undefined;
}

await runTest(
  "localApiProxy provider request draft service exposes shared draft format and parse helpers",
  () => {
    assert.ok(
      localApiProxyProviderRequestDraftServiceModule,
      "Expected localApiProxyProviderRequestDraftService.ts to exist",
    );
    assert.equal(
      typeof localApiProxyProviderRequestDraftServiceModule
        ?.formatLocalApiProxyProviderRequestOverridesDraft,
      "function",
    );
    assert.equal(
      typeof localApiProxyProviderRequestDraftServiceModule
        ?.parseLocalApiProxyProviderRequestOverridesDraft,
      "function",
    );
  },
);

await runTest(
  "localApiProxy provider request draft service formats and parses JSON5 request overrides",
  () => {
    const formatted =
      localApiProxyProviderRequestDraftServiceModule?.formatLocalApiProxyProviderRequestOverridesDraft(
        {
          headers: {
            "OpenAI-Organization": "org_live",
          },
          auth: {
            mode: "authorization-bearer",
            token: "${OPENAI_API_KEY}",
          },
        },
      ) || "";

    assert.match(formatted, /OpenAI-Organization/);
    assert.match(formatted, /authorization-bearer/);

    const parsed =
      localApiProxyProviderRequestDraftServiceModule?.parseLocalApiProxyProviderRequestOverridesDraft(
        `{
  headers: {
    "OpenAI-Organization": " org_live ",
  },
  auth: {
    mode: "authorization-bearer",
    token: "\${OPENAI_API_KEY}",
  },
  proxy: {
    mode: "explicit-proxy",
    url: "http://127.0.0.1:8080",
  },
  tls: {
    insecureSkipVerify: true,
    serverName: "api.openai.internal",
  },
}`,
      );

    assert.deepEqual(parsed, {
      headers: {
        "OpenAI-Organization": "org_live",
      },
      auth: {
        mode: "authorization-bearer",
        token: "${OPENAI_API_KEY}",
      },
      proxy: {
        mode: "explicit-proxy",
        url: "http://127.0.0.1:8080",
      },
      tls: {
        insecureSkipVerify: true,
        serverName: "api.openai.internal",
      },
    } satisfies LocalApiProxyRequestOverridesRecord);
  },
);

await runTest(
  "localApiProxy provider request draft service rejects unsupported auth modes",
  () => {
    assert.throws(
      () =>
        localApiProxyProviderRequestDraftServiceModule?.parseLocalApiProxyProviderRequestOverridesDraft(
          `{
  auth: {
    mode: "basic",
  },
}`,
        ),
      /auth\.mode/i,
    );
  },
);
