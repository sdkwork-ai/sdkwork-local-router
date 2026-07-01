import type {
  LocalApiProxyConfiguredProvider,
  LocalApiProxyProviderChannel,
  LocalApiProxyProviderChannelDefinition,
  LocalApiProxyProviderRoutingDraft,
  LocalApiProxyProviderRoutingDraftInput,
  LocalApiProxyProviderRoutingRecord,
} from "../types/localApiProxyProviderCatalog.ts";
import type {
  LocalApiProxyProviderRouteRecord,
  LocalApiProxyProviderRouteClientProtocol,
} from "../types/localApiProxyProviderRoute.ts";
import type { LocalApiProxyProviderRuntimeConfig } from "../types/localApiProxyProviderRuntimeConfig.ts";
import {
  LOCAL_AI_PROXY_DEFAULT_UPSTREAM_BASE_URL,
  normalizeLocalApiProxyProviderKey,
  normalizeProviderLocalApiProxyRouteRecord,
  normalizeProviderLocalApiProxyRouteRecords,
} from "./localApiProxyProviderRouteService.ts";
import {
  normalizeLocalApiProxyProviderRuntimeConfig,
} from "./localApiProxyProviderRuntimeConfigService.ts";

export const LOCAL_API_PROXY_PROVIDER_ROUTE_STORAGE_NAMESPACE =
  "lap.provider-routes";
const DEFAULT_SQLITE_PROFILE_ID = "default-sqlite";
const PROVIDER_CONFIG_ID_PREFIX = "provider-config-";

export interface LocalApiProxyProviderRoutingCatalogService {
  listProviderRoutingRecords(): Promise<LocalApiProxyProviderRoutingRecord[]>;
  saveProviderRoutingRecord(
    input: LocalApiProxyProviderRoutingDraft & { id?: string },
  ): Promise<LocalApiProxyProviderRoutingRecord>;
  deleteProviderRoutingRecord(id: string): Promise<boolean>;
  listConfiguredProviders(): Promise<LocalApiProxyConfiguredProvider[]>;
  listProviderChannels(): Promise<LocalApiProxyProviderChannel[]>;
}

export interface LocalApiProxyProviderRoutingStorageProfileRecord {
  id: string;
  provider: string;
  active?: boolean;
  readOnly?: boolean;
}

export interface LocalApiProxyProviderRoutingStorageInfo {
  activeProfileId?: string | null;
  profiles: LocalApiProxyProviderRoutingStorageProfileRecord[];
}

export interface LocalApiProxyProviderRoutingStorageApi {
  getStorageInfo(): Promise<LocalApiProxyProviderRoutingStorageInfo | null | undefined>;
  listKeys(input: {
    profileId: string;
    namespace: string;
  }): Promise<{ keys: string[] }>;
  getText(input: {
    profileId: string;
    namespace: string;
    key: string;
  }): Promise<{ value: string | null }>;
  putText(input: {
    profileId: string;
    namespace: string;
    key: string;
    value: string;
  }): Promise<unknown>;
  delete(input: {
    profileId: string;
    namespace: string;
    key: string;
  }): Promise<unknown>;
}

export interface CreateLocalApiProxyProviderRoutingCatalogServiceOptions {
  storageApi: LocalApiProxyProviderRoutingStorageApi;
  now?: () => number;
  storageNamespace?: string;
}

