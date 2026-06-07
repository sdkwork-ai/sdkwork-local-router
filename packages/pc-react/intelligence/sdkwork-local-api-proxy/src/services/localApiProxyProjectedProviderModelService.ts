import type {
  LocalApiProxyProjectedModelRecord,
  LocalApiProxyProjectedSelection,
} from "./localApiProxyProjectionService.ts";
import {
  resolveLocalApiProxyProjectedModelCatalogState,
} from "./localApiProxyProjectionService.ts";

export type LocalApiProxyProjectedProviderModelRole =
  | "default"
  | "reasoning"
  | "embedding"
  | "fallback";

export interface LocalApiProxyProjectedProviderModelSeed {
  id: string;
  name: string;
  role?: LocalApiProxyProjectedProviderModelRole | string;
}

export interface LocalApiProxyProjectedProviderModelCatalogEntry {
  id: string;
  name: string;
  role: LocalApiProxyProjectedProviderModelRole;
  streaming: boolean;
}

export interface LocalApiProxyProjectedProviderModelCatalogState {
  models: LocalApiProxyProjectedProviderModelCatalogEntry[];
  selection: LocalApiProxyProjectedSelection;
}

function normalizeModelId(value: string | undefined | null) {
  return value?.trim() || "";
}

function normalizeExistingModelRole(
  value: string | undefined,
): LocalApiProxyProjectedProviderModelRole | undefined {
  switch ((value || "").trim().toLowerCase()) {
    case "primary":
    case "default":
      return "default";
    case "reasoning":
      return "reasoning";
    case "embedding":
      return "embedding";
    case "fallback":
      return "fallback";
    default:
      return undefined;
  }
}

function buildProjectedProviderModelCatalogSeed(input: {
  existingModels?: readonly LocalApiProxyProjectedProviderModelSeed[];
  selection: LocalApiProxyProjectedSelection;
  selectionOverride?: Partial<LocalApiProxyProjectedSelection>;
}) {
  const nextModels = new Map<string, LocalApiProxyProjectedModelRecord>();
  const appendModel = (modelId: string | undefined, modelName?: string) => {
    const normalizedModelId = normalizeModelId(modelId);
    if (!normalizedModelId || nextModels.has(normalizedModelId)) {
      return;
    }

    nextModels.set(normalizedModelId, {
      id: normalizedModelId,
      name: normalizeModelId(modelName) || normalizedModelId,
    });
  };

  input.existingModels?.forEach((model) => {
    appendModel(model.id, model.name);
  });

  [
    input.selection.defaultModelId,
    input.selection.reasoningModelId,
    input.selection.embeddingModelId,
    input.selectionOverride?.defaultModelId,
    input.selectionOverride?.reasoningModelId,
    input.selectionOverride?.embeddingModelId,
  ].forEach((modelId) => {
    appendModel(modelId, modelId);
  });

  return [...nextModels.values()];
}

function resolveProjectedProviderModelRole(args: {
  modelId: string;
  selection: LocalApiProxyProjectedSelection;
  existingModels?: readonly LocalApiProxyProjectedProviderModelSeed[];
}) {
  if (args.modelId === args.selection.defaultModelId) {
    return "default";
  }
  if (args.modelId === args.selection.reasoningModelId) {
    return "reasoning";
  }
  if (args.modelId === args.selection.embeddingModelId) {
    return "embedding";
  }

  const existingRole = normalizeExistingModelRole(
    args.existingModels?.find((entry) => normalizeModelId(entry.id) === args.modelId)?.role,
  );
  if (existingRole === "embedding") {
    return "embedding";
  }

  return "fallback";
}

function inferProjectedProviderModelStreaming(model: {
  id: string;
  name: string;
  role: LocalApiProxyProjectedProviderModelRole;
}) {
  if (model.role === "embedding") {
    return false;
  }

  const id = model.id.toLowerCase();
  const name = model.name.toLowerCase();
  return !(id.includes("embed") || name.includes("embed"));
}

export function resolveLocalApiProxyProjectedProviderModelCatalog(input: {
  existingModels?: readonly LocalApiProxyProjectedProviderModelSeed[];
  selection: LocalApiProxyProjectedSelection;
  selectionOverride?: Partial<LocalApiProxyProjectedSelection>;
}): LocalApiProxyProjectedProviderModelCatalogState {
  const normalizedModelCatalogState = resolveLocalApiProxyProjectedModelCatalogState({
    models: buildProjectedProviderModelCatalogSeed(input),
    selection: input.selection,
    selectionOverride: input.selectionOverride,
  });

  return {
    selection: normalizedModelCatalogState.selection,
    models: normalizedModelCatalogState.models.map((model) => {
      const role = resolveProjectedProviderModelRole({
        modelId: model.id,
        selection: normalizedModelCatalogState.selection,
        existingModels: input.existingModels,
      });

      return {
        id: model.id,
        name: model.name,
        role,
        streaming: inferProjectedProviderModelStreaming({
          id: model.id,
          name: model.name,
          role,
        }),
      };
    }),
  };
}
