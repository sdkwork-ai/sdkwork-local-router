import {
  resolveLocalApiProxyProjectedModelCatalogState,
  resolveLocalApiProxyRuntimeBaseUrl,
} from "../services/localApiProxyProjectionService.ts";
import {
  createLocalApiProxyProviderRoutingCatalogService,
  normalizeLocalApiProxyProviderRoutingDraft,
  type CreateLocalApiProxyProviderRoutingCatalogServiceOptions,
  type LocalApiProxyProviderRoutingCatalogService,
} from "../services/localApiProxyProviderRoutingCatalogService.ts";
import {
  LOCAL_AI_PROXY_DEFAULT_CLIENT_PROTOCOL,
} from "../services/localApiProxyProviderRouteService.ts";
import type {
  LocalApiProxyProviderRouteClientProtocol,
} from "../types/localApiProxyProviderRoute.ts";
import { listLocalApiProxyProviderConfigPresets } from "./localApiProxyProviderConfigPresets.ts";
import type {
  LocalApiProxyApplyProjectionInput,
  LocalApiProxyManagedKernelProjection,
  LocalApiProxyManagedKernelProjectionSelection,
  LocalApiProxyProviderApplyTargetDetail,
  LocalApiProxyProviderApplyTargetRecord,
  LocalApiProxyProviderCenterActionSupport,
  LocalApiProxyProviderConfigDraft,
  LocalApiProxyProviderConfigRecord,
  LocalApiProxyProviderConfigPreset,
} from "./localApiProxyProviderCenterTypes.ts";

export const LOCAL_API_PROXY_MANAGED_PROVIDER_ID = "sdkwork-local-proxy";
export const LOCAL_API_PROXY_MANAGED_TOKEN_ENV_VAR = "SDKWORK_LOCAL_PROXY_TOKEN";
export const LOCAL_API_PROXY_MANAGED_API_KEY_PLACEHOLDER =
  "${SDKWORK_LOCAL_PROXY_TOKEN}";

export interface ApplyLocalApiProxyProviderConfigInput {
  targetId: string;
  config: LocalApiProxyProviderConfigRecord;
  agentIds?: string[];
}

interface LocalApiProxyRuntimeApi {
  getInfo(): Promise<any>;
  ensureRunning(): Promise<unknown>;
  testLocalApiProxyRoute?(routeId: string): Promise<any>;
}

interface LocalApiProxyProviderApplyApi {
  listTargets(): Promise<LocalApiProxyProviderApplyTargetRecord[]>;
  getTarget(targetId: string): Promise<LocalApiProxyProviderApplyTargetDetail | null>;
  applyProjection(input: LocalApiProxyApplyProjectionInput): Promise<void>;
}

interface LocalApiProxyProviderCenterServiceDependencies {
  providerRoutingApi: Pick<
    LocalApiProxyProviderRoutingCatalogService,
    "listProviderRoutingRecords" | "saveProviderRoutingRecord" | "deleteProviderRoutingRecord"
  >;
  runtimeApi: LocalApiProxyRuntimeApi;
  applyApi?: LocalApiProxyProviderApplyApi;
}

export interface LocalApiProxyProviderCenterServiceOverrides {
  storageApi?: CreateLocalApiProxyProviderRoutingCatalogServiceOptions["storageApi"];
  now?: () => number;
  providerRoutingApi?: Partial<LocalApiProxyProviderCenterServiceDependencies["providerRoutingApi"]>;
  runtimeApi?: Partial<LocalApiProxyRuntimeApi>;
  applyApi?: Partial<LocalApiProxyProviderApplyApi>;
}

const LOCAL_AI_PROXY_RUNTIME_UNAVAILABLE_ERROR =
  "The local AI proxy runtime is not available for the active platform bridge.";
const LOCAL_AI_PROXY_RUNTIME_STATUS_UNAVAILABLE_ERROR =
  "The local AI proxy runtime status is temporarily unavailable.";
const LOCAL_AI_PROXY_QUICK_APPLY_TARGET_UNAVAILABLE_ERROR =
  "No writable local API proxy kernel target is available for quick apply.";
const LOCAL_AI_PROXY_QUICK_APPLY_TARGETS_UNAVAILABLE_ERROR =
  "Quick apply targets are temporarily unavailable.";
const LOCAL_AI_PROXY_QUICK_APPLY_REQUIRES_LOOPBACK_ERROR =
  "Quick apply requires a loopback-only local AI proxy runtime.";

function indexRouteRuntimeMetrics(metrics: any[] | undefined | null) {
  return new Map((metrics || []).map((metric) => [metric.routeId, metric] as const));
}

