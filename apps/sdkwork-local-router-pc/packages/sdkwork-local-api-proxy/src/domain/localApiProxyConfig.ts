import {
  LOCAL_API_PROXY_DEFAULT_HOST,
  LOCAL_API_PROXY_DEFAULT_PORT,
  LOCAL_API_PROXY_DEFAULT_POSTGRES_SCHEMA,
  LOCAL_API_PROXY_SCHEMA_VERSION,
  type LocalApiClientProtocol,
  type LocalApiProxyBind,
  type LocalApiProxyCaptureConfig,
  type LocalApiProxyConfig,
  type LocalApiProxyConfigDraft,
  type LocalApiProxyDefaults,
  type LocalApiProxyExposure,
  type LocalApiProxyExposureDraft,
  type LocalApiProxyPolicies,
  type LocalApiProxyRoute,
  type LocalApiProxyRouteDraft,
  type LocalApiProxyRuntimeSettings,
  type ProxyUpstreamIdentity,
  type RouteCapabilityBinding,
} from "../types/localApiProxyTypes.ts";

const DEFAULT_REDACT_HEADERS = ["authorization", "x-api-key"] as const;
const DEFAULT_EXPOSURE_TARGET: LocalApiProxyExposure = {
  target: "desktop-clients",
  enabled: true,
  label: "Desktop Clients",
};

function normalizeString(value: unknown) {
  return typeof value === "string" ? value.trim() : "";
}

function normalizeOptionalString(value: unknown) {
  const normalized = normalizeString(value);
  return normalized || undefined;
}

function normalizeIdentifier(value: string, fallback: string) {
  const slug = value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+/g, "")
    .replace(/-+$/g, "");

  return slug || fallback;
}

function normalizeProviderId(value: string) {
  return normalizeIdentifier(value, "custom");
}

function normalizeStringList(values: readonly string[] | undefined, lowerCase = false) {
  const seen = new Set<string>();
  const normalized: string[] = [];

  for (const value of values ?? []) {
    const item = lowerCase ? normalizeString(value).toLowerCase() : normalizeString(value);
    if (!item || seen.has(item)) {
      continue;
    }

    seen.add(item);
    normalized.push(item);
  }

  return normalized;
}

function trimTrailingSlash(value: string) {
  return value.replace(/\/+$/g, "");
}

function toTitleCase(value: string) {
  return value
    .split(/[-_\s]+/g)
    .filter(Boolean)
    .map((segment) => segment.charAt(0).toUpperCase() + segment.slice(1))
    .join(" ");
}

function buildPublicBaseUrl(bind: Pick<LocalApiProxyBind, "host" | "port">) {
  return `http://${bind.host}:${bind.port}`;
}

function normalizeBind(bind: LocalApiProxyConfigDraft["bind"]): LocalApiProxyBind {
  const host = normalizeString(bind?.host) || LOCAL_API_PROXY_DEFAULT_HOST;
  const port = Number.isFinite(bind?.port) && Number(bind?.port) > 0
    ? Number(bind?.port)
    : LOCAL_API_PROXY_DEFAULT_PORT;

  return {
    host,
    port,
    publicBaseUrl: normalizeString(bind?.publicBaseUrl) || buildPublicBaseUrl({ host, port }),
  };
}

function normalizeCapture(capture: LocalApiProxyConfigDraft["capture"]): LocalApiProxyCaptureConfig {
  return {
    enabled: capture?.enabled ?? false,
    storeMessageContent: capture?.storeMessageContent ?? false,
    redactHeaders: normalizeStringList(capture?.redactHeaders, true).length > 0
      ? normalizeStringList(capture?.redactHeaders, true)
      : [...DEFAULT_REDACT_HEADERS],
    retentionDays:
      typeof capture?.retentionDays === "number" && capture.retentionDays > 0
        ? Math.floor(capture.retentionDays)
        : undefined,
  };
}

function normalizeStorage(storage: LocalApiProxyConfig["storage"]) {
  if (storage.dialect === "sqlite") {
    return {
      dialect: "sqlite" as const,
      sqlitePath: normalizeString(storage.sqlitePath),
    };
  }

  return {
    dialect: "postgresql" as const,
    postgresUrl: normalizeString(storage.postgresUrl),
    schema: normalizeString(storage.schema) || LOCAL_API_PROXY_DEFAULT_POSTGRES_SCHEMA,
  };
}

