import type {
  LocalApiProxyProviderRouteClientProtocol,
  LocalApiProxyProviderRouteManagedBy,
  LocalApiProxyProviderRouteModelRecord,
  LocalApiProxyProviderRouteRecord,
  LocalApiProxyProviderRouteUpstreamProtocol,
} from "./localApiProxyProviderRoute.ts";
import type { LocalApiProxyProviderRuntimeConfig } from "./localApiProxyProviderRuntimeConfig.ts";

export interface LocalApiProxyProviderChannelDefinition {
  id: string;
  name: string;
  vendor: string;
  description: string;
  modelFamily: string;
}

export type LocalApiProxyConfiguredProviderStatus =
  | "active"
  | "warning"
  | "disabled"
  | "expired";

export interface LocalApiProxyConfiguredProviderUsage {
  requestCount: number;
  tokenCount: number;
  spendUsd: number;
  period: "24h" | "7d" | "30d";
}

export interface LocalApiProxyConfiguredProviderModel {
  id: string;
  name: string;
}

export interface LocalApiProxyConfiguredProvider {
  id: string;
  channelId: string;
  name: string;
  apiKey: string;
  groupId: string;
  usage: LocalApiProxyConfiguredProviderUsage;
  expiresAt: string | null;
  status: LocalApiProxyConfiguredProviderStatus;
  createdAt: string | null;
  baseUrl: string;
  models: LocalApiProxyConfiguredProviderModel[];
  notes?: string;
  canCopyApiKey?: boolean;
  credentialReference?: string;
  clientProtocol?: LocalApiProxyProviderRouteClientProtocol;
  upstreamProtocol?: LocalApiProxyProviderRouteUpstreamProtocol;
  managedBy?: LocalApiProxyProviderRouteManagedBy;
  enabled?: boolean;
  isDefault?: boolean;
  defaultModelId?: string;
}

export interface LocalApiProxyProviderChannel
  extends LocalApiProxyProviderChannelDefinition {
  providerCount: number;
  activeProviderCount: number;
  warningProviderCount: number;
  disabledProviderCount: number;
}

export interface LocalApiProxyProviderRoutingDraft {
  presetId?: string;
  name: string;
  providerId: string;
  clientProtocol?: LocalApiProxyProviderRouteClientProtocol;
  upstreamProtocol?: LocalApiProxyProviderRouteUpstreamProtocol;
  upstreamBaseUrl?: string;
  baseUrl?: string;
  apiKey: string;
  enabled?: boolean;
  isDefault?: boolean;
  managedBy?: LocalApiProxyProviderRouteManagedBy;
  defaultModelId: string;
  reasoningModelId?: string;
  embeddingModelId?: string;
  models: LocalApiProxyProviderRouteModelRecord[];
  notes?: string;
  exposeTo?: string[];
  config?: Partial<LocalApiProxyProviderRuntimeConfig>;
}

export interface LocalApiProxyProviderRoutingDraftInput
  extends LocalApiProxyProviderRoutingDraft {
  channelId?: string;
}

export interface LocalApiProxyProviderRoutingRecord
  extends LocalApiProxyProviderRouteRecord {
  presetId?: string;
  baseUrl: string;
  config: LocalApiProxyProviderRuntimeConfig;
  createdAt: number;
  updatedAt: number;
}
