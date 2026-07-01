import {
  LOCAL_API_PROXY_SCHEMA_VERSION,
  type LocalApiCapability,
  type LocalApiClientProtocol,
  type LocalApiProxyExposure,
  type LocalApiProxyExposureTarget,
  type LocalApiProxyModelBinding,
  type LocalApiProxyRoute,
  type LocalApiUpstreamProtocol,
} from "../types/localApiProxyTypes.ts";
import {
  normalizeLocalApiProxyRoute,
  normalizeLocalApiProxyRouteId,
} from "../domain/localApiProxyConfig.ts";
import type {
  LocalApiProxyProviderRouteClientProtocol,
  LocalApiProxyProviderRouteManagedBy,
  LocalApiProxyProviderRouteModelRecord,
  LocalApiProxyProviderRouteModelState,
  LocalApiProxyProviderRouteRecord,
  LocalApiProxyProviderRouteUpstreamProtocol,
} from "../types/localApiProxyProviderRoute.ts";

export const LOCAL_AI_PROXY_ROUTE_SCHEMA_VERSION = LOCAL_API_PROXY_SCHEMA_VERSION;
export const LOCAL_AI_PROXY_DEFAULT_UPSTREAM_BASE_URL = "https://ai.sdkwork.com";
export const LOCAL_AI_PROXY_DEFAULT_CLIENT_API_KEY = "sk_sdkwork_api_key";
export const LOCAL_AI_PROXY_DEFAULT_CLIENT_PROTOCOL: LocalApiProxyProviderRouteClientProtocol =
  "openai-compatible";
export const LOCAL_AI_PROXY_SDKWORK_EXPOSE_TARGET = "sdkwork";
export const LOCAL_AI_PROXY_DESKTOP_CLIENTS_EXPOSE_TARGET = "desktop-clients";
export const LOCAL_AI_PROXY_SYSTEM_DEFAULT_OPENAI_ROUTE_ID =
  "local-ai-proxy-system-default-openai-compatible";

const PROVIDER_ROUTE_REQUIRED_DEFAULT_PROTOCOLS: readonly LocalApiProxyProviderRouteClientProtocol[] = [
  "openai-compatible",
  "anthropic",
  "gemini",
];

const LOCAL_AI_PROXY_DEFAULT_ROUTE_MODELS: LocalApiProxyProviderRouteModelRecord[] = [
  {
    id: "sdkwork-chat",
    name: "SDKWork Chat",
  },
  {
    id: "sdkwork-reasoning",
    name: "SDKWork Reasoning",
  },
  {
    id: "sdkwork-embedding",
    name: "SDKWork Embedding",
  },
];

export const LOCAL_API_PROXY_LEGACY_PROVIDER_KEY_PREFIX = "api-router-";

type UnknownRecord = Record<string, unknown>;

const LOCAL_AI_PROXY_CLIENT_PROTOCOLS: readonly LocalApiProxyProviderRouteClientProtocol[] = [
  "openai-compatible",
  "anthropic",
  "gemini",
];

const LOCAL_AI_PROXY_UPSTREAM_PROTOCOLS: readonly LocalApiProxyProviderRouteUpstreamProtocol[] = [
  "openai-compatible",
  "anthropic",
  "gemini",
  "ollama",
  "azure-openai",
  "openrouter",
  "sdkwork",
];

function isRecord(value: unknown): value is UnknownRecord {
  return Boolean(value && typeof value === "object" && !Array.isArray(value));
}

function normalizeString(value: unknown) {
  return typeof value === "string" ? value.trim() : "";
}

function normalizeOptionalString(value: unknown) {
  const normalized = normalizeString(value);
  return normalized || undefined;
}

