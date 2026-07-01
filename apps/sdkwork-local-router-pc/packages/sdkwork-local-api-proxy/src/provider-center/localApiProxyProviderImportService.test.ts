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

await runTest(
  "localApiProxyProviderImportService imports Codex tooling config through injected user-tooling ports",
  async () => {
    const serviceModule = await import("./localApiProxyProviderImportService.ts");

    const service = serviceModule.createLocalApiProxyProviderImportService({
      platformApi: {
        getPlatform: () => "desktop",
        pathExistsForUserTooling: async (filePath: string) =>
          filePath.replaceAll("\\", "/") === "C:/Users/admin/.codex/config.toml"
          || filePath.replaceAll("\\", "/") === "C:/Users/admin/.codex/auth.json",
        readFileForUserTooling: async (filePath: string) => {
          const normalizedPath = filePath.replaceAll("\\", "/");
          if (normalizedPath === "C:/Users/admin/.codex/config.toml") {
            return `
model = "gpt-5.4"
model_provider = "openai"

[model_providers.openai]
base_url = "https://api.openai.com/v1"
env_key = "OPENAI_API_KEY"
`;
          }

          return JSON.stringify({
            OPENAI_API_KEY: "sk-codex-openai",
          });
        },
      },
      runtimeApi: {
        getRuntimeInfo: async () =>
          ({
            platform: "desktop",
            paths: {
              userRoot: "C:/Users/admin/.sdkwork/crawstudio",
            },
          }) as any,
      },
    });

    const imported = await service.importProviderConfigs("codex");

    assert.equal(imported.drafts.length, 1);
    assert.equal(imported.drafts[0]?.draft.providerId, "openai");
    assert.equal(imported.drafts[0]?.draft.apiKey, "sk-codex-openai");
  },
);
