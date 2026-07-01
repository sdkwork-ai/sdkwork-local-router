import {
  LOCAL_AI_PROXY_DEFAULT_CLIENT_API_KEY,
  LOCAL_AI_PROXY_DEFAULT_UPSTREAM_BASE_URL,
  inferLocalAiProxyClientProtocol,
  inferLocalAiProxyUpstreamProtocol,
} from "../services/localApiProxyProviderRouteService.ts";
import {
  createDefaultLocalApiProxyProviderRuntimeConfig,
} from "../services/localApiProxyProviderRuntimeConfigService.ts";
import {
  listKnownLocalApiProxyProviderRoutingChannels,
  normalizeLocalApiProxyProviderRoutingDraft,
} from "../services/localApiProxyProviderRoutingCatalogService.ts";
import type { LocalApiProxyProviderConfigDraft, LocalApiProxyProviderConfigPreset } from "./localApiProxyProviderCenterTypes.ts";

function createPresetDraft(
  input: LocalApiProxyProviderConfigDraft,
): LocalApiProxyProviderConfigDraft {
  return normalizeLocalApiProxyProviderRoutingDraft({
    ...input,
    clientProtocol:
      input.clientProtocol || inferLocalAiProxyClientProtocol(input.providerId),
    enabled: true,
    isDefault: false,
    managedBy: "user",
    exposeTo: input.exposeTo || ["sdkwork"],
    config: createDefaultLocalApiProxyProviderRuntimeConfig(),
  });
}

function createTemplatePreset(
  channel: ReturnType<typeof listKnownLocalApiProxyProviderRoutingChannels>[number],
): LocalApiProxyProviderConfigPreset {
  return {
    id: channel.id,
    label: channel.name,
    description: `${channel.description} Fill in model ids for this provider or keep the SDKWork upstream fallback.`,
    draft: createPresetDraft({
      presetId: channel.id,
      name: channel.name,
      providerId: channel.id,
      upstreamProtocol: inferLocalAiProxyUpstreamProtocol(channel.id),
      upstreamBaseUrl: LOCAL_AI_PROXY_DEFAULT_UPSTREAM_BASE_URL,
      apiKey: "",
      defaultModelId: "",
      reasoningModelId: undefined,
      embeddingModelId: undefined,
      models: [],
    }),
  };
}