const providerChannelCatalog: LocalApiProxyProviderChannelDefinition[] = [
  {
    id: "sdkwork",
    name: "SDKWork",
    vendor: "SDKWork",
    description:
      "Universal AI gateway with OpenAI, Gemini, and Claude Code compatible APIs across leading global and China model providers.",
    modelFamily: "GPT / Claude / Gemini / DeepSeek",
  },
  {
    id: "openai",
    name: "OpenAI",
    vendor: "OpenAI",
    description:
      "GPT family routing, enterprise gateway connectivity, and ecosystem compatibility.",
    modelFamily: "GPT-4.1 / o-series",
  },
  {
    id: "anthropic",
    name: "Anthropic",
    vendor: "Anthropic",
    description:
      "Claude routing for long-context, safe completion, and tool orchestration workloads.",
    modelFamily: "Claude 3.7 / 4",
  },
  {
    id: "cloudflare-ai-gateway",
    name: "Cloudflare AI Gateway",
    vendor: "Cloudflare",
    description:
      "Anthropic-compatible Cloudflare AI Gateway routing with optional gateway-auth headers, caching, and provider analytics in front of Claude traffic.",
    modelFamily: "Claude / Anthropic Gateway",
  },
  {
    id: "google",
    name: "Google",
    vendor: "Google DeepMind",
    description:
      "Gemini class models for multimodal, retrieval, and enterprise productivity use cases.",
    modelFamily: "Gemini 2.x",
  },
  {
    id: "xai",
    name: "xAI",
    vendor: "xAI",
    description:
      "Grok-oriented proxies for fast conversational and reasoning-heavy traffic.",
    modelFamily: "Grok 2 / 3",
  },
  {
    id: "groq",
    name: "Groq",
    vendor: "Groq",
    description:
      "OpenAI-compatible Groq LPU inference for low-latency open-source model traffic and fast agent loops.",
    modelFamily: "Llama / Gemma / Mixtral",
  },
  {
    id: "ollama",
    name: "Ollama",
    vendor: "Ollama",
    description:
      "Native Ollama local-runtime routing for on-device and self-hosted open-source model traffic without forcing the OpenAI compatibility layer.",
    modelFamily: "GLM / GPT-OSS / Llama / Gemma",
  },
  {
    id: "meta",
    name: "Meta",
    vendor: "Meta AI",
    description:
      "Llama family routing for open ecosystem deployment and partner cloud inference lanes.",
    modelFamily: "Llama 4 / Llama 3",
  },
  {
    id: "mistral",
    name: "Mistral",
    vendor: "Mistral AI",
    description:
      "European frontier model routing with strong coding, multilingual, and agentic workloads.",
    modelFamily: "Mistral Large / Codestral",
  },
  {
    id: "cohere",
    name: "Cohere",
    vendor: "Cohere",
    description:
      "Enterprise-grade command, retrieval, and embedding traffic for production assistants.",
    modelFamily: "Command / Embed",
  },
  {
    id: "amazon-nova",
    name: "Amazon Nova",
    vendor: "Amazon Web Services",
    description:
      "Bedrock-native Nova routes for enterprise scaling, governance, and multimodal orchestration.",
    modelFamily: "Nova Pro / Nova Micro",
  },
  {
    id: "amazon-bedrock-mantle",
    name: "Amazon Bedrock Mantle",
    vendor: "Amazon Web Services",
    description:
      "Mantle OpenAI-compatible Bedrock routing with automatic model discovery and bearer-token or IAM-backed auth.",
    modelFamily: "GPT-OSS / Qwen / Kimi / GLM",
  },
  {
    id: "microsoft",
    name: "Microsoft",
    vendor: "Microsoft AI",
    description:
      "Phi and Azure AI inference routes for cost-sensitive agents and enterprise hosting.",
    modelFamily: "Phi / MAI",
  },
  {
    id: "nvidia",
    name: "NVIDIA",
    vendor: "NVIDIA",
    description:
      "Nemotron and NIM-ready gateways for accelerated inference and private deployment stacks.",
    modelFamily: "Nemotron / NIM",
  },
  {
    id: "deepseek",
    name: "DeepSeek",
    vendor: "DeepSeek",
    description:
      "High-efficiency Chinese and coding-oriented model gateways with cost leverage.",
    modelFamily: "DeepSeek V3 / R1",
  },
  {
    id: "qwen",
    name: "Qwen",
    vendor: "Alibaba Cloud",
    description:
      "Alibaba Qwen routing for bilingual enterprise and tool-augmented workloads.",
    modelFamily: "Qwen 2.5 / QwQ",
  },
  {
    id: "azure-openai",
    name: "Azure OpenAI",
    vendor: "Microsoft Azure",
    description:
      "Resource-scoped Azure OpenAI routing for enterprise deployments and private networking.",
    modelFamily: "GPT-4.1 / o-series",
  },
  {
    id: "openrouter",
    name: "OpenRouter",
    vendor: "OpenRouter",
    description:
      "Multi-vendor gateway routing across OpenAI, Anthropic, Google, and partner model lanes.",
    modelFamily: "Multi-vendor frontier mix",
  },
  {
    id: "vercel-ai-gateway",
    name: "Vercel AI Gateway",
    vendor: "Vercel",
    description:
      "Anthropic-compatible Vercel AI Gateway routing with shared access to Anthropic, OpenAI, xAI, and other provider lanes.",
    modelFamily: "Anthropic / OpenAI / xAI",
  },
  {
    id: "litellm",
    name: "LiteLLM",
    vendor: "LiteLLM",
    description:
      "Self-hosted or managed model gateway routing that normalizes many upstream providers behind one OpenAI-compatible endpoint.",
    modelFamily: "Claude / GPT / Gemini / Multi-provider",
  },
  {
    id: "together",
    name: "Together",
    vendor: "Together AI",
    description:
      "Together AI hosted inference lanes for open-weight and partner frontier models through an OpenAI-compatible API.",
    modelFamily: "Kimi / GLM / Llama",
  },
  {
    id: "fireworks",
    name: "Fireworks",
    vendor: "Fireworks AI",
    description:
      "Fireworks open-weight and routed-model lanes exposed through an OpenAI-compatible inference endpoint.",
    modelFamily: "Kimi / Qwen / Gemma",
  },
  {
    id: "kilocode",
    name: "Kilo Code",
    vendor: "Kilo Code",
    description:
      "Kilo Gateway routing with automatic model selection and OpenAI-compatible agent access.",
    modelFamily: "Auto / Claude / GPT / Gemini",
  },
  {
    id: "venice",
    name: "Venice",
    vendor: "Venice AI",
    description:
      "Privacy-focused Venice AI inference through an OpenAI-compatible endpoint for chat and reasoning workloads.",
    modelFamily: "Kimi / Claude / Qwen",
  },
  {
    id: "vllm",
    name: "vLLM",
    vendor: "vLLM",
    description:
      "Self-hosted OpenAI-compatible serving for local or private-cluster large language model deployments.",
    modelFamily: "Self-hosted OSS models",
  },
  {
    id: "sglang",
    name: "SGLang",
    vendor: "SGLang",
    description:
      "Self-hosted OpenAI-compatible serving for local or cluster GPU deployments backed by the SGLang runtime.",
    modelFamily: "Self-hosted OSS models",
  },
  {
    id: "zhipu",
    name: "Z.AI",
    vendor: "Zhipu AI",
    description:
      "GLM model access for mainland connectivity, multimodal reasoning, and compliant enterprise integration.",
    modelFamily: "GLM-5 / GLM-Vision",
  },
  {
    id: "baidu",
    name: "Baidu",
    vendor: "Baidu AI Cloud",
    description:
      "ERNIE and Qianfan routing for enterprise Chinese language, reasoning, and search-heavy flows.",
    modelFamily: "ERNIE / X1",
  },
  {
    id: "tencent-hunyuan",
    name: "Tencent Hunyuan",
    vendor: "Tencent Cloud",
    description:
      "Hunyuan routing for consumer-scale assistants, mainland traffic, and enterprise copilots.",
    modelFamily: "Hunyuan Turbo / T1",
  },
  {
    id: "doubao",
    name: "Doubao",
    vendor: "ByteDance",
    description:
      "Volcengine Ark and Doubao routes for high-throughput consumer and business traffic.",
    modelFamily: "Doubao / Seed",
  },
  {
    id: "moonshot",
    name: "Moonshot AI",
    vendor: "Moonshot AI",
    description:
      "Kimi-oriented routing for long context, web-native reasoning, and global Chinese users.",
    modelFamily: "Kimi / K1",
  },
  {
    id: "minimax",
    name: "MiniMax",
    vendor: "MiniMax",
    description:
      "Multi-protocol model routing for multimodal chat and Claude-compatible workflows.",
    modelFamily: "MiniMax Text / M1",
  },
  {
    id: "stepfun",
    name: "StepFun",
    vendor: "StepFun",
    description:
      "Step series routing for reasoning, long-context orchestration, and fast domestic deployment.",
    modelFamily: "Step / Step-R",
  },
  {
    id: "sensenova",
    name: "SenseNova",
    vendor: "SenseTime",
    description:
      "SenseNova gateways for regulated enterprise deployments and multimodal industrial workloads.",
    modelFamily: "SenseChat / SenseNova",
  },
  {
    id: "baichuan",
    name: "Baichuan",
    vendor: "Baichuan Intelligence",
    description:
      "Baichuan family routes tuned for business assistants, bilingual chat, and finance use cases.",
    modelFamily: "Baichuan 4 / M1",
  },
  {
    id: "yi",
    name: "Yi",
    vendor: "01.AI",
    description:
      "Yi model routing for lean deployment, multilingual assistants, and pragmatic enterprise workloads.",
    modelFamily: "Yi / Yi Vision",
  },
  {
    id: "iflytek-spark",
    name: "iFlytek Spark",
    vendor: "iFlytek",
    description:
      "Spark routes for education and Chinese enterprise productivity flows.",
    modelFamily: "Spark / Xinghuo",
  },
  {
    id: "huawei-pangu",
    name: "Huawei Pangu",
    vendor: "Huawei Cloud",
    description:
      "Pangu model routing for sovereign cloud deployments and large-scale industry solutions.",
    modelFamily: "Pangu / Pangu Pro",
  },
];

