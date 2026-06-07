import type {
  LocalApiClientProtocol,
  LocalApiUpstreamProtocol,
} from "./localApiProxyTypes.ts";

export type LocalApiProxyProviderRouteClientProtocol = Extract<
  LocalApiClientProtocol,
  "openai-compatible" | "anthropic" | "gemini"
>;

export type LocalApiProxyProviderRouteUpstreamProtocol = Extract<
  LocalApiUpstreamProtocol,
  "openai-compatible" | "anthropic" | "gemini" | "ollama" | "azure-openai" | "openrouter" | "sdkwork"
>;

export type LocalApiProxyProviderRouteManagedBy = "system-default" | "user";

export interface LocalApiProxyProviderRouteModelRecord {
  id: string;
  name: string;
}

export interface LocalApiProxyProviderRouteRecord {
  id: string;
  schemaVersion: 1;
  name: string;
  enabled: boolean;
  isDefault: boolean;
  managedBy: LocalApiProxyProviderRouteManagedBy;
  clientProtocol: LocalApiProxyProviderRouteClientProtocol;
  upstreamProtocol: LocalApiProxyProviderRouteUpstreamProtocol;
  providerId: string;
  upstreamBaseUrl: string;
  apiKey: string;
  defaultModelId: string;
  reasoningModelId?: string;
  embeddingModelId?: string;
  models: LocalApiProxyProviderRouteModelRecord[];
  notes?: string;
  exposeTo: string[];
}

export interface LocalApiProxyProviderRouteModelState {
  defaultModelId: string;
  reasoningModelId?: string;
  embeddingModelId?: string;
  models: LocalApiProxyProviderRouteModelRecord[];
}
