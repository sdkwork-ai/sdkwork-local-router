import assert from "node:assert/strict";

async function runTest(name: string, callback: () => Promise<void> | void) {
  try {
    await callback();
    console.log(`ok - ${name}`);
  } catch (error) {
    console.error(`not ok - ${name}`);
    throw error;
  }
}

let localApiProxyProviderRouteServiceModule:
  | typeof import("./localApiProxyProviderRouteService.ts")
  | undefined;

try {
  localApiProxyProviderRouteServiceModule = await import(
    "./localApiProxyProviderRouteService.ts"
  );
} catch {
  localApiProxyProviderRouteServiceModule = undefined;
}

await runTest(
  "localApiProxy legacy provider compat exposes the legacy api-router prefix and model-ref normalizer",
  () => {
    assert.ok(
      localApiProxyProviderRouteServiceModule,
      "Expected localApiProxyProviderRouteService.ts to exist",
    );
    assert.equal(
      localApiProxyProviderRouteServiceModule?.LOCAL_API_PROXY_LEGACY_PROVIDER_KEY_PREFIX,
      "api-router-",
    );
    assert.equal(
      typeof localApiProxyProviderRouteServiceModule
        ?.normalizeLocalApiProxyLegacyProviderModelRef,
      "function",
    );
  },
);

await runTest(
  "localApiProxy legacy provider compat normalizes model refs backed by legacy provider ids",
  () => {
    assert.equal(
      localApiProxyProviderRouteServiceModule?.normalizeLocalApiProxyLegacyProviderModelRef(
        " api-router-openai/gpt-4.1 ",
      ),
      "openai/gpt-4.1",
    );
    assert.equal(
      localApiProxyProviderRouteServiceModule?.normalizeLocalApiProxyLegacyProviderModelRef(
        "openai/text-embedding-3-small",
      ),
      "openai/text-embedding-3-small",
    );
    assert.equal(
      localApiProxyProviderRouteServiceModule?.normalizeLocalApiProxyLegacyProviderModelRef(
        "broken-ref",
      ),
      "broken-ref",
    );
  },
);