export function normalizeLocalApiProxyLegacyProviderId(
  providerId: string | undefined | null,
) {
  const normalized = (providerId || "").trim();
  if (!normalized) {
    return "";
  }

  return normalized.startsWith(LOCAL_API_PROXY_LEGACY_PROVIDER_KEY_PREFIX)
    ? normalized.slice(LOCAL_API_PROXY_LEGACY_PROVIDER_KEY_PREFIX.length)
    : normalized;
}

export function normalizeLocalApiProxyLegacyProviderModelRef(
  value: string | undefined | null,
) {
  const normalized = (value || "").trim();
  if (!normalized) {
    return "";
  }

  const slashIndex = normalized.indexOf("/");
  if (slashIndex <= 0 || slashIndex === normalized.length - 1) {
    return normalized;
  }

  return `${normalizeLocalApiProxyLegacyProviderId(normalized.slice(0, slashIndex))}/${normalized.slice(slashIndex + 1)}`;
}

export function normalizeLocalApiProxyProviderKey(
  providerId: string | undefined | null,
) {
  return normalizeLocalApiProxyLegacyProviderId(providerId).toLowerCase();
}

function normalizeProviderKey(value: unknown) {
  return normalizeLocalApiProxyProviderKey(normalizeString(value));
}

function normalizeClientProtocol(
  value: unknown,
): LocalApiProxyProviderRouteClientProtocol | null {
  const normalized = normalizeString(value) as LocalApiProxyProviderRouteClientProtocol;
  return LOCAL_AI_PROXY_CLIENT_PROTOCOLS.includes(normalized) ? normalized : null;
}

function normalizeUpstreamProtocol(
  value: unknown,
): LocalApiProxyProviderRouteUpstreamProtocol | null {
  const normalized = normalizeString(value) as LocalApiProxyProviderRouteUpstreamProtocol;
  return LOCAL_AI_PROXY_UPSTREAM_PROTOCOLS.includes(normalized) ? normalized : null;
}

function normalizeManagedBy(value: unknown): LocalApiProxyProviderRouteManagedBy {
  return value === "system-default" ? "system-default" : "user";
}

function titleize(value: string) {
  return value
    .split(/[-_\s]+/g)
    .filter(Boolean)
    .map((segment) => segment.charAt(0).toUpperCase() + segment.slice(1))
    .join(" ");
}

function normalizeModelId(value: unknown) {
  return normalizeString(value);
}

function normalizeProviderModels(
  models: readonly LocalApiProxyProviderRouteModelRecord[] | undefined,
  extraModelIds: Array<string | undefined>,
) {
  const normalizedModels = new Map<string, LocalApiProxyProviderRouteModelRecord>();

  for (const model of models ?? []) {
    const id = normalizeModelId(model.id);
    if (!id) {
      continue;
    }

    normalizedModels.set(id, {
      id,
      name: normalizeString(model.name) || id,
    });
  }

  for (const modelId of extraModelIds) {
    const normalizedModelId = normalizeModelId(modelId);
    if (!normalizedModelId || normalizedModels.has(normalizedModelId)) {
      continue;
    }

    normalizedModels.set(normalizedModelId, {
      id: normalizedModelId,
      name: normalizedModelId,
    });
  }

  return [...normalizedModels.values()];
}

function inferCanonicalCapability(
  role: LocalApiProxyModelBinding["role"],
): LocalApiCapability | undefined {
  switch (role) {
    case "embedding":
      return "embedding";
    case "default":
    case "reasoning":
    case "vision":
      return "chat";
    case "rerank":
      return "rerank";
    default:
      return undefined;
  }
}