function indexRouteLatestTests(tests: any[] | undefined | null) {
  return new Map((tests || []).map((test) => [test.routeId, test] as const));
}

function attachRouteRuntimeState(
  records: LocalApiProxyProviderConfigRecord[],
  metrics: any[] | undefined | null,
  tests: any[] | undefined | null,
) {
  const metricsByRouteId = indexRouteRuntimeMetrics(metrics);
  const testsByRouteId = indexRouteLatestTests(tests);

  return records.map((record) => ({
    ...record,
    runtimeMetrics: metricsByRouteId.get(record.id),
    latestTest: testsByRouteId.get(record.id) ?? null,
  }));
}

function hasAvailableLocalAiProxyRuntime(kernelInfo: any) {
  return ["openai-compatible", "anthropic", "gemini"].some((clientProtocol) =>
    Boolean(
      resolveLocalApiProxyRuntimeBaseUrl(
        kernelInfo?.localAiProxy,
        clientProtocol as LocalApiProxyProviderRouteClientProtocol,
      ),
    ),
  );
}

function hasLoopbackOnlyLocalAiProxyRuntime(kernelInfo: any) {
  return kernelInfo?.localAiProxy?.loopbackOnly === true;
}

function ensureWritableApplyTarget(
  detail: LocalApiProxyProviderApplyTargetDetail | null,
): LocalApiProxyProviderApplyTargetDetail {
  if (!detail) {
    throw new Error(LOCAL_AI_PROXY_QUICK_APPLY_TARGET_UNAVAILABLE_ERROR);
  }
  if (!detail.target.writable || !detail.target.configFile) {
    throw new Error(LOCAL_AI_PROXY_QUICK_APPLY_TARGET_UNAVAILABLE_ERROR);
  }

  return detail;
}

function ensureWritableApplyTargetRecord(
  target: LocalApiProxyProviderApplyTargetRecord | null,
): LocalApiProxyProviderApplyTargetRecord {
  if (!target || !target.writable || !target.configFile) {
    throw new Error(LOCAL_AI_PROXY_QUICK_APPLY_TARGET_UNAVAILABLE_ERROR);
  }

  return target;
}

function toUniqueIds(values: string[]) {
  return Array.from(
    new Set(
      values.map((value) => value.trim()).filter(Boolean),
    ),
  );
}

function createManagedKernelProjection(input: {
  route: LocalApiProxyProviderConfigRecord;
  proxyBaseUrl: string;
}): LocalApiProxyManagedKernelProjection {
  const selection = resolveLocalApiProxyProjectedModelCatalogState({
    models: input.route.models.map((model) => ({
      id: model.id,
      name: model.name,
    })),
    selection: {
      defaultModelId: input.route.defaultModelId,
      reasoningModelId: input.route.reasoningModelId,
      embeddingModelId: input.route.embeddingModelId,
    },
  });

  return {
    sourceRoute: input.route,
    provider: {
      id: LOCAL_API_PROXY_MANAGED_PROVIDER_ID,
      channelId: input.route.clientProtocol,
      name: "SDKWork Local Proxy",
      apiKey: LOCAL_API_PROXY_MANAGED_API_KEY_PLACEHOLDER,
      baseUrl: input.proxyBaseUrl.trim(),
      models: selection.models.map((model) => ({
        id: model.id,
        name: model.name,
      })),
      notes: `Managed local proxy projection for route "${input.route.name}".`,
      config: input.route.config,
    },
    selection: selection.selection as LocalApiProxyManagedKernelProjectionSelection,
  };
}

class LocalApiProxyProviderCenterService {
  private readonly dependencies: LocalApiProxyProviderCenterServiceDependencies;

  constructor(dependencies: LocalApiProxyProviderCenterServiceDependencies) {
    this.dependencies = dependencies;
  }

  listPresets(): LocalApiProxyProviderConfigPreset[] {
    return listLocalApiProxyProviderConfigPresets();
  }

  private async resolveRuntimeInfo() {
    try {
      return {
        info: await this.dependencies.runtimeApi.getInfo(),
        readFailed: false,
      };
    } catch {
      return {
        info: null,
        readFailed: true,
      };
    }
  }

  private async syncRuntime() {
    try {
      await this.dependencies.runtimeApi.ensureRunning();
    } catch {
      // Route catalog persistence is authoritative; runtime sync is best-effort.
    }
  }

  async listProviderConfigs() {
    const [records, runtimeInfoResult] = await Promise.all([
      this.dependencies.providerRoutingApi.listProviderRoutingRecords(),
      this.resolveRuntimeInfo(),
    ]);

    return attachRouteRuntimeState(
      records,
      runtimeInfoResult.info?.localAiProxy?.routeMetrics,
      runtimeInfoResult.info?.localAiProxy?.routeTests,
    );
  }