type StoredLocalApiProxyProviderRoutingRecord = LocalApiProxyProviderRoutingRecord;

export function listKnownLocalApiProxyProviderRoutingChannels() {
  return providerChannelCatalog.map((channel) => ({ ...channel }));
}

function normalizeRuntimeConfig(
  input?: Partial<LocalApiProxyProviderRuntimeConfig> | null,
): LocalApiProxyProviderRuntimeConfig {
  return normalizeLocalApiProxyProviderRuntimeConfig(input);
}

function slugify(value: string) {
  return value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+/g, "")
    .replace(/-+$/g, "");
}

function normalizeProviderId(providerId: string | undefined | null) {
  return normalizeLocalApiProxyProviderKey(providerId);
}

export function normalizeLocalApiProxyProviderRoutingDraft(
  input: LocalApiProxyProviderRoutingDraftInput,
): LocalApiProxyProviderRoutingDraft {
  const inputName = input.name.trim();
  const inputProviderId = normalizeProviderId(input.providerId);
  const inputChannelId = normalizeProviderId(input.channelId);
  const normalizedRoute = normalizeProviderLocalApiProxyRouteRecord({
    name: input.name,
    providerId: input.providerId,
    channelId: input.channelId,
    clientProtocol: input.clientProtocol,
    upstreamProtocol: input.upstreamProtocol,
    upstreamBaseUrl: input.upstreamBaseUrl,
    baseUrl: input.baseUrl,
    apiKey: input.apiKey,
    enabled: input.enabled,
    isDefault: input.isDefault,
    managedBy: input.managedBy,
    defaultModelId: input.defaultModelId,
    reasoningModelId: input.reasoningModelId,
    embeddingModelId: input.embeddingModelId,
    models: input.models,
    notes: input.notes,
    exposeTo: input.exposeTo,
  });
  const normalizedProviderId =
    inputProviderId || inputChannelId
      ? normalizedRoute?.providerId || inputProviderId || inputChannelId
      : "";
  const normalizedUpstreamBaseUrl =
    normalizedRoute?.upstreamBaseUrl ||
    (input.upstreamBaseUrl ?? input.baseUrl ?? "").trim() ||
    LOCAL_AI_PROXY_DEFAULT_UPSTREAM_BASE_URL;

  return {
    presetId: input.presetId?.trim() || undefined,
    name: inputName,
    providerId: normalizedProviderId,
    clientProtocol: normalizedRoute?.clientProtocol || input.clientProtocol,
    upstreamProtocol: normalizedRoute?.upstreamProtocol || input.upstreamProtocol,
    upstreamBaseUrl: normalizedUpstreamBaseUrl,
    baseUrl: normalizedUpstreamBaseUrl,
    apiKey: normalizedRoute?.apiKey || input.apiKey.trim(),
    enabled:
      normalizedRoute?.enabled ??
      (typeof input.enabled === "boolean" ? input.enabled : true),
    isDefault: normalizedRoute?.isDefault ?? (input.isDefault === true),
    managedBy: normalizedRoute?.managedBy || input.managedBy || "user",
    defaultModelId: normalizedRoute?.defaultModelId || input.defaultModelId.trim(),
    reasoningModelId: normalizedRoute?.reasoningModelId,
    embeddingModelId: normalizedRoute?.embeddingModelId,
    models: normalizedRoute?.models.map((model) => ({ ...model })) || [],
    notes: normalizedRoute?.notes,
    exposeTo: normalizedRoute?.exposeTo.map((target) => target) || [],
    config: normalizeRuntimeConfig(input.config),
  };
}