function buildModelBindings(route: {
  defaultModelId: string;
  reasoningModelId?: string;
  embeddingModelId?: string;
  models: LocalApiProxyProviderRouteModelRecord[];
}) {
  const modelNameById = new Map(route.models.map((model) => [model.id, model.name] as const));
  const bindings: LocalApiProxyModelBinding[] = [];
  const boundModelIds = new Set<string>();

  const appendBinding = (
    role: LocalApiProxyModelBinding["role"],
    modelId: string | undefined,
  ) => {
    const normalizedModelId = normalizeModelId(modelId);
    if (!normalizedModelId) {
      return;
    }

    boundModelIds.add(normalizedModelId);
    bindings.push({
      role,
      modelId: normalizedModelId,
      capability: inferCanonicalCapability(role),
      label: modelNameById.get(normalizedModelId) || normalizedModelId,
    });
  };

  appendBinding("default", route.defaultModelId);
  appendBinding("reasoning", route.reasoningModelId);
  appendBinding("embedding", route.embeddingModelId);

  for (const model of route.models) {
    if (boundModelIds.has(model.id)) {
      continue;
    }

    bindings.push({
      role: "custom",
      modelId: model.id,
      capability: undefined,
      label: model.name,
    });
  }

  return bindings;
}

function normalizeExposeTargets(exposeTo: readonly string[] | undefined) {
  const normalizedTargets = Array.from(
    new Set((exposeTo ?? []).map((entry) => normalizeString(entry)).filter(Boolean)),
  );

  return normalizedTargets.length > 0
    ? normalizedTargets
    : [LOCAL_AI_PROXY_SDKWORK_EXPOSE_TARGET];
}

function mapProviderExposeTarget(target: string): LocalApiProxyExposure {
  switch (target) {
    case LOCAL_AI_PROXY_SDKWORK_EXPOSE_TARGET:
      return {
        target: "sdkwork",
        enabled: true,
        label: "Sdkwork",
      };
    case LOCAL_AI_PROXY_DESKTOP_CLIENTS_EXPOSE_TARGET:
      return {
        target: "desktop-clients",
        enabled: true,
        label: "Desktop Clients",
      };
    case "internal-sdk":
      return {
        target: "internal-sdk",
        enabled: true,
        label: "Internal SDK",
      };
    default:
      return {
        target: "custom",
        enabled: true,
        consumerId: normalizeLocalApiProxyRouteId(target, "custom"),
        label: titleize(target),
      };
  }
}

function mapCanonicalExposureToProviderTarget(exposure: LocalApiProxyExposure) {
  if (exposure.target === "custom") {
    return exposure.consumerId || normalizeString(exposure.label) || "custom";
  }

  return exposure.target;
}

function buildProviderRouteId(
  managedBy: LocalApiProxyProviderRouteManagedBy,
  clientProtocol: LocalApiProxyProviderRouteClientProtocol,
  providerId: string,
) {
  if (managedBy === "system-default") {
    return clientProtocol === "openai-compatible"
      ? LOCAL_AI_PROXY_SYSTEM_DEFAULT_OPENAI_ROUTE_ID
      : `local-ai-proxy-system-default-${clientProtocol}`;
  }

  return `local-ai-route-${normalizeLocalApiProxyRouteId(providerId || clientProtocol, "route")}`;
}

function buildProviderRouteName(
  managedBy: LocalApiProxyProviderRouteManagedBy,
  providerId: string,
  clientProtocol: LocalApiProxyProviderRouteClientProtocol,
  explicitName: unknown,
) {
  const normalizedName = normalizeString(explicitName);
  if (normalizedName) {
    return normalizedName;
  }

  if (managedBy === "system-default") {
    return clientProtocol === "openai-compatible"
      ? "SDKWork Default"
      : `SDKWork ${titleize(clientProtocol)} Default`;
  }

  return titleize(providerId || clientProtocol || "route") || "Route";
}