  async saveProviderConfig(input: LocalApiProxyProviderConfigDraft & { id?: string }) {
    const normalizedDraft = normalizeLocalApiProxyProviderRoutingDraft(input);
    const record = await this.dependencies.providerRoutingApi.saveProviderRoutingRecord({
      ...normalizedDraft,
      id: input.id?.trim() || undefined,
    });
    await this.syncRuntime();
    return record;
  }

  async deleteProviderConfig(id: string) {
    const existed = await this.dependencies.providerRoutingApi.deleteProviderRoutingRecord(id.trim());
    await this.syncRuntime();
    return existed;
  }

  async getActionSupport(): Promise<LocalApiProxyProviderCenterActionSupport> {
    const runtimeInfoResult = await this.resolveRuntimeInfo();
    if (runtimeInfoResult.readFailed) {
      return {
        quickApply: {
          available: false,
          reasonKey: "runtimeStatusUnavailable",
          reason: LOCAL_AI_PROXY_RUNTIME_STATUS_UNAVAILABLE_ERROR,
        },
        test: {
          available: false,
          reasonKey: "runtimeStatusUnavailable",
          reason: LOCAL_AI_PROXY_RUNTIME_STATUS_UNAVAILABLE_ERROR,
        },
      };
    }

    if (!hasAvailableLocalAiProxyRuntime(runtimeInfoResult.info)) {
      return {
        quickApply: {
          available: false,
          reasonKey: "runtimeUnavailable",
          reason: LOCAL_AI_PROXY_RUNTIME_UNAVAILABLE_ERROR,
        },
        test: {
          available: false,
          reasonKey: "runtimeUnavailable",
          reason: LOCAL_AI_PROXY_RUNTIME_UNAVAILABLE_ERROR,
        },
      };
    }

    if (!hasLoopbackOnlyLocalAiProxyRuntime(runtimeInfoResult.info)) {
      return {
        quickApply: {
          available: false,
          reasonKey: "quickApplyRequiresLoopback",
          reason: LOCAL_AI_PROXY_QUICK_APPLY_REQUIRES_LOOPBACK_ERROR,
        },
        test: {
          available: true,
        },
      };
    }

    try {
      const targets = await this.listApplyTargets();
      return {
        quickApply: targets.length > 0
          ? { available: true }
          : {
              available: false,
              reasonKey: "quickApplyInstanceUnavailable",
              reason: LOCAL_AI_PROXY_QUICK_APPLY_TARGET_UNAVAILABLE_ERROR,
            },
        test: {
          available: true,
        },
      };
    } catch {
      return {
        quickApply: {
          available: false,
          reasonKey: "quickApplyTargetsUnavailable",
          reason: LOCAL_AI_PROXY_QUICK_APPLY_TARGETS_UNAVAILABLE_ERROR,
        },
        test: {
          available: true,
        },
      };
    }
  }

  async listApplyTargets() {
    if (!this.dependencies.applyApi) {
      return [];
    }

    const runtimeInfoResult = await this.resolveRuntimeInfo();
    if (runtimeInfoResult.readFailed || !hasAvailableLocalAiProxyRuntime(runtimeInfoResult.info)) {
      return [];
    }
    if (!hasLoopbackOnlyLocalAiProxyRuntime(runtimeInfoResult.info)) {
      return [];
    }

    return (await this.dependencies.applyApi.listTargets()).filter(
      (target) => target.writable && Boolean(target.configFile),
    );
  }

  async getApplyTarget(targetId: string) {
    if (!this.dependencies.applyApi) {
      throw new Error(LOCAL_AI_PROXY_QUICK_APPLY_TARGET_UNAVAILABLE_ERROR);
    }

    return ensureWritableApplyTarget(
      await this.dependencies.applyApi.getTarget(targetId.trim()),
    );
  }

  private async resolveApplyProjectionTarget(targetId: string) {
    if (!this.dependencies.applyApi) {
      throw new Error(LOCAL_AI_PROXY_QUICK_APPLY_TARGET_UNAVAILABLE_ERROR);
    }

    const normalizedTargetId = targetId.trim();
    const target = (await this.dependencies.applyApi.listTargets()).find(
      (candidate) => candidate.id === normalizedTargetId,
    ) ?? null;

    return ensureWritableApplyTargetRecord(target);
  }

