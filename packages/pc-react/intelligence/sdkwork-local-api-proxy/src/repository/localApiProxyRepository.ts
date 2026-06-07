import type {
  LocalApiCapability,
  LocalApiProxyConfig,
  LocalApiProxyRuntimeSettings,
  LocalApiProxyRoute,
} from "../types/localApiProxyTypes.ts";

export interface LocalApiProxyPaginatedResult<TItem> {
  items: TItem[];
  nextCursor?: string;
  total: number;
}

export interface LocalApiProxyRequestLogsQuery {
  cursor?: string;
  limit?: number;
  routeId?: string;
  capability?: LocalApiCapability;
  operationId?: string;
  consumer?: string;
  status?: string;
}

export interface LocalApiProxyRequestLogRecord {
  requestId: string;
  traceId: string;
  routeId: string;
  capability: LocalApiCapability;
  operationId: string;
  consumer: string;
  status: string;
  streaming: boolean;
  latencyMs?: number;
  ttftMs?: number;
  inputTokens?: number;
  outputTokens?: number;
  totalTokens?: number;
  requestPreview?: string;
  responsePreview?: string;
  errorSummary?: string;
  createdAtMs: number;
}

export interface LocalApiProxyMessageLogsQuery {
  cursor?: string;
  limit?: number;
  requestId?: string;
  routeId?: string;
  role?: string;
}

export interface LocalApiProxyMessageLogRecord {
  messageId: string;
  requestId: string;
  role: string;
  contentPreview?: string;
  redactionState: string;
  createdAtMs: number;
}

export interface LocalApiProxyProbeRecord {
  probeId: string;
  routeId: string;
  capability?: LocalApiCapability;
  operationId?: string;
  status: string;
  latencyMs?: number;
  detailJson?: unknown;
  probedAtMs: number;
}

export interface LocalApiProxyRuntimeEventRecord {
  eventId: string;
  eventType: string;
  routeId?: string;
  status?: string;
  detailJson?: unknown;
  createdAtMs: number;
}

export interface LocalApiProxyCaptureSettingsRecord {
  settingsId: string;
  enabled: boolean;
  storeMessageContent: boolean;
  redactHeaders: string[];
  retentionDays?: number;
}

export interface LocalApiProxyControlRepository {
  loadConfig(): Promise<LocalApiProxyConfig | null>;
  saveConfig(config: LocalApiProxyConfig): Promise<void>;
  listRoutes(): Promise<LocalApiProxyRoute[]>;
  upsertRoute(route: LocalApiProxyRoute): Promise<void>;
  deleteRoute(routeId: string): Promise<void>;
  loadRuntimeSettings(): Promise<LocalApiProxyRuntimeSettings | null>;
  saveRuntimeSettings(runtime: LocalApiProxyRuntimeSettings): Promise<void>;
}

export interface LocalApiProxyObservabilityRepository {
  listRequestLogs(
    query: LocalApiProxyRequestLogsQuery,
  ): Promise<LocalApiProxyPaginatedResult<LocalApiProxyRequestLogRecord>>;
  listMessageLogs(
    query: LocalApiProxyMessageLogsQuery,
  ): Promise<LocalApiProxyPaginatedResult<LocalApiProxyMessageLogRecord>>;
  recordProbeResult(record: LocalApiProxyProbeRecord): Promise<void>;
  appendRuntimeEvent(record: LocalApiProxyRuntimeEventRecord): Promise<void>;
  getCaptureSettings(): Promise<LocalApiProxyCaptureSettingsRecord>;
  updateCaptureSettings(
    settings: LocalApiProxyCaptureSettingsRecord,
  ): Promise<LocalApiProxyCaptureSettingsRecord>;
}