function normalizeCapabilityBindings(
  bindings: LocalApiProxyRouteDraft["capabilities"],
): RouteCapabilityBinding[] {
  return (bindings ?? []).map((binding) => ({
    capability: binding.capability,
    enabled: binding.enabled ?? true,
    operationSet: normalizeStringList(binding.operationSet),
    streaming: binding.streaming ?? false,
    timeoutMs:
      typeof binding.timeoutMs === "number" && binding.timeoutMs > 0
        ? Math.floor(binding.timeoutMs)
        : undefined,
    pathOverride: normalizeOptionalString(binding.pathOverride),
    methodOverride: binding.methodOverride?.toUpperCase() as RouteCapabilityBinding["methodOverride"],
    requestPolicyRef: normalizeOptionalString(binding.requestPolicyRef),
    responsePolicyRef: normalizeOptionalString(binding.responsePolicyRef),
  }));
}

function normalizeModelBindings(modelBindings: LocalApiProxyRouteDraft["modelBindings"]) {
  return (modelBindings ?? []).map((binding) => ({
    role: binding.role,
    modelId: normalizeString(binding.modelId),
    capability: binding.capability,
    label: normalizeOptionalString(binding.label),
  }));
}

function normalizeExposureDraft(
  exposure: LocalApiProxyExposureDraft,
  routeId: string,
): LocalApiProxyExposure {
  const target = exposure.target;
  const rawConsumerId =
    target === "custom"
      ? normalizeString(exposure.consumerId || exposure.label) || routeId
      : undefined;
  const consumerId = rawConsumerId
    ? normalizeIdentifier(rawConsumerId, routeId)
    : undefined;
  const labelSource = normalizeString(exposure.label) || rawConsumerId || target;

  return {
    target,
    enabled: exposure.enabled ?? true,
    consumerId,
    label: toTitleCase(labelSource),
  };
}

function normalizeExposureTargets(
  exposures: LocalApiProxyRouteDraft["exposures"],
  routeId: string,
) {
  if (!exposures || exposures.length === 0) {
    return [{ ...DEFAULT_EXPOSURE_TARGET }];
  }

  const seen = new Set<string>();
  const normalized: LocalApiProxyExposure[] = [];

  for (const exposure of exposures) {
    const nextExposure = normalizeExposureDraft(exposure, routeId);
    const key = `${nextExposure.target}:${nextExposure.consumerId ?? nextExposure.target}`;
    if (seen.has(key)) {
      continue;
    }

    seen.add(key);
    normalized.push(nextExposure);
  }

  return normalized.length > 0 ? normalized : [{ ...DEFAULT_EXPOSURE_TARGET }];
}

function normalizeUpstream(
  upstream: LocalApiProxyRouteDraft["upstream"],
): ProxyUpstreamIdentity {
  return {
    providerId: normalizeProviderId(upstream.providerId),
    protocolKind: upstream.protocolKind,
    mirrorProtocolIdentity: normalizeOptionalString(upstream.mirrorProtocolIdentity),
    baseUrl: trimTrailingSlash(normalizeString(upstream.baseUrl)),
    credentialRef: normalizeOptionalString(upstream.credentialRef),
  };
}

function normalizeRouteName(route: LocalApiProxyRouteDraft, routeId: string) {
  const explicitName = normalizeString(route.name);
  if (explicitName) {
    return explicitName;
  }

  return toTitleCase(routeId);
}

export function normalizeLocalApiProxyRouteId(value: string, fallback = "route") {
  return normalizeIdentifier(value, fallback);
}

export function normalizeLocalApiProxyRoute(route: LocalApiProxyRouteDraft): LocalApiProxyRoute {
  const providerId = normalizeProviderId(route.providerId);
  const routeId = normalizeLocalApiProxyRouteId(
    normalizeString(route.id) || normalizeString(route.name) || providerId,
    "route",
  );

  return {
    id: routeId,
    name: normalizeRouteName(route, routeId),
    enabled: route.enabled ?? true,
    managedBy: route.managedBy ?? "user",
    providerId,
    clientProtocol: route.clientProtocol,
    upstreamProtocol: route.upstreamProtocol,
    upstream: normalizeUpstream(route.upstream),
    capabilities: normalizeCapabilityBindings(route.capabilities),
    modelBindings: normalizeModelBindings(route.modelBindings),
    runtimePolicy: route.runtimePolicy,
    exposures: normalizeExposureTargets(route.exposures, routeId),
    tags: normalizeStringList(route.tags),
    notes: normalizeOptionalString(route.notes),
  };
}