  async applyProviderConfig(input: ApplyLocalApiProxyProviderConfigInput) {
    if (!this.dependencies.applyApi) {
      throw new Error(LOCAL_AI_PROXY_QUICK_APPLY_TARGET_UNAVAILABLE_ERROR);
    }

    await this.dependencies.runtimeApi.ensureRunning();
    const runtimeInfoResult = await this.resolveRuntimeInfo();
    if (runtimeInfoResult.readFailed) {
      throw new Error(LOCAL_AI_PROXY_RUNTIME_STATUS_UNAVAILABLE_ERROR);
    }
    if (!hasLoopbackOnlyLocalAiProxyRuntime(runtimeInfoResult.info)) {
      throw new Error(LOCAL_AI_PROXY_QUICK_APPLY_REQUIRES_LOOPBACK_ERROR);
    }

    const target = await this.resolveApplyProjectionTarget(input.targetId);
    const proxyBaseUrl = resolveLocalApiProxyRuntimeBaseUrl(
      runtimeInfoResult.info?.localAiProxy,
      input.config.clientProtocol || LOCAL_AI_PROXY_DEFAULT_CLIENT_PROTOCOL,
    );
    if (!proxyBaseUrl) {
      throw new Error(LOCAL_AI_PROXY_RUNTIME_UNAVAILABLE_ERROR);
    }

    await this.dependencies.applyApi.applyProjection({
      targetId: target.id,
      configFile: target.configFile!,
      projection: createManagedKernelProjection({
        route: input.config,
        proxyBaseUrl,
      }),
      agentIds: toUniqueIds(input.agentIds || []),
    });
  }

  async testProviderConfigRoute(routeId: string) {
    await this.dependencies.runtimeApi.ensureRunning();
    const runtimeInfoResult = await this.resolveRuntimeInfo();
    if (runtimeInfoResult.readFailed) {
      throw new Error(LOCAL_AI_PROXY_RUNTIME_STATUS_UNAVAILABLE_ERROR);
    }
    if (!hasAvailableLocalAiProxyRuntime(runtimeInfoResult.info)) {
      throw new Error(LOCAL_AI_PROXY_RUNTIME_UNAVAILABLE_ERROR);
    }
    if (!this.dependencies.runtimeApi.testLocalApiProxyRoute) {
      throw new Error(LOCAL_AI_PROXY_RUNTIME_UNAVAILABLE_ERROR);
    }

    const result = await this.dependencies.runtimeApi.testLocalApiProxyRoute(routeId.trim());
    if (!result) {
      throw new Error(LOCAL_AI_PROXY_RUNTIME_UNAVAILABLE_ERROR);
    }

    return result;
  }
}

function createDefaultDependencies(
  overrides: LocalApiProxyProviderCenterServiceOverrides = {},
): LocalApiProxyProviderCenterServiceDependencies {
  const routingDefaults = createLocalApiProxyProviderRoutingCatalogService({
    storageApi: overrides.storageApi || {
      getStorageInfo: async () => null,
      listKeys: async () => ({ keys: [] }),
      getText: async () => ({ value: null }),
      putText: async () => undefined,
      delete: async () => undefined,
    },
    now: overrides.now,
  });

  return {
    providerRoutingApi: {
      listProviderRoutingRecords:
        overrides.providerRoutingApi?.listProviderRoutingRecords
        ?? routingDefaults.listProviderRoutingRecords.bind(routingDefaults),
      saveProviderRoutingRecord:
        overrides.providerRoutingApi?.saveProviderRoutingRecord
        ?? routingDefaults.saveProviderRoutingRecord.bind(routingDefaults),
      deleteProviderRoutingRecord:
        overrides.providerRoutingApi?.deleteProviderRoutingRecord
        ?? routingDefaults.deleteProviderRoutingRecord.bind(routingDefaults),
    },
    runtimeApi: {
      getInfo:
        overrides.runtimeApi?.getInfo
        ?? (async () => null),
      ensureRunning:
        overrides.runtimeApi?.ensureRunning
        ?? (async () => undefined),
      testLocalApiProxyRoute: overrides.runtimeApi?.testLocalApiProxyRoute,
    },
    applyApi: overrides.applyApi
      ? {
          listTargets:
            overrides.applyApi.listTargets
            ?? (async () => []),
          getTarget:
            overrides.applyApi.getTarget
            ?? (async () => null),
          applyProjection:
            overrides.applyApi.applyProjection
            ?? (async () => undefined),
        }
      : undefined,
  };
}

export function createLocalApiProxyProviderCenterService(
  overrides: LocalApiProxyProviderCenterServiceOverrides = {},
) {
  return new LocalApiProxyProviderCenterService(
    createDefaultDependencies(overrides),
  );
}
