import type {
  LocalApiProxyProviderRouteClientProtocol,
  LocalApiProxyProviderRouteModelRecord,
  LocalApiProxyProviderRouteRecord,
} from "../types/localApiProxyProviderRoute.ts";
import {
  LOCAL_AI_PROXY_DEFAULT_CLIENT_PROTOCOL,
  listProviderLocalApiProxyRouteExposureTargets,
  normalizeProviderLocalApiProxyRouteRecords,
  selectDefaultProviderLocalApiProxyRouteForExposureTarget,
} from "./localApiProxyProviderRouteService.ts";

export interface LocalApiProxyProjectedModelRecord {
  id: string;
  name: string;
}

export interface LocalApiProxyProjectedSelection {
  defaultModelId: string;
  reasoningModelId?: string;
  embeddingModelId?: string;
}

export interface LocalApiProxyProjectedModelCatalogState {
  models: LocalApiProxyProjectedModelRecord[];
  selection: LocalApiProxyProjectedSelection;
}

export interface LocalApiProxyRuntimeEndpointSnapshot {
  rootBaseUrl?: string | null;
  baseUrl?: string | null;
  openaiCompatibleBaseUrl?: string | null;
  anthropicBaseUrl?: string | null;
  geminiBaseUrl?: string | null;
}

function normalizeProjectedModelId(value: string | undefined | null) {
  return value?.trim() || "";
}

function normalizeProjectedModelCatalog(
  models: readonly LocalApiProxyProjectedModelRecord[],
) {
  const normalizedModels = new Map<string, LocalApiProxyProjectedModelRecord>();

  for (const model of models) {
    const id = normalizeProjectedModelId(model.id);
    if (!id || normalizedModels.has(id)) {
      continue;
    }

    normalizedModels.set(id, {
      id,
      name: normalizeProjectedModelId(model.name) || id,
    });
  }

  return [...normalizedModels.values()];
}

export function resolveLocalApiProxyProjectedModelCatalogState(input: {
  models: readonly LocalApiProxyProjectedModelRecord[];
  selection: LocalApiProxyProjectedSelection;
  selectionOverride?: Partial<LocalApiProxyProjectedSelection>;
}): LocalApiProxyProjectedModelCatalogState {
  const normalizedModels = normalizeProjectedModelCatalog(input.models);
  const modelById = new Map(normalizedModels.map((model) => [model.id, model] as const));
  const baseDefaultModelId =
    normalizeProjectedModelId(input.selection.defaultModelId) || normalizedModels[0]?.id || "";
  const baseReasoningModelId =
    normalizeProjectedModelId(input.selection.reasoningModelId) || undefined;
  const baseEmbeddingModelId =
    normalizeProjectedModelId(input.selection.embeddingModelId) || undefined;
  const selectedDefaultModelId =
    normalizeProjectedModelId(input.selectionOverride?.defaultModelId) || baseDefaultModelId;
  const selectedReasoningModelId =
    normalizeProjectedModelId(input.selectionOverride?.reasoningModelId) || baseReasoningModelId;
  const selectedEmbeddingModelId =
    normalizeProjectedModelId(input.selectionOverride?.embeddingModelId) || baseEmbeddingModelId;

  if (!selectedDefaultModelId) {
    throw new Error("The selected route does not expose a default model for projection.");
  }
  if (!modelById.has(selectedDefaultModelId)) {
    throw new Error("The selected default model is not exposed by the projected route.");
  }
  if (selectedReasoningModelId && !modelById.has(selectedReasoningModelId)) {
    throw new Error("The selected reasoning model is not exposed by the projected route.");
  }
  if (selectedEmbeddingModelId && !modelById.has(selectedEmbeddingModelId)) {
    throw new Error("The selected embedding model is not exposed by the projected route.");
  }

  const orderedModelIds: string[] = [];
  const appendModelId = (modelId: string | undefined) => {
    if (!modelId || orderedModelIds.includes(modelId)) {
      return;
    }

    orderedModelIds.push(modelId);
  };

  appendModelId(selectedDefaultModelId);
  appendModelId(selectedReasoningModelId);
  appendModelId(selectedEmbeddingModelId);

  for (const model of normalizedModels) {
    appendModelId(model.id);
  }

  return {
    models: orderedModelIds.map((modelId) => modelById.get(modelId)!),
    selection: {
      defaultModelId: selectedDefaultModelId,
      reasoningModelId: selectedReasoningModelId,
      embeddingModelId: selectedEmbeddingModelId,
    },
  };
}

