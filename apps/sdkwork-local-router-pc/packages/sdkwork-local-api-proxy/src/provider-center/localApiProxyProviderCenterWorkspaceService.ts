import type {
  LocalApiProxyProviderCenterActionSupport,
  LocalApiProxyProviderConfigDraft,
  LocalApiProxyProviderConfigRecord,
} from "./localApiProxyProviderCenterTypes.ts";
import type {
  LocalApiProxyProviderImportResult,
  LocalApiProxyProviderImportSource,
} from "./localApiProxyProviderImportService.ts";

interface LocalApiProxyProviderCenterWorkspaceAgentLike {
  id: string;
  isDefault: boolean;
}

interface LocalApiProxyProviderCenterWorkspaceTargetLike {
  agents: LocalApiProxyProviderCenterWorkspaceAgentLike[];
}

interface LocalApiProxyProviderCenterWorkspaceInstanceLike {
  id: string;
  isDefault: boolean;
}

interface LocalApiProxyProviderCenterWorkspaceCenterApi<
  TApplyInstance extends LocalApiProxyProviderCenterWorkspaceInstanceLike,
  TApplyTarget extends LocalApiProxyProviderCenterWorkspaceTargetLike,
> {
  listProviderConfigs(): Promise<LocalApiProxyProviderConfigRecord[]>;
  getActionSupport(): Promise<LocalApiProxyProviderCenterActionSupport>;
  saveProviderConfig(
    input: LocalApiProxyProviderConfigDraft & { id?: string },
  ): Promise<LocalApiProxyProviderConfigRecord>;
  listApplyInstances(): Promise<TApplyInstance[]>;
  getInstanceApplyTarget(instanceId: string): Promise<TApplyTarget>;
  applyProviderConfig(input: {
    instanceId: string;
    config: LocalApiProxyProviderConfigRecord;
    agentIds?: string[];
  }): Promise<void>;
}

interface LocalApiProxyProviderCenterWorkspaceImportApi {
  importProviderConfigs(
    source: LocalApiProxyProviderImportSource,
  ): Promise<LocalApiProxyProviderImportResult>;
}

export interface LocalApiProxyProviderCenterWorkspaceDependencies<
  TApplyInstance extends LocalApiProxyProviderCenterWorkspaceInstanceLike,
  TApplyTarget extends LocalApiProxyProviderCenterWorkspaceTargetLike,
> {
  centerApi: LocalApiProxyProviderCenterWorkspaceCenterApi<TApplyInstance, TApplyTarget>;
  importApi: LocalApiProxyProviderCenterWorkspaceImportApi;
}

export interface LocalApiProxyProviderCenterOverview {
  records: LocalApiProxyProviderConfigRecord[];
  actionSupport: LocalApiProxyProviderCenterActionSupport;
}

export interface LocalApiProxyProviderCenterApplyInstancesState<
  TApplyInstance extends LocalApiProxyProviderCenterWorkspaceInstanceLike,
> {
  instances: TApplyInstance[];
  selectedInstanceId: string;
}

export interface LocalApiProxyProviderCenterApplyTargetState<
  TApplyTarget extends LocalApiProxyProviderCenterWorkspaceTargetLike,
> {
  instanceTarget: TApplyTarget | null;
  selectedAgentIds: string[];
}

export interface ImportLocalApiProxyProviderConfigsInput {
  source: LocalApiProxyProviderImportSource;
  existingRecords: LocalApiProxyProviderConfigRecord[];
}

export interface LocalApiProxyProviderCenterImportExecutionResult {
  result: LocalApiProxyProviderImportResult;
  savedRecordIds: string[];
  savedNames: string[];
}

function resolveSelectedInstanceId<
  TApplyInstance extends LocalApiProxyProviderCenterWorkspaceInstanceLike,
>(instances: TApplyInstance[]) {
  return instances.find((instance) => instance.isDefault)?.id || instances[0]?.id || "";
}

function resolveDefaultAgentIds<
  TApplyTarget extends LocalApiProxyProviderCenterWorkspaceTargetLike,
>(target: TApplyTarget | null) {
  return (target?.agents || [])
    .filter((agent) => agent.isDefault)
    .map((agent) => agent.id);
}

export function createLocalApiProxyProviderCenterWorkspaceService<
  TApplyInstance extends LocalApiProxyProviderCenterWorkspaceInstanceLike,
  TApplyTarget extends LocalApiProxyProviderCenterWorkspaceTargetLike,
>(
  dependencies: LocalApiProxyProviderCenterWorkspaceDependencies<TApplyInstance, TApplyTarget>,
) {
  return {
    async loadOverview(): Promise<LocalApiProxyProviderCenterOverview> {
      const [records, actionSupport] = await Promise.all([
        dependencies.centerApi.listProviderConfigs(),
        dependencies.centerApi.getActionSupport(),
      ]);

      return {
        records,
        actionSupport,
      };
    },

    async loadApplyInstances(): Promise<
      LocalApiProxyProviderCenterApplyInstancesState<TApplyInstance>
    > {
      const instances = await dependencies.centerApi.listApplyInstances();
      return {
        instances,
        selectedInstanceId: resolveSelectedInstanceId(instances),
      };
    },

    async loadApplyInstanceTarget(
      instanceId: string,
    ): Promise<LocalApiProxyProviderCenterApplyTargetState<TApplyTarget>> {
      const normalizedInstanceId = instanceId.trim();
      if (!normalizedInstanceId) {
        return {
          instanceTarget: null,
          selectedAgentIds: [],
        };
      }

      const instanceTarget = await dependencies.centerApi.getInstanceApplyTarget(
        normalizedInstanceId,
      );
      return {
        instanceTarget,
        selectedAgentIds: resolveDefaultAgentIds(instanceTarget),
      };
    },

    async importProviderConfigs(
      input: ImportLocalApiProxyProviderConfigsInput,
    ): Promise<LocalApiProxyProviderCenterImportExecutionResult> {
      const result = await dependencies.importApi.importProviderConfigs(input.source);
      const savedRecordIds: string[] = [];
      const savedNames: string[] = [];

      for (const imported of result.drafts) {
        const existingRecord = input.existingRecords.find(
          (record) =>
            record.managedBy === "user"
            && record.name === imported.draft.name,
        );
        const savedRecord = await dependencies.centerApi.saveProviderConfig({
          ...imported.draft,
          id: existingRecord?.id,
        });
        savedRecordIds.push(savedRecord.id);
        savedNames.push(savedRecord.name);
      }

      return {
        result,
        savedRecordIds,
        savedNames,
      };
    },
  };
}
