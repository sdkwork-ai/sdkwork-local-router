import assert from "node:assert/strict";
import type {
  LocalApiProxyApplyProjectionInput,
  LocalApiProxyProviderConfigDraft,
} from "./localApiProxyProviderCenterTypes.ts";

async function runTest(name: string, callback: () => Promise<void> | void) {
  try {
    await callback();
    console.log(`ok - ${name}`);
  } catch (error) {
    console.error(`not ok - ${name}`);
    throw error;
  }
}

await runTest(
  "localApiProxyProviderCenterService saves normalized provider routes and exposes runtime action support",
  async () => {
    const serviceModule = await import("./localApiProxyProviderCenterService.ts");

    const savedInputs: Array<LocalApiProxyProviderConfigDraft & { id?: string }> = [];
    const service = serviceModule.createLocalApiProxyProviderCenterService({
      providerRoutingApi: {
        listProviderRoutingRecords: async () => [],
        saveProviderRoutingRecord: async (input) => {
          savedInputs.push(input);
          return {
            id: "provider-config-openai",
            schemaVersion: 1,
            name: "OpenAI",
            enabled: true,
            isDefault: true,
            managedBy: "user",
            clientProtocol: "openai-compatible",
            upstreamProtocol: "openai-compatible",
            providerId: "openai",
            upstreamBaseUrl: "https://api.openai.com/v1",
            apiKey: "sk-test",
            defaultModelId: "gpt-5.4",
            reasoningModelId: undefined,
            embeddingModelId: undefined,
            models: [{ id: "gpt-5.4", name: "GPT-5.4" }],
            notes: undefined,
            exposeTo: ["sdkwork"],
            baseUrl: "https://api.openai.com/v1",
            config: {
              temperature: 0.2,
              topP: 1,
              maxTokens: 8192,
              timeoutMs: 60000,
              streaming: true,
            },
            createdAt: 1,
            updatedAt: 1,
          };
        },
        deleteProviderRoutingRecord: async () => true,
      },
      runtimeApi: {
        ensureRunning: async () => undefined,
        getInfo: async () =>
          ({
            localAiProxy: {
              lifecycle: "running",
              loopbackOnly: true,
              openaiCompatibleBaseUrl: "http://127.0.0.1:21281/v1",
            },
          }) as any,
      },
      applyApi: {
        listTargets: async () => [
          {
            id: "sdkwork-default",
            name: "Sdkwork",
            kind: "sdkwork",
            isDefault: true,
            writable: true,
            configFile: "C:/Users/admin/.sdkwork/crawstudio/.sdkwork/sdkwork.json",
          },
        ],
      },
    });

    const actionSupport = await service.getActionSupport();
    await service.saveProviderConfig({
      name: " OpenAI ",
      providerId: " api-router-openai ",
      apiKey: " sk-test ",
      baseUrl: " https://api.openai.com/v1/ ",
      defaultModelId: " gpt-5.4 ",
      models: [{ id: " gpt-5.4 ", name: " GPT-5.4 " }],
    } as any);

    assert.equal(actionSupport.quickApply.available, true);
    assert.equal(actionSupport.test.available, true);
    assert.equal(savedInputs.length, 1);
    assert.equal(savedInputs[0]?.providerId, "openai");
    assert.equal(savedInputs[0]?.baseUrl, "https://api.openai.com/v1");
  },
);

await runTest(
  "localApiProxyProviderCenterService applies configs through writable targets without requiring target detail reads",
  async () => {
    const serviceModule = await import("./localApiProxyProviderCenterService.ts");

    const calls: string[] = [];
    const applyCalls: LocalApiProxyApplyProjectionInput[] = [];
    const service = serviceModule.createLocalApiProxyProviderCenterService({
      providerRoutingApi: {
        listProviderRoutingRecords: async () => [],
        saveProviderRoutingRecord: async () => {
          throw new Error("saveProviderRoutingRecord should not be called");
        },
        deleteProviderRoutingRecord: async () => {
          throw new Error("deleteProviderRoutingRecord should not be called");
        },
      },
      runtimeApi: {
        ensureRunning: async () => {
          calls.push("ensureRunning");
          return undefined;
        },
        getInfo: async () =>
          ({
            localAiProxy: {
              lifecycle: "running",
              loopbackOnly: true,
              openaiCompatibleBaseUrl: "http://127.0.0.1:21281/v1",
            },
          }) as any,
      },
      applyApi: {
        listTargets: async () => [
          {
            id: "sdkwork-default",
            name: "Sdkwork",
            kind: "sdkwork",
            isDefault: true,
            writable: true,
            configFile: "C:/Users/admin/.sdkwork/crawstudio/.sdkwork/sdkwork.json",
          },
        ],
        getTarget: async () => {
          throw new Error("getTarget should not be called during apply");
        },
        applyProjection: async (input) => {
          applyCalls.push(input);
        },
      },
    });

    await service.applyProviderConfig({
      targetId: "sdkwork-default",
      config: {
        id: "provider-config-openai",
        schemaVersion: 1,
        name: "OpenAI",
        enabled: true,
        isDefault: true,
        managedBy: "user",
        clientProtocol: "openai-compatible",
        upstreamProtocol: "openai-compatible",
        providerId: "openai",
        upstreamBaseUrl: "https://api.openai.com/v1",
        apiKey: "sk-test",
        defaultModelId: "gpt-5.4",
        reasoningModelId: "o4-mini",
        embeddingModelId: undefined,
        models: [
          { id: "gpt-5.4", name: "GPT-5.4" },
          { id: "o4-mini", name: "o4-mini" },
        ],
        notes: undefined,
        exposeTo: ["sdkwork"],
        baseUrl: "https://api.openai.com/v1",
        config: {
          temperature: 0.2,
          topP: 1,
          maxTokens: 8192,
          timeoutMs: 60000,
          streaming: true,
        },
        createdAt: 1,
        updatedAt: 1,
      },
      agentIds: ["main", "main"],
    });

    assert.deepEqual(calls, ["ensureRunning"]);
    assert.equal(applyCalls.length, 1);
    assert.equal(applyCalls[0]?.targetId, "sdkwork-default");
    assert.equal(
      applyCalls[0]?.configFile,
      "C:/Users/admin/.sdkwork/crawstudio/.sdkwork/sdkwork.json",
    );
    assert.deepEqual(applyCalls[0]?.agentIds, ["main"]);
  },
);