function buildRecordId(providerId: string, now: number) {
  const slug = slugify(providerId) || "provider";
  return `${PROVIDER_CONFIG_ID_PREFIX}${slug}-${now.toString(36)}`;
}

function toRouteSeed(input: LocalApiProxyProviderRoutingDraft & { id?: string }) {
  const normalized = normalizeLocalApiProxyProviderRoutingDraft(input);

  return {
    id: input.id?.trim() || undefined,
    name: normalized.name,
    enabled: normalized.enabled,
    isDefault: normalized.isDefault,
    managedBy: normalized.managedBy,
    clientProtocol: normalized.clientProtocol,
    upstreamProtocol: normalized.upstreamProtocol,
    providerId: normalized.providerId,
    upstreamBaseUrl: normalized.upstreamBaseUrl,
    baseUrl: normalized.baseUrl,
    apiKey: normalized.apiKey,
    defaultModelId: normalized.defaultModelId,
    reasoningModelId: normalized.reasoningModelId,
    embeddingModelId: normalized.embeddingModelId,
    models: normalized.models,
    notes: normalized.notes,
    exposeTo: normalized.exposeTo,
  };
}

function createProviderRoutingRecord(
  route: LocalApiProxyProviderRouteRecord,
  metadata?: {
    presetId?: string;
    config?: Partial<LocalApiProxyProviderRuntimeConfig> | null;
    createdAt?: number;
    updatedAt?: number;
  },
): LocalApiProxyProviderRoutingRecord {
  return {
    ...route,
    presetId: metadata?.presetId?.trim() || undefined,
    baseUrl: route.upstreamBaseUrl,
    config: normalizeRuntimeConfig(metadata?.config),
    createdAt:
      typeof metadata?.createdAt === "number" && Number.isFinite(metadata.createdAt)
        ? metadata.createdAt
        : 0,
    updatedAt:
      typeof metadata?.updatedAt === "number" && Number.isFinite(metadata.updatedAt)
        ? metadata.updatedAt
        : 0,
  };
}

