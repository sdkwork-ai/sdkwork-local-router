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

let localApiProxyProjectedProviderModelServiceModule:
  | typeof import("./localApiProxyProjectedProviderModelService.ts")
  | undefined;

try {
  localApiProxyProjectedProviderModelServiceModule = await import(
    "./localApiProxyProjectedProviderModelService.ts"
  );
} catch {
  localApiProxyProjectedProviderModelServiceModule = undefined;
}

await runTest(
  "localApiProxy projected provider model service exposes the projected catalog resolver",
  () => {
    assert.ok(
      localApiProxyProjectedProviderModelServiceModule,
      "Expected localApiProxyProjectedProviderModelService.ts to exist",
    );
    assert.equal(
      typeof localApiProxyProjectedProviderModelServiceModule
        ?.resolveLocalApiProxyProjectedProviderModelCatalog,
      "function",
    );
  },
);

await runTest(
  "localApiProxy projected provider model service merges existing models with explicit default and reasoning selections",
  () => {
    const result =
      localApiProxyProjectedProviderModelServiceModule?.resolveLocalApiProxyProjectedProviderModelCatalog(
        {
          existingModels: [
            {
              id: "gpt-4.1",
              name: "GPT-4.1",
              role: "fallback",
            },
            {
              id: "legacy-fallback",
              name: "Legacy Fallback",
              role: "fallback",
            },
          ],
          selection: {
            defaultModelId: "gpt-5.4",
            reasoningModelId: "o4-mini",
          },
        },
      );

    assert.deepEqual(result, {
      selection: {
        defaultModelId: "gpt-5.4",
        reasoningModelId: "o4-mini",
        embeddingModelId: undefined,
      },
      models: [
        {
          id: "gpt-5.4",
          name: "gpt-5.4",
          role: "default",
          streaming: true,
        },
        {
          id: "o4-mini",
          name: "o4-mini",
          role: "reasoning",
          streaming: true,
        },
        {
          id: "gpt-4.1",
          name: "GPT-4.1",
          role: "fallback",
          streaming: true,
        },
        {
          id: "legacy-fallback",
          name: "Legacy Fallback",
          role: "fallback",
          streaming: true,
        },
      ],
    });
  },
);

await runTest(
  "localApiProxy projected provider model service preserves existing embedding metadata when selection omits embedding",
  () => {
    const result =
      localApiProxyProjectedProviderModelServiceModule?.resolveLocalApiProxyProjectedProviderModelCatalog(
        {
          existingModels: [
            {
              id: "gpt-4.1",
              name: "GPT-4.1",
              role: "fallback",
            },
            {
              id: "text-embedding-3-small",
              name: "text-embedding-3-small",
              role: "embedding",
            },
          ],
          selection: {
            defaultModelId: "gpt-4.1",
          },
        },
      );

    assert.deepEqual(result?.models, [
      {
        id: "gpt-4.1",
        name: "GPT-4.1",
        role: "default",
        streaming: true,
      },
      {
        id: "text-embedding-3-small",
        name: "text-embedding-3-small",
        role: "embedding",
        streaming: false,
      },
    ]);
  },
);