function normalizeDefaults(
  defaults: LocalApiProxyConfigDraft["defaults"],
): LocalApiProxyDefaults {
  const defaultRouteByCapability = Object.fromEntries(
    Object.entries(defaults?.defaultRouteByCapability ?? {})
      .map(([capability, routeId]) => [capability, normalizeLocalApiProxyRouteId(routeId, "")])
      .filter(([, routeId]) => Boolean(routeId)),
  ) as LocalApiProxyDefaults["defaultRouteByCapability"];

  const defaultRouteByProtocol = Object.fromEntries(
    Object.entries(defaults?.defaultRouteByProtocol ?? {})
      .map(([protocol, routeId]) => [
        protocol as LocalApiClientProtocol,
        normalizeLocalApiProxyRouteId(routeId, ""),
      ])
      .filter(([, routeId]) => Boolean(routeId)),
  ) as LocalApiProxyDefaults["defaultRouteByProtocol"];

  return {
    defaultRouteByCapability,
    defaultRouteByProtocol,
  };
}

function normalizePolicies(policies: LocalApiProxyConfigDraft["policies"]): LocalApiProxyPolicies {
  return {
    request: Object.fromEntries(
      Object.entries(policies?.request ?? {})
        .map(([key, value]) => [normalizeString(key), normalizeString(value)])
        .filter(([key, value]) => Boolean(key) && Boolean(value)),
    ),
    response: Object.fromEntries(
      Object.entries(policies?.response ?? {})
        .map(([key, value]) => [normalizeString(key), normalizeString(value)])
        .filter(([key, value]) => Boolean(key) && Boolean(value)),
    ),
  };
}

function normalizeRuntime(runtime: LocalApiProxyConfigDraft["runtime"]): LocalApiProxyRuntimeSettings {
  return {
    retryCount:
      typeof runtime?.retryCount === "number" && runtime.retryCount >= 0
        ? Math.floor(runtime.retryCount)
        : 1,
    cleanupIntervalMs:
      typeof runtime?.cleanupIntervalMs === "number" && runtime.cleanupIntervalMs > 0
        ? Math.floor(runtime.cleanupIntervalMs)
        : 300_000,
    maxConcurrentRequests:
      typeof runtime?.maxConcurrentRequests === "number" && runtime.maxConcurrentRequests > 0
        ? Math.floor(runtime.maxConcurrentRequests)
        : 8,
    startupProbeTimeoutMs:
      typeof runtime?.startupProbeTimeoutMs === "number" && runtime.startupProbeTimeoutMs > 0
        ? Math.floor(runtime.startupProbeTimeoutMs)
        : 15_000,
  };
}

export function normalizeLocalApiProxyConfig(
  draft: LocalApiProxyConfigDraft,
): LocalApiProxyConfig {
  const storage = normalizeStorage(draft.storage);
  const mode = draft.mode ?? (storage.dialect === "postgresql" ? "server-managed" : "desktop-local");

  return {
    schemaVersion:
      typeof draft.schemaVersion === "number" && draft.schemaVersion > 0
        ? Math.floor(draft.schemaVersion)
        : LOCAL_API_PROXY_SCHEMA_VERSION,
    mode,
    bind: normalizeBind(draft.bind),
    storage,
    capture: normalizeCapture(draft.capture),
    routes: (draft.routes ?? []).map((route) => normalizeLocalApiProxyRoute(route)),
    defaults: normalizeDefaults(draft.defaults),
    policies: normalizePolicies(draft.policies),
    runtime: normalizeRuntime(draft.runtime),
  };
}

export function createDefaultLocalApiProxyConfig(
  draft: Pick<LocalApiProxyConfigDraft, "storage"> &
    Omit<Partial<LocalApiProxyConfigDraft>, "storage">,
) {
  return normalizeLocalApiProxyConfig({
    ...draft,
    routes: draft.routes ?? [],
  });
}