function buildCanonicalCapabilities(
  route: Pick<
    LocalApiProxyProviderRouteRecord,
    "clientProtocol" | "defaultModelId" | "embeddingModelId"
  >,
) {
  const capabilities: Array<{
    capability: LocalApiCapability;
    enabled: true;
    operationSet: string[];
    streaming: boolean;
  }> = [];

  if (route.defaultModelId) {
    capabilities.push({
      capability: "chat",
      enabled: true,
      operationSet:
        route.clientProtocol === "openai-compatible"
          ? ["openai.v1.chat.completions.create"]
          : [],
      streaming: true,
    });
  }

  if (route.clientProtocol === "openai-compatible") {
    capabilities.push({
      capability: "response",
      enabled: true,
      operationSet: ["openai.v1.responses.create"],
      streaming: true,
    });
  }

  if (route.embeddingModelId) {
    capabilities.push({
      capability: "embedding",
      enabled: true,
      operationSet:
        route.clientProtocol === "openai-compatible"
          ? ["openai.v1.embeddings.create"]
          : [],
      streaming: false,
    });
  }

  return capabilities;
}

export function inferLocalAiProxyUpstreamProtocol(
  providerId: string,
): LocalApiProxyProviderRouteUpstreamProtocol {
  switch (normalizeProviderKey(providerId)) {
    case "anthropic":
    case "cloudflare-ai-gateway":
    case "vercel-ai-gateway":
      return "anthropic";
    case "google":
    case "gemini":
      return "gemini";
    case "ollama":
      return "ollama";
    case "azure":
    case "azure-openai":
      return "azure-openai";
    case "openrouter":
      return "openrouter";
    case "sdkwork":
      return "sdkwork";
    default:
      return "openai-compatible";
  }
}

export function inferLocalAiProxyClientProtocol(
  providerId: string,
): LocalApiProxyProviderRouteClientProtocol {
  const upstreamProtocol = inferLocalAiProxyUpstreamProtocol(providerId);
  if (upstreamProtocol === "anthropic" || upstreamProtocol === "gemini") {
    return upstreamProtocol;
  }

  return "openai-compatible";
}

export function createCanonicalLocalApiProxyRouteFromProviderRoute(
  input: Partial<LocalApiProxyProviderRouteRecord> & {
    providerId?: string;
    channelId?: string;
    baseUrl?: string;
    upstreamBaseUrl?: string;
    apiKey?: string;
    models?: LocalApiProxyProviderRouteModelRecord[];
    exposeTo?: string[];
  },
): LocalApiProxyRoute {
  const managedBy = normalizeManagedBy(input.managedBy);
  const providerId = normalizeProviderKey(input.providerId || input.channelId) || "sdkwork";
  const clientProtocol =
    normalizeClientProtocol(input.clientProtocol) || inferLocalAiProxyClientProtocol(providerId);
  const upstreamProtocol =
    normalizeUpstreamProtocol(input.upstreamProtocol) || inferLocalAiProxyUpstreamProtocol(providerId);
  const models = normalizeProviderModels(input.models, [
    normalizeModelId(input.defaultModelId),
    normalizeModelId(input.reasoningModelId),
    normalizeModelId(input.embeddingModelId),
  ]);

  return normalizeLocalApiProxyRoute({
    id:
      normalizeString(input.id) ||
      buildProviderRouteId(managedBy, clientProtocol, providerId),
    name: buildProviderRouteName(managedBy, providerId, clientProtocol, input.name),
    enabled: typeof input.enabled === "boolean" ? input.enabled : true,
    managedBy: managedBy === "system-default" ? "system" : "user",
    providerId,
    clientProtocol: clientProtocol as LocalApiClientProtocol,
    upstreamProtocol: upstreamProtocol as LocalApiUpstreamProtocol,
    upstream: {
      providerId,
      protocolKind: upstreamProtocol as LocalApiUpstreamProtocol,
      baseUrl:
        normalizeString(input.upstreamBaseUrl || input.baseUrl) ||
        LOCAL_AI_PROXY_DEFAULT_UPSTREAM_BASE_URL,
    },
    capabilities: buildCanonicalCapabilities({
      clientProtocol,
      defaultModelId: normalizeModelId(input.defaultModelId),
      embeddingModelId: normalizeOptionalString(input.embeddingModelId),
    }),
    modelBindings: buildModelBindings({
      defaultModelId: normalizeModelId(input.defaultModelId) || models[0]?.id || "",
      reasoningModelId: normalizeOptionalString(input.reasoningModelId),
      embeddingModelId: normalizeOptionalString(input.embeddingModelId),
      models,
    }),
    exposures: normalizeExposeTargets(input.exposeTo).map(mapProviderExposeTarget),
    tags: [],
    notes: normalizeOptionalString(input.notes),
  });
}