function parseStoredRecord(value: string | null): LocalApiProxyProviderRoutingRecord | null {
  if (!value) {
    return null;
  }

  try {
    const parsed = JSON.parse(value) as Record<string, unknown>;
    const route = normalizeProviderLocalApiProxyRouteRecord(parsed);

    if (!route) {
      return null;
    }

    return createProviderRoutingRecord(route, {
      presetId: typeof parsed.presetId === "string" ? parsed.presetId : undefined,
      config:
        parsed.config && typeof parsed.config === "object"
          ? (parsed.config as Partial<LocalApiProxyProviderRuntimeConfig>)
          : undefined,
      createdAt: typeof parsed.createdAt === "number" ? parsed.createdAt : 0,
      updatedAt: typeof parsed.updatedAt === "number" ? parsed.updatedAt : 0,
    });
  } catch {
    return null;
  }
}

function toSerializableRecord(record: LocalApiProxyProviderRoutingRecord) {
  return {
    ...record,
    baseUrl: record.upstreamBaseUrl,
  };
}

function sortRecords(records: LocalApiProxyProviderRoutingRecord[]) {
  return [...records].sort((left, right) => {
    if (left.isDefault !== right.isDefault) {
      return left.isDefault ? -1 : 1;
    }

    if (left.managedBy !== right.managedBy) {
      return left.managedBy === "user" ? -1 : 1;
    }

    if (right.updatedAt !== left.updatedAt) {
      return right.updatedAt - left.updatedAt;
    }

    return left.name.localeCompare(right.name);
  });
}

