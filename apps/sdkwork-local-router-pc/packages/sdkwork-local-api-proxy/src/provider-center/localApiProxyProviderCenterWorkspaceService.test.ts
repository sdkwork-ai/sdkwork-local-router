import assert from "node:assert/strict";
import type { LocalApiProxyProviderConfigDraft } from "./localApiProxyProviderCenterTypes.ts";

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
  "localApiProxyProviderCenterWorkspaceService reuses matching user-managed route ids during provider imports",
  async () => {
    const serviceModule = await import("./localApiProxyProviderCenterWorkspaceService.ts");

    const saveCalls: Array<LocalApiProxyProviderConfigDraft & { id?: string }> = [];
    const workspace = serviceModule.createLocalApiProxyProviderCenterWorkspaceService({
      centerApi: {
        listProviderConfigs: async () => [],
        getActionSupport: async () => ({
          quickApply: { available: true },
          test: { available: true },
        }),
        saveProviderConfig: async (input) => {
          saveCalls.push(input);
          return {
            id: String(input.id || "new-route"),
            schemaVersion: 1,
            name: input.name || "Imported Route",
            enabled: true,
            isDefault: false,
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
          } as any;
        },
        listApplyInstances: async () => [],
        getInstanceApplyTarget: async () => {
          throw new Error("getInstanceApplyTarget should not be called");
        },
        applyProviderConfig: async () => {
          throw new Error("applyProviderConfig should not be called");
        },
      },
      importApi: {
        importProviderConfigs: async () => ({
          source: "codex",
          sourceLabel: "Codex",
          sourcePaths: ["C:/Users/admin/.codex/config.toml"],
          warnings: [],
          drafts: [
            {
              source: "codex",
              sourceLabel: "Codex",
              sourcePaths: ["C:/Users/admin/.codex/config.toml"],
              warnings: [],
              draft: {
                name: "OpenAI Production",
                providerId: "openai",
                apiKey: "sk-imported",
                baseUrl: "https://api.openai.com/v1",
                defaultModelId: "gpt-5.4",
                models: [{ id: "gpt-5.4", name: "GPT-5.4" }],
              },
            },
          ],
        }),
      },
    });

    const result = await workspace.importProviderConfigs({
      source: "codex",
      existingRecords: [
        {
          id: "provider-config-openai-prod",
          schemaVersion: 1,
          name: "OpenAI Production",
          enabled: true,
          isDefault: false,
          managedBy: "user",
          clientProtocol: "openai-compatible",
          upstreamProtocol: "openai-compatible",
          providerId: "openai",
          upstreamBaseUrl: "https://api.openai.com/v1",
          apiKey: "sk-existing",
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
        },
      ],
    });

    assert.equal(saveCalls.length, 1);
    assert.equal(saveCalls[0]?.id, "provider-config-openai-prod");
    assert.equal(result.savedRecordIds[0], "provider-config-openai-prod");
    assert.equal(result.savedNames[0], "OpenAI Production");
  },
);

await runTest(
  "localApiProxyProviderCenterWorkspaceService resolves default agent selections for quick apply targets",
  async () => {
    const serviceModule = await import("./localApiProxyProviderCenterWorkspaceService.ts");

    const workspace = serviceModule.createLocalApiProxyProviderCenterWorkspaceService({
      centerApi: {
        listProviderConfigs: async () => [],
        getActionSupport: async () => ({
          quickApply: { available: true },
          test: { available: true },
        }),
        saveProviderConfig: async () => {
          throw new Error("saveProviderConfig should not be called");
        },
        listApplyInstances: async () => [
          {
            id: "default-instance",
            name: "Default Instance",
            isDefault: true,
          },
          {
            id: "secondary-instance",
            name: "Secondary Instance",
            isDefault: false,
          },
        ],
        getInstanceApplyTarget: async (instanceId) => ({
          instance: {
            id: instanceId,
            name: "Default Instance",
            isDefault: true,
          },
          agents: [
            {
              id: "main",
              name: "Main",
              isDefault: true,
            },
            {
              id: "research",
              name: "Research",
              isDefault: false,
            },
          ],
        }),
        applyProviderConfig: async () => {
          throw new Error("applyProviderConfig should not be called");
        },
      },
      importApi: {
        importProviderConfigs: async () => {
          throw new Error("importProviderConfigs should not be called");
        },
      },
    });

    const applyState = await workspace.loadApplyInstances();
    const targetState = await workspace.loadApplyInstanceTarget(applyState.selectedInstanceId);

    assert.equal(applyState.instances.length, 2);
    assert.equal(applyState.selectedInstanceId, "default-instance");
    assert.equal(targetState.instanceTarget?.instance.id, "default-instance");
    assert.deepEqual(targetState.selectedAgentIds, ["main"]);
  },
);
