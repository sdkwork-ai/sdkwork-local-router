import type {
  LocalApiCapability,
  LocalApiClientProtocol,
  LocalApiProxyConfig,
  LocalApiProxyExposure,
  LocalApiProxyExposureTarget,
  LocalApiProxyMode,
  LocalApiProxyRoute,
  LocalApiProxyStorageConfig,
  LocalApiProxyBind,
  LocalApiProxyModelBinding,
  ProxyUpstreamIdentity,
  RouteCapabilityBinding,
} from "../types/localApiProxyTypes.ts";

export interface LocalApiProxyRuntimeConsumerFilter {
  target: LocalApiProxyExposureTarget;
  consumerId?: string;
}

export interface LocalApiProxyRuntimeRoutePlan {
  routeId: string;
  routeName: string;
  providerId: string;
  clientProtocol: LocalApiClientProtocol;
  upstream: ProxyUpstreamIdentity;
  modelBindings: LocalApiProxyModelBinding[];
  exposures: LocalApiProxyExposure[];
  activeCapabilities: RouteCapabilityBinding[];
  tags: string[];
}

export interface LocalApiProxyRuntimeCapabilityEntry {
  routeIds: string[];
  defaultRouteId?: string;
}

export interface LocalApiProxyRuntimePlan {
  schemaVersion: number;
  mode: LocalApiProxyMode;
  bind: LocalApiProxyBind;
  storage: LocalApiProxyStorageConfig;
  routes: LocalApiProxyRuntimeRoutePlan[];
  capabilityIndex: Partial<Record<LocalApiCapability, LocalApiProxyRuntimeCapabilityEntry>>;
  protocolDefaults: Partial<Record<LocalApiClientProtocol, string>>;
}

function normalizeConsumerId(value: string | undefined) {
  return value?.trim().toLowerCase();
}

function routeMatchesConsumer(
  route: LocalApiProxyRoute,
  consumer: LocalApiProxyRuntimeConsumerFilter | undefined,
) {
  if (!consumer) {
    return true;
  }

  return route.exposures.some((exposure) => {
    if (!exposure.enabled || exposure.target !== consumer.target) {
      return false;
    }

    if (exposure.target !== "custom") {
      return true;
    }

    return normalizeConsumerId(exposure.consumerId) === normalizeConsumerId(consumer.consumerId);
  });
}

function toRuntimeRoute(route: LocalApiProxyRoute): LocalApiProxyRuntimeRoutePlan {
  return {
    routeId: route.id,
    routeName: route.name,
    providerId: route.providerId,
    clientProtocol: route.clientProtocol,
    upstream: route.upstream,
    modelBindings: route.modelBindings,
    exposures: route.exposures.filter((exposure) => exposure.enabled),
    activeCapabilities: route.capabilities.filter((binding) => binding.enabled),
    tags: route.tags,
  };
}

function selectCapabilityDefaultRouteId(
  config: LocalApiProxyConfig,
  capability: LocalApiCapability,
  routeIds: string[],
) {
  const configuredDefault = config.defaults.defaultRouteByCapability[capability];
  if (configuredDefault && routeIds.includes(configuredDefault)) {
    return configuredDefault;
  }

  return routeIds[0];
}

function selectProtocolDefaultRouteId(
  config: LocalApiProxyConfig,
  protocol: LocalApiClientProtocol,
  routeIds: string[],
) {
  const configuredDefault = config.defaults.defaultRouteByProtocol[protocol];
  if (configuredDefault && routeIds.includes(configuredDefault)) {
    return configuredDefault;
  }

  return routeIds[0];
}

export function compileLocalApiProxyRuntimePlan(
  config: LocalApiProxyConfig,
  options?: {
    consumer?: LocalApiProxyRuntimeConsumerFilter;
  },
): LocalApiProxyRuntimePlan {
  const runtimeRoutes = config.routes
    .filter((route) => route.enabled)
    .filter((route) => routeMatchesConsumer(route, options?.consumer))
    .map((route) => toRuntimeRoute(route))
    .filter((route) => route.activeCapabilities.length > 0);

  const capabilityIndex: LocalApiProxyRuntimePlan["capabilityIndex"] = {};

  for (const route of runtimeRoutes) {
    for (const capabilityBinding of route.activeCapabilities) {
      const current = capabilityIndex[capabilityBinding.capability] ?? {
        routeIds: [],
      };
      if (!current.routeIds.includes(route.routeId)) {
        current.routeIds.push(route.routeId);
      }
      capabilityIndex[capabilityBinding.capability] = current;
    }
  }

  for (const capability of Object.keys(capabilityIndex) as LocalApiCapability[]) {
    const routeIds = capabilityIndex[capability]?.routeIds ?? [];
    capabilityIndex[capability] = {
      routeIds,
      defaultRouteId: selectCapabilityDefaultRouteId(config, capability, routeIds),
    };
  }

  const protocolDefaults: LocalApiProxyRuntimePlan["protocolDefaults"] = {};

  for (const route of runtimeRoutes) {
    const routeIdsForProtocol = runtimeRoutes
      .filter((candidate) => candidate.clientProtocol === route.clientProtocol)
      .map((candidate) => candidate.routeId);

    if (!protocolDefaults[route.clientProtocol]) {
      protocolDefaults[route.clientProtocol] = selectProtocolDefaultRouteId(
        config,
        route.clientProtocol,
        routeIdsForProtocol,
      );
    }
  }

  return {
    schemaVersion: config.schemaVersion,
    mode: config.mode,
    bind: config.bind,
    storage: config.storage,
    routes: runtimeRoutes,
    capabilityIndex,
    protocolDefaults,
  };
}

export function selectLocalApiProxyDefaultRoute(
  plan: LocalApiProxyRuntimePlan,
  selector: {
    capability?: LocalApiCapability;
    clientProtocol?: LocalApiClientProtocol;
  },
) {
  if (selector.capability) {
    const capabilityDefaultRouteId = plan.capabilityIndex[selector.capability]?.defaultRouteId;
    if (capabilityDefaultRouteId) {
      return plan.routes.find((route) => route.routeId === capabilityDefaultRouteId) ?? null;
    }
  }

  if (selector.clientProtocol) {
    const protocolDefaultRouteId = plan.protocolDefaults[selector.clientProtocol];
    if (protocolDefaultRouteId) {
      return plan.routes.find((route) => route.routeId === protocolDefaultRouteId) ?? null;
    }
  }

  return plan.routes[0] ?? null;
}
