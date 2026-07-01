export const LOCAL_API_PROXY_SCHEMA_VERSION = 1 as const;
export const LOCAL_API_PROXY_TABLE_PREFIX = "lap_" as const;
export const LOCAL_API_PROXY_DEFAULT_SQLITE_FILENAME = "local-api-proxy.db" as const;
export const LOCAL_API_PROXY_DEFAULT_POSTGRES_SCHEMA = "local_api_proxy" as const;
export const LOCAL_API_PROXY_DEFAULT_HOST = "127.0.0.1" as const;
export const LOCAL_API_PROXY_DEFAULT_PORT = 21281 as const;

export type LocalApiProxyMode = "desktop-local" | "server-managed";

export type LocalApiClientProtocol =
  | "openai-compatible"
  | "anthropic"
  | "gemini"
  | "custom-http";

export type LocalApiUpstreamProtocol =
  | "openai-compatible"
  | "anthropic"
  | "gemini"
  | "ollama"
  | "azure-openai"
  | "openrouter"
  | "sdkwork"
  | "custom-http";

export type LocalApiCapability =
  | "chat"
  | "response"
  | "embedding"
  | "model-catalog"
  | "file-transfer"
  | "batch"
  | "moderation"
  | "rerank"
  | "vector-store"
  | "custom";

export type LocalApiProxyManagedBy = "system" | "user";

export type LocalApiProxyModelRole =
  | "default"
  | "reasoning"
  | "embedding"
  | "vision"
  | "rerank"
  | "custom";

export type LocalApiProxyExposureTarget =
  | "sdkwork"
  | "desktop-clients"
  | "internal-sdk"
  | "custom";

export type LocalApiProxyHttpMethod =
  | "GET"
  | "POST"
  | "PUT"
  | "PATCH"
  | "DELETE";

export type LocalApiProxyStorageConfig =
  | {
      dialect: "sqlite";
      sqlitePath: string;
    }
  | {
      dialect: "postgresql";
      postgresUrl: string;
      schema?: string;
    };

export interface LocalApiProxyBind {
  host: string;
  port: number;
  publicBaseUrl: string;
}

export interface LocalApiProxyCaptureConfig {
  enabled: boolean;
  storeMessageContent: boolean;
  redactHeaders: string[];
  retentionDays?: number;
}

export interface ProxyUpstreamIdentity {
  providerId: string;
  protocolKind: LocalApiUpstreamProtocol;
  mirrorProtocolIdentity?: string;
  baseUrl: string;
  credentialRef?: string;
}

export interface RouteCapabilityBinding {
  capability: LocalApiCapability;
  enabled: boolean;
  operationSet: string[];
  streaming: boolean;
  timeoutMs?: number;
  pathOverride?: string;
  methodOverride?: LocalApiProxyHttpMethod;
  requestPolicyRef?: string;
  responsePolicyRef?: string;
}

export interface LocalApiProxyModelBinding {
  role: LocalApiProxyModelRole;
  modelId: string;
  capability?: LocalApiCapability;
  label?: string;
}

export interface LocalApiProxyExposure {
  target: LocalApiProxyExposureTarget;
  enabled: boolean;
  consumerId?: string;
  label?: string;
}

export interface LocalApiProxyRouteRuntimePolicy {
  retryCount?: number;
  timeoutMs?: number;
}

export interface LocalApiProxyRoute {
  id: string;
  name: string;
  enabled: boolean;
  managedBy: LocalApiProxyManagedBy;
  providerId: string;
  clientProtocol: LocalApiClientProtocol;
  upstreamProtocol: LocalApiUpstreamProtocol;
  upstream: ProxyUpstreamIdentity;
  capabilities: RouteCapabilityBinding[];
  modelBindings: LocalApiProxyModelBinding[];
  runtimePolicy?: LocalApiProxyRouteRuntimePolicy;
  exposures: LocalApiProxyExposure[];
  tags: string[];
  notes?: string;
}

export type LocalApiProxyRouteHealth =
  | "healthy"
  | "degraded"
  | "failed"
  | "disabled";

export interface LocalApiProxyDefaults {
  defaultRouteByCapability: Partial<Record<LocalApiCapability, string>>;
  defaultRouteByProtocol: Partial<Record<LocalApiClientProtocol, string>>;
}

export interface LocalApiProxyPolicies {
  request: Record<string, string>;
  response: Record<string, string>;
}

export interface LocalApiProxyRuntimeSettings {
  retryCount: number;
  cleanupIntervalMs: number;
  maxConcurrentRequests: number;
  startupProbeTimeoutMs: number;
}

export interface LocalApiProxyConfig {
  schemaVersion: number;
  mode: LocalApiProxyMode;
  bind: LocalApiProxyBind;
  storage: LocalApiProxyStorageConfig;
  capture: LocalApiProxyCaptureConfig;
  routes: LocalApiProxyRoute[];
  defaults: LocalApiProxyDefaults;
  policies: LocalApiProxyPolicies;
  runtime: LocalApiProxyRuntimeSettings;
}

export interface ProxyUpstreamIdentityDraft {
  providerId: string;
  protocolKind: LocalApiUpstreamProtocol;
  mirrorProtocolIdentity?: string;
  baseUrl: string;
  credentialRef?: string;
}

export interface RouteCapabilityBindingDraft {
  capability: LocalApiCapability;
  enabled?: boolean;
  operationSet?: string[];
  streaming?: boolean;
  timeoutMs?: number;
  pathOverride?: string;
  methodOverride?: LocalApiProxyHttpMethod | Lowercase<LocalApiProxyHttpMethod>;
  requestPolicyRef?: string;
  responsePolicyRef?: string;
}

export interface LocalApiProxyModelBindingDraft {
  role: LocalApiProxyModelRole;
  modelId: string;
  capability?: LocalApiCapability;
  label?: string;
}

export interface LocalApiProxyExposureDraft {
  target: LocalApiProxyExposureTarget;
  enabled?: boolean;
  consumerId?: string;
  label?: string;
}

export interface LocalApiProxyRouteDraft {
  id?: string;
  name?: string;
  enabled?: boolean;
  managedBy?: LocalApiProxyManagedBy;
  providerId: string;
  clientProtocol: LocalApiClientProtocol;
  upstreamProtocol: LocalApiUpstreamProtocol;
  upstream: ProxyUpstreamIdentityDraft;
  capabilities?: RouteCapabilityBindingDraft[];
  modelBindings?: LocalApiProxyModelBindingDraft[];
  runtimePolicy?: LocalApiProxyRouteRuntimePolicy;
  exposures?: LocalApiProxyExposureDraft[];
  tags?: string[];
  notes?: string;
}

export interface LocalApiProxyConfigDraft {
  schemaVersion?: number;
  mode?: LocalApiProxyMode;
  bind?: Partial<LocalApiProxyBind>;
  storage: LocalApiProxyStorageConfig;
  capture?: Partial<LocalApiProxyCaptureConfig>;
  routes?: LocalApiProxyRouteDraft[];
  defaults?: Partial<LocalApiProxyDefaults>;
  policies?: Partial<LocalApiProxyPolicies>;
  runtime?: Partial<LocalApiProxyRuntimeSettings>;
}
