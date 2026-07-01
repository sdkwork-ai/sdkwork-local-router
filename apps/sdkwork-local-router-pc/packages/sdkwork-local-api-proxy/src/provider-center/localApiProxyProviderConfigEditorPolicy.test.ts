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
  "localApiProxyProviderConfigEditorPolicy normalizes provider drafts through shared local-api-proxy standards",
  async () => {
    const policyModule = await import("./localApiProxyProviderConfigEditorPolicy.ts");

    const draft = policyModule.createLocalApiProxyProviderConfigDraftFromForm({
      ...policyModule.createLocalApiProxyProviderConfigFormState(),
      providerId: " api-router-openai ",
      name: " OpenAI Route ",
      baseUrl: " https://api.openai.com/v1/ ",
      apiKey: " sk-test ",
      defaultModelId: " gpt-5.4 ",
      modelsText: " gpt-5.4 = GPT-5.4 ",
    });

    assert.equal(draft.providerId, "openai");
    assert.equal(draft.baseUrl, "https://api.openai.com/v1");
    assert.equal(draft.defaultModelId, "gpt-5.4");
    assert.deepEqual(draft.models, [{ id: "gpt-5.4", name: "GPT-5.4" }]);
  },
);

await runTest(
  "localApiProxyProviderConfigEditorPolicy exposes reusable provider-center view helpers",
  async () => {
    const policyModule = await import("./localApiProxyProviderConfigEditorPolicy.ts");

    const form = policyModule.createLocalApiProxyProviderConfigFormState({
      providerId: "openai",
      defaultModelId: "gpt-5.4",
      reasoningModelId: "o4-mini",
      embeddingModelId: "text-embedding-3-large",
      models: [
        { id: "gpt-5.4", name: "GPT-5.4" },
        { id: "o4-mini", name: "o4-mini" },
        { id: "text-embedding-3-large", name: "Embeddings" },
        { id: "gpt-5.4", name: "Duplicate GPT-5.4" },
      ],
    });
    const knownOptions = policyModule.listLocalApiProxyProviderConfigKnownProviderOptions([
      {
        id: "openai",
        label: "OpenAI",
        description: "OpenAI preset",
        draft: {
          name: "OpenAI",
          providerId: "openai",
          baseUrl: "https://api.openai.com/v1",
          apiKey: "",
          defaultModelId: "gpt-5.4",
          models: [{ id: "gpt-5.4", name: "GPT-5.4" }],
        },
      },
    ]);

    assert.equal(
      policyModule.createLocalApiProxyProviderConfigBadgeLabel("Azure OpenAI", ""),
      "AO",
    );
    assert.equal(
      policyModule.createLocalApiProxyProviderConfigBadgeLabel("", "openai"),
      "OP",
    );
    assert.equal(
      policyModule.matchLocalApiProxyProviderConfigKnownProviderSearch(
        "claude",
        knownOptions[0]!,
      ),
      false,
    );
    assert.equal(
      policyModule.matchLocalApiProxyProviderConfigKnownProviderSearch(
        "openai",
        knownOptions[0]!,
      ),
      true,
    );
    assert.equal(
      policyModule.matchLocalApiProxyProviderConfigCustomRouteSearch(
        "custom",
        "Custom Route",
        "User managed route",
      ),
      true,
    );
    assert.deepEqual(
      policyModule.listLocalApiProxyProviderConfigModelSelectionOptions(form),
      [
        { id: "gpt-5.4", label: "Duplicate GPT-5.4" },
        { id: "o4-mini", label: "o4-mini" },
        { id: "text-embedding-3-large", label: "Embeddings" },
      ],
    );
    assert.deepEqual(
      policyModule.listLocalApiProxyProviderConfigModelRoles(form, "gpt-5.4", {
        defaultModel: "Default",
        reasoningModel: "Reasoning",
        embeddingModel: "Embedding",
      }),
      ["Default"],
    );
    assert.deepEqual(
      policyModule.listLocalApiProxyProviderConfigModelRoles(form, "o4-mini", {
        defaultModel: "Default",
        reasoningModel: "Reasoning",
        embeddingModel: "Embedding",
      }),
      ["Reasoning"],
    );
    assert.deepEqual(
      policyModule.listLocalApiProxyProviderConfigModelRoles(form, "text-embedding-3-large", {
        defaultModel: "Default",
        reasoningModel: "Reasoning",
        embeddingModel: "Embedding",
      }),
      ["Embedding"],
    );
  },
);