function createCuratedPresets(): LocalApiProxyProviderConfigPreset[] {
  return [
    {
      id: "sdkwork",
      label: "SDKWork",
      description:
        "SDKWork universal gateway preset with OpenAI, Gemini, and Claude Code compatible APIs across mainstream global and China model families.",
      draft: createPresetDraft({
        presetId: "sdkwork",
        name: "SDKWork",
        providerId: "sdkwork",
        upstreamProtocol: "sdkwork",
        upstreamBaseUrl: LOCAL_AI_PROXY_DEFAULT_UPSTREAM_BASE_URL,
        apiKey: LOCAL_AI_PROXY_DEFAULT_CLIENT_API_KEY,
        defaultModelId: "gpt-5.4",
        reasoningModelId: "o4-mini",
        embeddingModelId: "text-embedding-3-large",
        models: [
          { id: "gpt-5.4", name: "GPT-5.4" },
          { id: "o4-mini", name: "o4-mini" },
          { id: "claude-sonnet-4-20250514", name: "Claude Sonnet 4" },
          { id: "claude-opus-4-20250514", name: "Claude Opus 4" },
          { id: "gemini-2.5-pro", name: "Gemini 2.5 Pro" },
          { id: "gemini-2.5-flash", name: "Gemini 2.5 Flash" },
          { id: "deepseek-chat", name: "DeepSeek Chat" },
          { id: "deepseek-reasoner", name: "DeepSeek Reasoner" },
          { id: "glm-5.1", name: "GLM-5.1" },
          { id: "glm-5v-turbo", name: "GLM-5V Turbo" },
          { id: "qwen-max", name: "Qwen Max" },
          { id: "qwq-plus", name: "QwQ Plus" },
          { id: "minimax-m1", name: "MiniMax M1" },
          { id: "kimi-k2", name: "Moonshot Kimi K2" },
          { id: "text-embedding-3-large", name: "text-embedding-3-large" },
        ],
      }),
    },
    {
      id: "openai",
      label: "OpenAI",
      description: "OpenAI-compatible route with GPT-5 and embedding defaults.",
      draft: createPresetDraft({
        presetId: "openai",
        name: "OpenAI",
        providerId: "openai",
        upstreamProtocol: "openai-compatible",
        upstreamBaseUrl: "https://api.openai.com/v1",
        apiKey: "",
        defaultModelId: "gpt-5.4",
        reasoningModelId: "o4-mini",
        embeddingModelId: "text-embedding-3-large",
        models: [
          { id: "gpt-5.4", name: "GPT-5.4" },
          { id: "gpt-5.4-mini", name: "GPT-5.4 mini" },
          { id: "o4-mini", name: "o4-mini" },
          { id: "text-embedding-3-large", name: "text-embedding-3-large" },
        ],
      }),
    },
    {
      id: "anthropic",
      label: "Anthropic",
      description: "Anthropic Claude route preset.",
      draft: createPresetDraft({
        presetId: "anthropic",
        name: "Anthropic",
        providerId: "anthropic",
        upstreamProtocol: "anthropic",
        upstreamBaseUrl: "https://api.anthropic.com/v1",
        apiKey: "",
        defaultModelId: "claude-sonnet-4-20250514",
        reasoningModelId: "claude-opus-4-20250514",
        models: [
          { id: "claude-sonnet-4-20250514", name: "Claude Sonnet 4" },
          { id: "claude-opus-4-20250514", name: "Claude Opus 4" },
          { id: "claude-3-7-sonnet-latest", name: "Claude 3.7 Sonnet" },
        ],
      }),
    },
    {
      id: "cloudflare-ai-gateway",
      label: "Cloudflare AI Gateway",
      description:
        "Cloudflare AI Gateway Anthropic-compatible preset with the official gateway URL shape and Claude Sonnet 4.5 starter model.",
      draft: createPresetDraft({
        presetId: "cloudflare-ai-gateway",
        name: "Cloudflare AI Gateway",
        providerId: "cloudflare-ai-gateway",
        clientProtocol: "anthropic",
        upstreamProtocol: "anthropic",
        upstreamBaseUrl: "https://gateway.ai.cloudflare.com/v1/<account_id>/<gateway_id>/anthropic",
        apiKey: "",
        defaultModelId: "cloudflare-ai-gateway/claude-sonnet-4-5",
        reasoningModelId: "cloudflare-ai-gateway/claude-sonnet-4-5",
        models: [
          {
            id: "cloudflare-ai-gateway/claude-sonnet-4-5",
            name: "Claude Sonnet 4.5 (Cloudflare AI Gateway)",
          },
        ],
      }),
    },
    {
      id: "gemini",
      label: "Gemini",
      description: "Google Gemini native route preset.",
      draft: createPresetDraft({
        presetId: "gemini",
        name: "Gemini",
        providerId: "google",
        upstreamProtocol: "gemini",
        upstreamBaseUrl: "https://generativelanguage.googleapis.com",
        apiKey: "",
        defaultModelId: "gemini-2.5-pro",
        reasoningModelId: "gemini-2.5-pro",
        embeddingModelId: "text-embedding-004",
        models: [
          { id: "gemini-2.5-pro", name: "Gemini 2.5 Pro" },
          { id: "gemini-2.5-flash", name: "Gemini 2.5 Flash" },
          { id: "text-embedding-004", name: "text-embedding-004" },
        ],
      }),
    },
    {
      id: "xai",
      label: "xAI",
      description: "xAI Grok route preset.",
      draft: createPresetDraft({
        presetId: "xai",
        name: "xAI",
        providerId: "xai",
        upstreamProtocol: "openai-compatible",
        upstreamBaseUrl: "https://api.x.ai/v1",
        apiKey: "",
        defaultModelId: "grok-4",
        reasoningModelId: "grok-4",
        models: [
          { id: "grok-4", name: "Grok 4" },
          { id: "grok-4-fast-reasoning", name: "Grok 4 Fast Reasoning" },
        ],
      }),
    },
    {
      id: "groq",
      label: "Groq",
      description: "Groq OpenAI-compatible route preset with the official Llama 3.3 starter model.",
      draft: createPresetDraft({
        presetId: "groq",
        name: "Groq",
        providerId: "groq",
        upstreamProtocol: "openai-compatible",
        upstreamBaseUrl: "https://api.groq.com/openai/v1",
        apiKey: "",
        defaultModelId: "llama-3.3-70b-versatile",
        models: [{ id: "llama-3.3-70b-versatile", name: "Llama 3.3 70B Versatile" }],
      }),
    },
    {
      id: "ollama",
      label: "Ollama",
      description: "Ollama native local-runtime preset aligned with localhost deployment default.",
      draft: createPresetDraft({
        presetId: "ollama",
        name: "Ollama",
        providerId: "ollama",
        upstreamProtocol: "ollama",
        upstreamBaseUrl: "http://127.0.0.1:11434",
        apiKey: "ollama-local",
        defaultModelId: "glm-4.7-flash",
        models: [
          { id: "glm-4.7-flash", name: "GLM 4.7 Flash" },
          { id: "gpt-oss:20b", name: "GPT-OSS 20B" },
          { id: "nomic-embed-text", name: "nomic-embed-text" },
        ],
      }),
    },
    {
      id: "deepseek",
      label: "DeepSeek",
      description: "DeepSeek chat and reasoning route preset.",
      draft: createPresetDraft({
        presetId: "deepseek",
        name: "DeepSeek",
        providerId: "deepseek",
        upstreamProtocol: "openai-compatible",
        upstreamBaseUrl: "https://api.deepseek.com/v1",
        apiKey: "",
        defaultModelId: "deepseek-chat",
        reasoningModelId: "deepseek-reasoner",
        models: [
          { id: "deepseek-chat", name: "DeepSeek Chat" },
          { id: "deepseek-reasoner", name: "DeepSeek Reasoner" },
        ],
      }),
    },
    {
      id: "qwen",
      label: "Qwen",
      description: "Qwen OpenAI-compatible route preset.",
      draft: createPresetDraft({
        presetId: "qwen",
        name: "Qwen",
        providerId: "qwen",
        upstreamProtocol: "openai-compatible",
        upstreamBaseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        apiKey: "",
        defaultModelId: "qwen-max",
        reasoningModelId: "qwq-plus",
        embeddingModelId: "text-embedding-v4",
        models: [
          { id: "qwen-max", name: "Qwen Max" },
          { id: "qwq-plus", name: "QwQ Plus" },
          { id: "text-embedding-v4", name: "Text Embedding V4" },
        ],
      }),
    },
    {
      id: "azure-openai",
      label: "Azure OpenAI",
      description: "Azure OpenAI v1 route preset for resource-scoped deployments.",
      draft: createPresetDraft({
        presetId: "azure-openai",
        name: "Azure OpenAI",
        providerId: "azure-openai",
        upstreamProtocol: "azure-openai",
        upstreamBaseUrl: "https://YOUR-RESOURCE-NAME.openai.azure.com",
        apiKey: "",
        defaultModelId: "gpt-4.1",
        reasoningModelId: "gpt-4.1",
        embeddingModelId: "text-embedding-3-large",
        models: [
          { id: "gpt-4.1", name: "GPT-4.1" },
          { id: "gpt-4.1-mini", name: "GPT-4.1 Mini" },
          { id: "text-embedding-3-large", name: "text-embedding-3-large" },
        ],
      }),
    },
    {
      id: "openrouter",
      label: "OpenRouter",
      description: "OpenRouter OpenAI-compatible route preset.",
      draft: createPresetDraft({
        presetId: "openrouter",
        name: "OpenRouter",
        providerId: "openrouter",
        upstreamProtocol: "openrouter",
        upstreamBaseUrl: "https://openrouter.ai/api/v1",
        apiKey: "",
        defaultModelId: "openai/gpt-4o",
        reasoningModelId: "anthropic/claude-3.7-sonnet",
        models: [
          { id: "openai/gpt-4o", name: "OpenAI GPT-4o" },
          { id: "anthropic/claude-3.7-sonnet", name: "Anthropic Claude 3.7 Sonnet" },
          { id: "google/gemini-2.5-pro", name: "Google Gemini 2.5 Pro" },
        ],
      }),
    },
    {
      id: "vercel-ai-gateway",
      label: "Vercel AI Gateway",
      description: "Vercel AI Gateway Anthropic-compatible preset.",
      draft: createPresetDraft({
        presetId: "vercel-ai-gateway",
        name: "Vercel AI Gateway",
        providerId: "vercel-ai-gateway",
        clientProtocol: "anthropic",
        upstreamProtocol: "anthropic",
        upstreamBaseUrl: "https://ai-gateway.vercel.sh",
        apiKey: "",
        defaultModelId: "anthropic/claude-opus-4.6",
        reasoningModelId: "anthropic/claude-opus-4.6",
        models: [
          { id: "anthropic/claude-opus-4.6", name: "Anthropic Claude Opus 4.6" },
          { id: "openai/gpt-5.4", name: "OpenAI GPT-5.4" },
          { id: "xai/grok-4-fast-reasoning", name: "xAI Grok 4 Fast Reasoning" },
        ],
      }),
    },
    {
      id: "litellm",
      label: "LiteLLM",
      description: "LiteLLM gateway preset aligned with the official local proxy example.",
      draft: createPresetDraft({
        presetId: "litellm",
        name: "LiteLLM",
        providerId: "litellm",
        upstreamProtocol: "openai-compatible",
        upstreamBaseUrl: "http://localhost:4000",
        apiKey: "",
        defaultModelId: "claude-opus-4-6",
        models: [
          { id: "claude-opus-4-6", name: "Claude Opus 4.6" },
          { id: "gpt-4o", name: "GPT-4o" },
        ],
      }),
    },
    {
      id: "together",
      label: "Together",
      description: "Together AI OpenAI-compatible route preset.",
      draft: createPresetDraft({
        presetId: "together",
        name: "Together",
        providerId: "together",
        upstreamProtocol: "openai-compatible",
        upstreamBaseUrl: "https://api.together.xyz/v1",
        apiKey: "",
        defaultModelId: "moonshotai/Kimi-K2.5",
        models: [
          { id: "moonshotai/Kimi-K2.5", name: "Kimi K2.5" },
          { id: "zai-org/GLM-4.7", name: "GLM-4.7" },
        ],
      }),
    },
    {
      id: "fireworks",
      label: "Fireworks",
      description: "Fireworks OpenAI-compatible preset.",
      draft: createPresetDraft({
        presetId: "fireworks",
        name: "Fireworks",
        providerId: "fireworks",
        upstreamProtocol: "openai-compatible",
        upstreamBaseUrl: "https://api.fireworks.ai/inference/v1",
        apiKey: "",
        defaultModelId: "fireworks/accounts/fireworks/routers/kimi-k2p5-turbo",
        models: [
          {
            id: "fireworks/accounts/fireworks/routers/kimi-k2p5-turbo",
            name: "Kimi K2.5 Turbo (Fire Pass)",
          },
        ],
      }),
    },
    {
      id: "amazon-bedrock-mantle",
      label: "Amazon Bedrock Mantle",
      description: "Amazon Bedrock Mantle OpenAI-compatible preset.",
      draft: createPresetDraft({
        presetId: "amazon-bedrock-mantle",
        name: "Amazon Bedrock Mantle",
        providerId: "amazon-bedrock-mantle",
        upstreamProtocol: "openai-compatible",
        upstreamBaseUrl: "https://bedrock-mantle.us-east-1.api.aws/v1",
        apiKey: "",
        defaultModelId: "gpt-oss-120b",
        reasoningModelId: "gpt-oss-120b",
        models: [{ id: "gpt-oss-120b", name: "GPT-OSS 120B" }],
      }),
    },
    {
      id: "kilocode",
      label: "Kilo Code",
      description: "Kilo Gateway OpenAI-compatible preset.",
      draft: createPresetDraft({
        presetId: "kilocode",
        name: "Kilo Code",
        providerId: "kilocode",
        upstreamProtocol: "openai-compatible",
        upstreamBaseUrl: "https://api.kilo.ai/api/gateway/",
        apiKey: "",
        defaultModelId: "kilo/auto",
        models: [{ id: "kilo/auto", name: "Kilo Auto" }],
      }),
    },
    {
      id: "venice",
      label: "Venice",
      description: "Venice AI OpenAI-compatible preset.",
      draft: createPresetDraft({
        presetId: "venice",
        name: "Venice",
        providerId: "venice",
        upstreamProtocol: "openai-compatible",
        upstreamBaseUrl: "https://api.venice.ai/api/v1",
        apiKey: "",
        defaultModelId: "kimi-k2-5",
        models: [{ id: "kimi-k2-5", name: "Kimi K2.5" }],
      }),
    },
    {
      id: "vllm",
      label: "vLLM",
      description: "vLLM local OpenAI-compatible preset.",
      draft: createPresetDraft({
        presetId: "vllm",
        name: "vLLM",
        providerId: "vllm",
        upstreamProtocol: "openai-compatible",
        upstreamBaseUrl: "http://127.0.0.1:8000/v1",
        apiKey: "vllm-local",
        defaultModelId: "",
        models: [],
      }),
    },
    {
      id: "sglang",
      label: "SGLang",
      description: "SGLang local OpenAI-compatible preset.",
      draft: createPresetDraft({
        presetId: "sglang",
        name: "SGLang",
        providerId: "sglang",
        upstreamProtocol: "openai-compatible",
        upstreamBaseUrl: "http://127.0.0.1:30000/v1",
        apiKey: "sglang-local",
        defaultModelId: "",
        models: [],
      }),
    },
    {
      id: "zhipu",
      label: "Z.AI",
      description: "Z.AI GLM route preset.",
      draft: createPresetDraft({
        presetId: "zhipu",
        name: "Z.AI",
        providerId: "zhipu",
        upstreamProtocol: "openai-compatible",
        upstreamBaseUrl: "https://open.bigmodel.cn/api/paas/v4",
        apiKey: "",
        defaultModelId: "glm-5.1",
        reasoningModelId: "glm-5.1",
        models: [
          { id: "glm-5.1", name: "GLM-5.1" },
          { id: "glm-5v-turbo", name: "GLM-5V Turbo" },
        ],
      }),
    },
  ];
}

export function listLocalApiProxyProviderConfigPresets() {
  const curatedPresets = createCuratedPresets();
  const coveredProviderIds = new Set(
    curatedPresets.flatMap((preset) => [preset.id, preset.draft.providerId]),
  );
  const templatePresets = listKnownLocalApiProxyProviderRoutingChannels()
    .filter((channel) => !coveredProviderIds.has(channel.id))
    .map(createTemplatePreset)
    .sort((left, right) => left.label.localeCompare(right.label));

  return [...curatedPresets, ...templatePresets];
}