export function createProviderLocalApiProxyRouteFromCanonicalRoute(
  route: LocalApiProxyRoute,
  options?: {
    apiKey?: string;
    isDefault?: boolean;
  },
): LocalApiProxyProviderRouteRecord {
  const models = buildProviderModelCatalog(route);
  const defaultModelId =
    route.modelBindings.find((binding) => binding.role === "default")?.modelId || models[0]?.id || "";
  const reasoningModelId = route.modelBindings.find(
    (binding) => binding.role === "reasoning",
  )?.modelId;
  const embeddingModelId = route.modelBindings.find(
    (binding) => binding.role === "embedding",
  )?.modelId;

  return {
    id: route.id,
    schemaVersion: LOCAL_AI_PROXY_ROUTE_SCHEMA_VERSION,
    name: route.name,
    enabled: route.enabled,
    isDefault: options?.isDefault === true,
    managedBy: route.managedBy === "system" ? "system-default" : "user",
    clientProtocol:
      normalizeClientProtocol(route.clientProtocol) || LOCAL_AI_PROXY_DEFAULT_CLIENT_PROTOCOL,
    upstreamProtocol:
      normalizeUpstreamProtocol(route.upstreamProtocol) || "openai-compatible",
    providerId: route.providerId,
    upstreamBaseUrl: route.upstream.baseUrl || LOCAL_AI_PROXY_DEFAULT_UPSTREAM_BASE_URL,
    apiKey: normalizeString(options?.apiKey),
    defaultModelId,
    reasoningModelId: normalizeOptionalString(reasoningModelId),
    embeddingModelId: normalizeOptionalString(embeddingModelId),
    models,
    notes: normalizeOptionalString(route.notes),
    exposeTo: Array.from(
      new Set(
        route.exposures
          .filter((exposure) => exposure.enabled)
          .map(mapCanonicalExposureToProviderTarget)
          .map((entry) => normalizeString(entry))
          .filter(Boolean),
      ),
    ),
  };
}

export function createSystemDefaultProviderLocalApiProxyRoute(
  clientProtocol: LocalApiProxyProviderRouteClientProtocol = LOCAL_AI_PROXY_DEFAULT_CLIENT_PROTOCOL,
): LocalApiProxyProviderRouteRecord {
  const models = LOCAL_AI_PROXY_DEFAULT_ROUTE_MODELS.map((model) => ({ ...model }));

  return createProviderLocalApiProxyRouteFromCanonicalRoute(
    createCanonicalLocalApiProxyRouteFromProviderRoute({
      managedBy: "system-default",
      clientProtocol,
      upstreamProtocol: "sdkwork",
      providerId: "sdkwork",
      upstreamBaseUrl: LOCAL_AI_PROXY_DEFAULT_UPSTREAM_BASE_URL,
      defaultModelId: models[0]?.id,
      reasoningModelId: models[1]?.id,
      embeddingModelId: models[2]?.id,
      models,
      exposeTo:
        clientProtocol === "openai-compatible"
          ? [LOCAL_AI_PROXY_SDKWORK_EXPOSE_TARGET]
          : [LOCAL_AI_PROXY_SDKWORK_EXPOSE_TARGET, LOCAL_AI_PROXY_DESKTOP_CLIENTS_EXPOSE_TARGET],
    }),
    {
      apiKey: LOCAL_AI_PROXY_DEFAULT_CLIENT_API_KEY,
      isDefault: true,
    },
  );
}