function stripShadowedSystemDefaults(records: LocalApiProxyProviderRoutingRecord[]) {
  const protocolsWithUserRoutes = new Set(
    records
      .filter((record) => record.managedBy === "user")
      .map((record) => record.clientProtocol),
  );

  return records.filter(
    (record) =>
      !(
        record.managedBy === "system-default" &&
        protocolsWithUserRoutes.has(record.clientProtocol)
      ),
  );
}

function normalizeProviderRoutingRecords(records: LocalApiProxyProviderRoutingRecord[]) {
  const normalizedInputRecords = stripShadowedSystemDefaults(records);
  const metadataById = new Map(
    normalizedInputRecords.map((record) => [record.id, record] as const),
  );

  return sortRecords(
    normalizeProviderLocalApiProxyRouteRecords(normalizedInputRecords).map((route) =>
      createProviderRoutingRecord(route, {
        presetId: metadataById.get(route.id)?.presetId,
        config: metadataById.get(route.id)?.config,
        createdAt: metadataById.get(route.id)?.createdAt,
        updatedAt: metadataById.get(route.id)?.updatedAt,
      }),
    ),
  );
}

function mapStoredProviderConfigToConfiguredProvider(
  record: StoredLocalApiProxyProviderRoutingRecord,
): LocalApiProxyConfiguredProvider {
  const status = !record.enabled
    ? "disabled"
    : record.apiKey || record.managedBy === "system-default"
      ? "active"
      : "warning";

  return {
    id: record.id,
    channelId: record.providerId,
    name: record.name,
    apiKey: record.apiKey,
    groupId: "local-api-proxy",
    usage: {
      requestCount: 0,
      tokenCount: 0,
      spendUsd: 0,
      period: "30d",
    },
    expiresAt: null,
    status,
    createdAt:
      typeof record.createdAt === "number"
        ? new Date(record.createdAt).toISOString()
        : null,
    baseUrl: record.upstreamBaseUrl,
    models: record.models.map((model) => ({
      id: model.id,
      name: model.name,
    })),
    notes: record.notes,
    credentialReference: "local-api-proxy",
    canCopyApiKey: false,
    clientProtocol: record.clientProtocol,
    upstreamProtocol: record.upstreamProtocol,
    managedBy: record.managedBy,
    enabled: record.enabled,
    isDefault: record.isDefault,
    defaultModelId: record.defaultModelId,
  };
}

function titleizeProviderId(providerId: string) {
  return providerId
    .split(/[-_]+/)
    .filter(Boolean)
    .map((segment) => segment.charAt(0).toUpperCase() + segment.slice(1))
    .join(" ");
}

function inferDynamicModelFamily(provider: LocalApiProxyConfiguredProvider) {
  const names = provider.models
    .map((model) => model.name.trim())
    .filter(Boolean)
    .slice(0, 2);

  if (names.length === 0) {
    return "Custom";
  }

  return names.join(" / ");
}