export function resolveLocalApiProxyRouteModelCatalogState(input: {
  routeModels: readonly LocalApiProxyProviderRouteModelRecord[];
  selection: LocalApiProxyProjectedSelection;
  selectionOverride?: Partial<LocalApiProxyProjectedSelection>;
}) {
  return resolveLocalApiProxyProjectedModelCatalogState({
    models: input.routeModels.map((model) => ({
      id: model.id,
      name: model.name,
    })),
    selection: input.selection,
    selectionOverride: input.selectionOverride,
  });
}

export function selectLocalApiProxyProjectedProviderRoute(input: {
  routes: readonly LocalApiProxyProviderRouteRecord[];
  exposureTarget: string;
  preferredClientProtocol?: LocalApiProxyProviderRouteClientProtocol;
  fallbackClientProtocol?: LocalApiProxyProviderRouteClientProtocol;
}) {
  const preferredProtocols = Array.from(
    new Set(
      [input.preferredClientProtocol, input.fallbackClientProtocol ?? LOCAL_AI_PROXY_DEFAULT_CLIENT_PROTOCOL]
        .filter((protocol): protocol is LocalApiProxyProviderRouteClientProtocol => Boolean(protocol)),
    ),
  );

  for (const protocol of preferredProtocols) {
    const route = selectDefaultProviderLocalApiProxyRouteForExposureTarget(
      input.routes,
      input.exposureTarget,
      protocol,
    );
    if (route) {
      return route;
    }
  }

  const normalizedRoutes = normalizeProviderLocalApiProxyRouteRecords(input.routes).filter((route) =>
    listProviderLocalApiProxyRouteExposureTargets(route).includes(input.exposureTarget.trim()),
  );
  return normalizedRoutes.find((route) => route.enabled && route.isDefault)
    || normalizedRoutes.find((route) => route.enabled)
    || null;
}

export function resolveLocalApiProxyRuntimeBaseUrl(
  snapshot: LocalApiProxyRuntimeEndpointSnapshot | null | undefined,
  clientProtocol: LocalApiProxyProviderRouteClientProtocol = LOCAL_AI_PROXY_DEFAULT_CLIENT_PROTOCOL,
  normalizeBaseUrl?: (baseUrl: string) => string | null | undefined,
) {
  const normalize = (baseUrl?: string | null) => {
    const normalized = baseUrl?.trim();
    if (!normalized) {
      return null;
    }

    return normalizeBaseUrl ? normalizeBaseUrl(normalized) || null : normalized;
  };

  switch (clientProtocol) {
    case "anthropic":
      return (
        normalize(snapshot?.anthropicBaseUrl)
        || normalize(snapshot?.openaiCompatibleBaseUrl)
        || normalize(snapshot?.baseUrl)
        || normalize(snapshot?.rootBaseUrl)
        || null
      );
    case "gemini":
      return (
        normalize(snapshot?.geminiBaseUrl)
        || normalize(snapshot?.rootBaseUrl)
        || normalize(snapshot?.baseUrl)
        || normalize(snapshot?.openaiCompatibleBaseUrl)
        || null
      );
    default:
      return (
        normalize(snapshot?.openaiCompatibleBaseUrl)
        || normalize(snapshot?.baseUrl)
        || normalize(snapshot?.anthropicBaseUrl)
        || normalize(snapshot?.rootBaseUrl)
        || null
      );
  }
}