export function normalizeProviderLocalApiProxyRouteRecord(input: unknown) {
  if (!isRecord(input)) {
    return null;
  }

  const providerId = normalizeProviderKey(input.providerId || input.channelId) || "sdkwork";
  const clientProtocol =
    normalizeClientProtocol(input.clientProtocol) || inferLocalAiProxyClientProtocol(providerId);

  if (normalizeManagedBy(input.managedBy) === "system-default") {
    return createSystemDefaultProviderLocalApiProxyRoute(clientProtocol);
  }

  return createProviderLocalApiProxyRouteFromCanonicalRoute(
    createCanonicalLocalApiProxyRouteFromProviderRoute(input),
    {
      apiKey: normalizeString(input.apiKey),
      isDefault: input.isDefault === true,
    },
  );
}

function normalizeProviderLocalApiProxyRouteState(route: LocalApiProxyProviderRouteRecord) {
  return createProviderLocalApiProxyRouteFromCanonicalRoute(
    createCanonicalLocalApiProxyRouteFromProviderRoute(route),
    {
      apiKey: route.apiKey,
      isDefault: route.isDefault,
    },
  );
}

export function listProviderLocalApiProxyRouteExposureTargets(
  route: LocalApiProxyProviderRouteRecord,
) {
  return normalizeProviderLocalApiProxyRouteState(route).exposeTo.map((target) => target);
}

export function resolveProviderLocalApiProxyRouteModelState(
  route: LocalApiProxyProviderRouteRecord,
): LocalApiProxyProviderRouteModelState {
  const normalizedRoute = normalizeProviderLocalApiProxyRouteState(route);

  return {
    defaultModelId: normalizedRoute.defaultModelId,
    reasoningModelId: normalizedRoute.reasoningModelId,
    embeddingModelId: normalizedRoute.embeddingModelId,
    models: normalizedRoute.models.map((model) => ({ ...model })),
  };
}

function buildProviderModelCatalog(route: LocalApiProxyRoute) {
  const orderedModels = new Map<string, LocalApiProxyProviderRouteModelRecord>();

  for (const role of ["default", "reasoning", "embedding", "custom"] as const) {
    for (const binding of route.modelBindings) {
      if (binding.role !== role) {
        continue;
      }

      const modelId = normalizeModelId(binding.modelId);
      if (!modelId || orderedModels.has(modelId)) {
        continue;
      }

      orderedModels.set(modelId, {
        id: modelId,
        name: normalizeString(binding.label) || modelId,
      });
    }
  }

  return [...orderedModels.values()];
}

function pickDefaultRouteIndex(routes: LocalApiProxyProviderRouteRecord[]) {
  const preferredExplicitDefaultIndex = routes.findIndex((route) => route.enabled && route.isDefault);
  if (preferredExplicitDefaultIndex >= 0) {
    return preferredExplicitDefaultIndex;
  }

  return routes.findIndex((route) => route.enabled);
}

function normalizeDefaultFlags(routes: LocalApiProxyProviderRouteRecord[]) {
  if (routes.length === 0) {
    return routes;
  }

  const defaultIndex = pickDefaultRouteIndex(routes);
  return routes.map((route, index) => ({
    ...route,
    isDefault: defaultIndex >= 0 ? index === defaultIndex : false,
  }));
}

function ensureRequiredDefaultProtocols(routes: LocalApiProxyProviderRouteRecord[]) {
  const nextRoutes = [...routes];

  for (const clientProtocol of PROVIDER_ROUTE_REQUIRED_DEFAULT_PROTOCOLS) {
    const protocolRoutes = nextRoutes.filter((route) => route.clientProtocol === clientProtocol);
    const hasEnabledDefault = protocolRoutes.some((route) => route.enabled && route.isDefault);
    if (hasEnabledDefault) {
      continue;
    }

    nextRoutes.push(createSystemDefaultProviderLocalApiProxyRoute(clientProtocol));
  }

  return nextRoutes;
}