function buildDynamicChannelDefinition(
  provider: LocalApiProxyConfiguredProvider,
): LocalApiProxyProviderChannelDefinition {
  const providerName = titleizeProviderId(provider.channelId) || provider.channelId;

  return {
    id: provider.channelId,
    name: providerName,
    vendor: providerName,
    description: `${providerName} routes configured through the local API proxy catalog.`,
    modelFamily: inferDynamicModelFamily(provider),
  };
}

type SortableConfiguredProvider = LocalApiProxyConfiguredProvider & { updatedAtMs: number };

function sortProviders(providers: SortableConfiguredProvider[]) {
  return [...providers].sort((left, right) => {
    const updatedDiff = right.updatedAtMs - left.updatedAtMs;
    if (updatedDiff !== 0) {
      return updatedDiff;
    }

    return left.name.localeCompare(right.name);
  });
}

async function resolveStorageProfileId(storageApi: LocalApiProxyProviderRoutingStorageApi) {
  const info = await storageApi.getStorageInfo().catch(() => null);
  const writableSqliteProfiles =
    info?.profiles.filter(
      (profile) => profile.provider === "sqlite" && !profile.readOnly,
    ) || [];

  if (writableSqliteProfiles.length > 0) {
    const activeSqliteProfile = writableSqliteProfiles.find((profile) => profile.active);
    return activeSqliteProfile?.id || writableSqliteProfiles[0]!.id;
  }

  return (
    info?.profiles.find(
      (profile) => profile.id === DEFAULT_SQLITE_PROFILE_ID && !profile.readOnly,
    )?.id ||
    info?.profiles.find((profile) => profile.active && !profile.readOnly)?.id ||
    DEFAULT_SQLITE_PROFILE_ID
  );
}

