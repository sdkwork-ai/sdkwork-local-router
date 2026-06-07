import type {
  LocalApiProxyProviderRoutingDraft,
  LocalApiProxyProviderRoutingRecord,
} from "../types/localApiProxyProviderCatalog.ts";

export type LocalApiProxyProviderConfigDraft = LocalApiProxyProviderRoutingDraft;
export type LocalApiProxyProviderConfigRecord = LocalApiProxyProviderRoutingRecord;

export interface LocalApiProxyProviderConfigPreset {
  id: string;
  label: string;
  description: string;
  draft: LocalApiProxyProviderConfigDraft;
}

export interface LocalApiProxyProviderApplyTargetRecord {
  id: string;
  name: string;
  kind: string;
  isDefault: boolean;
  writable: boolean;
  configFile?: string | null;
  detail?: string;
}

export interface LocalApiProxyProviderApplyAgentRecord {
  id: string;
  name: string;
  isDefault: boolean;
  primaryModel?: string;
}

export interface LocalApiProxyProviderApplyTargetDetail {
  target: LocalApiProxyProviderApplyTargetRecord;
  agents: LocalApiProxyProviderApplyAgentRecord[];
}

export interface LocalApiProxyProviderCenterActionSupportItem {
  available: boolean;
  reasonKey?:
    | "runtimeUnavailable"
    | "runtimeStatusUnavailable"
    | "quickApplyTargetsUnavailable"
    | "quickApplyInstanceUnavailable"
    | "quickApplyRequiresLoopback";
  reason?: string;
}

export interface LocalApiProxyProviderCenterActionSupport {
  quickApply: LocalApiProxyProviderCenterActionSupportItem;
  test: LocalApiProxyProviderCenterActionSupportItem;
}

export interface LocalApiProxyManagedProviderModel {
  id: string;
  name: string;
}

export interface LocalApiProxyManagedKernelProjectionSelection {
  defaultModelId: string;
  reasoningModelId?: string;
  embeddingModelId?: string;
}

export interface LocalApiProxyManagedKernelProjectionProvider {
  id: string;
  channelId: string;
  name: string;
  apiKey: string;
  baseUrl: string;
  models: LocalApiProxyManagedProviderModel[];
  notes?: string;
  config?: LocalApiProxyProviderConfigRecord["config"];
}

export interface LocalApiProxyManagedKernelProjection {
  sourceRoute: LocalApiProxyProviderConfigRecord;
  provider: LocalApiProxyManagedKernelProjectionProvider;
  selection: LocalApiProxyManagedKernelProjectionSelection;
}

export interface LocalApiProxyApplyProjectionInput {
  targetId: string;
  configFile: string;
  projection: LocalApiProxyManagedKernelProjection;
  agentIds?: string[];
}