export function normalizeProviderLocalApiProxyRouteRecords(inputs: readonly unknown[]) {
  const groupedRoutes = new Map<
    LocalApiProxyProviderRouteClientProtocol,
    LocalApiProxyProviderRouteRecord[]
  >();

  for (const input of inputs) {
    const route = normalizeProviderLocalApiProxyRouteRecord(input);
    if (!route) {
      continue;
    }

    const group = groupedRoutes.get(route.clientProtocol) || [];
    group.push(route);
    groupedRoutes.set(route.clientProtocol, group);
  }

  const normalizedRoutes = Array.from(groupedRoutes.values()).flatMap((routes) =>
    normalizeDefaultFlags(routes),
  );

  return ensureRequiredDefaultProtocols(normalizedRoutes);
}

function normalizeExposureTarget(exposureTarget: string) {
  return exposureTarget.trim();
}

function routeExposesTarget(
  route: LocalApiProxyProviderRouteRecord,
  exposureTarget: string,
) {
  const normalizedExposureTarget = normalizeExposureTarget(exposureTarget);
  if (!normalizedExposureTarget) {
    return true;
  }

  return listProviderLocalApiProxyRouteExposureTargets(route).some(
    (target) => target === normalizedExposureTarget,
  );
}

export function listActiveProviderLocalApiProxyRoutes(
  routes: readonly LocalApiProxyProviderRouteRecord[],
  clientProtocol?: LocalApiProxyProviderRouteClientProtocol,
) {
  return routes.filter(
    (route) =>
      route.enabled && (clientProtocol ? route.clientProtocol === clientProtocol : true),
  );
}

export function selectDefaultProviderLocalApiProxyRoute(
  routes: readonly LocalApiProxyProviderRouteRecord[],
  clientProtocol: LocalApiProxyProviderRouteClientProtocol = LOCAL_AI_PROXY_DEFAULT_CLIENT_PROTOCOL,
) {
  const protocolRoutes = normalizeProviderLocalApiProxyRouteRecords(routes).filter(
    (route) => route.clientProtocol === clientProtocol,
  );

  return (
    protocolRoutes.find((route) => route.enabled && route.isDefault) ||
    protocolRoutes.find((route) => route.enabled) ||
    null
  );
}

export function selectDefaultProviderLocalApiProxyRouteForExposureTarget(
  routes: readonly LocalApiProxyProviderRouteRecord[],
  exposureTarget: string,
  clientProtocol: LocalApiProxyProviderRouteClientProtocol = LOCAL_AI_PROXY_DEFAULT_CLIENT_PROTOCOL,
) {
  const normalizedExposureTarget = normalizeExposureTarget(exposureTarget);
  if (!normalizedExposureTarget) {
    return selectDefaultProviderLocalApiProxyRoute(routes, clientProtocol);
  }

  const protocolRoutes = normalizeProviderLocalApiProxyRouteRecords(routes).filter(
    (route) =>
      route.clientProtocol === clientProtocol &&
      routeExposesTarget(route, normalizedExposureTarget),
  );

  return (
    protocolRoutes.find((route) => route.enabled && route.isDefault) ||
    protocolRoutes.find((route) => route.enabled) ||
    (() => {
      const systemDefaultRoute = createSystemDefaultProviderLocalApiProxyRoute(clientProtocol);
      return routeExposesTarget(systemDefaultRoute, normalizedExposureTarget)
        ? systemDefaultRoute
        : null;
    })()
  );
}

export function resolveCanonicalLocalApiProxyExposureTarget(
  target: string,
): LocalApiProxyExposureTarget {
  switch (normalizeString(target)) {
    case "sdkwork":
      return "sdkwork";
    case "desktop-clients":
      return "desktop-clients";
    case "internal-sdk":
      return "internal-sdk";
    default:
      return "custom";
  }
}