export function createLocalApiProxyProviderRoutingCatalogService(
  options: CreateLocalApiProxyProviderRoutingCatalogServiceOptions,
): LocalApiProxyProviderRoutingCatalogService {
  const { storageApi } = options;
  const now = options.now || (() => Date.now());
  const storageNamespace =
    options.storageNamespace || LOCAL_API_PROXY_PROVIDER_ROUTE_STORAGE_NAMESPACE;

  async function listStoredProviderRoutingRecords(profileId: string) {
    const response = await storageApi.listKeys({
      profileId,
      namespace: storageNamespace,
    });
    const records = await Promise.all(
      response.keys.map(async (key) => {
        const entry = await storageApi.getText({
          profileId,
          namespace: storageNamespace,
          key,
        });
        return parseStoredRecord(entry.value);
      }),
    );

    return records.filter(
      (record): record is LocalApiProxyProviderRoutingRecord => Boolean(record),
    );
  }

  async function syncStoredProviderRoutingRecords(
    profileId: string,
    currentRecords: LocalApiProxyProviderRoutingRecord[],
    nextRecords: LocalApiProxyProviderRoutingRecord[],
  ) {
    const persistedNextRecords = nextRecords.filter(
      (record) => record.managedBy !== "system-default",
    );
    const nextIds = new Set(persistedNextRecords.map((record) => record.id));

    for (const record of persistedNextRecords) {
      await storageApi.putText({
        profileId,
        namespace: storageNamespace,
        key: record.id,
        value: JSON.stringify(toSerializableRecord(record)),
      });
    }

    for (const record of currentRecords) {
      if (!nextIds.has(record.id)) {
        await storageApi.delete({
          profileId,
          namespace: storageNamespace,
          key: record.id,
        });
      }
    }
  }

  return {
    async listProviderRoutingRecords() {
      const profileId = await resolveStorageProfileId(storageApi);
      return normalizeProviderRoutingRecords(
        await listStoredProviderRoutingRecords(profileId),
      );
    },

    async saveProviderRoutingRecord(
      input: LocalApiProxyProviderRoutingDraft & { id?: string },
    ) {
      const normalizedDraft = normalizeLocalApiProxyProviderRoutingDraft(input);
      if (!normalizedDraft.name) {
        throw new Error("Route name is required.");
      }
      if (!normalizedDraft.providerId) {
        throw new Error("Provider id is required.");
      }

      const currentTime = now();
      const profileId = await resolveStorageProfileId(storageApi);
      const currentRecords = await listStoredProviderRoutingRecords(profileId);
      const existingRecord = input.id?.trim()
        ? currentRecords.find((record) => record.id === input.id?.trim()) || null
        : null;
      const id =
        input.id?.trim() || buildRecordId(normalizedDraft.providerId, currentTime);
      const route = normalizeProviderLocalApiProxyRouteRecord({
        ...toRouteSeed(normalizedDraft),
        id,
        managedBy: normalizedDraft.managedBy || "user",
      });

      if (!route) {
        throw new Error("Unable to normalize the route config.");
      }
      if (route.models.length === 0) {
        throw new Error("At least one model is required.");
      }
      if (!route.defaultModelId) {
        throw new Error("Default model is required.");
      }
      if (!route.models.some((model) => model.id === route.defaultModelId)) {
        throw new Error("Default model must exist in the model list.");
      }
      if (
        route.reasoningModelId &&
        !route.models.some((model) => model.id === route.reasoningModelId)
      ) {
        throw new Error("Reasoning model must exist in the model list.");
      }
      if (
        route.embeddingModelId &&
        !route.models.some((model) => model.id === route.embeddingModelId)
      ) {
        throw new Error("Embedding model must exist in the model list.");
      }

      const record = createProviderRoutingRecord(route, {
        presetId: normalizedDraft.presetId,
        config: normalizedDraft.config,
        createdAt: existingRecord?.createdAt || currentTime,
        updatedAt: currentTime,
      });

      const remainingRecords = currentRecords.filter(
        (currentRecord) => currentRecord.id !== record.id,
      );
      const nextRecords = normalizeProviderRoutingRecords(
        record.isDefault ? [record, ...remainingRecords] : [...remainingRecords, record],
      );

      await syncStoredProviderRoutingRecords(profileId, currentRecords, nextRecords);

      return nextRecords.find((nextRecord) => nextRecord.id === record.id) || record;
    },

    async deleteProviderRoutingRecord(id: string) {
      const profileId = await resolveStorageProfileId(storageApi);
      const currentRecords = await listStoredProviderRoutingRecords(profileId);
      const normalizedId = id.trim();
      const existed = currentRecords.some((record) => record.id === normalizedId);
      const nextRecords = normalizeProviderRoutingRecords(
        currentRecords.filter((record) => record.id !== normalizedId),
      );

      await syncStoredProviderRoutingRecords(profileId, currentRecords, nextRecords);
      return existed;
    },

    async listConfiguredProviders() {
      const records = await this.listProviderRoutingRecords();
      const providers = records.map((record) => ({
        ...mapStoredProviderConfigToConfiguredProvider(record),
        updatedAtMs: record.updatedAt ?? record.createdAt ?? 0,
      }));

      return sortProviders(providers).map(
        ({ updatedAtMs: _updatedAtMs, ...provider }) => provider,
      );
    },

    async listProviderChannels() {
      const configuredProviders = await this.listConfiguredProviders();
      const definitions = new Map(
        providerChannelCatalog.map((channel) => [channel.id, channel] as const),
      );

      for (const provider of configuredProviders) {
        if (!definitions.has(provider.channelId)) {
          definitions.set(provider.channelId, buildDynamicChannelDefinition(provider));
        }
      }

      return [...definitions.values()].map((channel) => {
        const channelProviders = configuredProviders.filter(
          (provider) => provider.channelId === channel.id,
        );

        return {
          ...channel,
          providerCount: channelProviders.length,
          activeProviderCount: channelProviders.filter(
            (provider) => provider.status === "active",
          ).length,
          warningProviderCount: channelProviders.filter(
            (provider) =>
              provider.status === "warning" || provider.status === "expired",
          ).length,
          disabledProviderCount: channelProviders.filter(
            (provider) => provider.status === "disabled",
          ).length,
        } satisfies LocalApiProxyProviderChannel;
      });
    },
  };
}
