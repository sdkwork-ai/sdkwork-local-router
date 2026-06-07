import type {
  LocalApiCapability,
  LocalApiProxyConfig,
} from "./localApiProxyTypes.ts";
import type {
  LocalApiProxyCaptureSettingsRecord,
  LocalApiProxyMessageLogRecord,
  LocalApiProxyMessageLogsQuery,
  LocalApiProxyPaginatedResult,
  LocalApiProxyProbeRecord,
  LocalApiProxyRequestLogRecord,
  LocalApiProxyRequestLogsQuery,
} from "../repository/localApiProxyRepository.ts";

export type LocalApiProxyHostCommandName =
  | "localApiProxy.loadConfig"
  | "localApiProxy.saveConfig"
  | "localApiProxy.validateConfig"
  | "localApiProxy.startRuntime"
  | "localApiProxy.stopRuntime"
  | "localApiProxy.restartRuntime"
  | "localApiProxy.getRuntimeStatus"
  | "localApiProxy.probeRoute"
  | "localApiProxy.listRequestLogs"
  | "localApiProxy.listMessageLogs"
  | "localApiProxy.updateCaptureSettings";

export type LocalApiProxyHostEventName =
  | "localApiProxy.runtimeStateChanged"
  | "localApiProxy.routeProbeCompleted"
  | "localApiProxy.configReloaded"
  | "localApiProxy.requestLogAppended"
  | "localApiProxy.runtimeDegraded";

export type LocalApiProxyRuntimeState =
  | "stopped"
  | "starting"
  | "running"
  | "degraded"
  | "failed";

export interface LocalApiProxyValidationResult {
  valid: boolean;
  errors: string[];
  warnings: string[];
}

export interface LocalApiProxyHostRuntimeStatus {
  state: LocalApiProxyRuntimeState;
  activeRouteIds: string[];
  bindHost: string;
  bindPort: number;
  publicBaseUrl: string;
  startedAtMs?: number;
  errorSummary?: string;
}

export type LocalApiProxyEventUnsubscribe = () => void | Promise<void>;

export interface LocalApiProxyHostPort {
  loadConfig(): Promise<LocalApiProxyConfig>;
  saveConfig(config: LocalApiProxyConfig): Promise<LocalApiProxyConfig>;
  validateConfig(config: LocalApiProxyConfig): Promise<LocalApiProxyValidationResult>;
  startRuntime(): Promise<LocalApiProxyHostRuntimeStatus>;
  stopRuntime(): Promise<LocalApiProxyHostRuntimeStatus>;
  restartRuntime(): Promise<LocalApiProxyHostRuntimeStatus>;
  getRuntimeStatus(): Promise<LocalApiProxyHostRuntimeStatus>;
  probeRoute(request: {
    routeId: string;
    capability?: LocalApiCapability;
    operationId?: string;
  }): Promise<LocalApiProxyProbeRecord | null>;
  listRequestLogs(
    query: LocalApiProxyRequestLogsQuery,
  ): Promise<LocalApiProxyPaginatedResult<LocalApiProxyRequestLogRecord>>;
  listMessageLogs(
    query: LocalApiProxyMessageLogsQuery,
  ): Promise<LocalApiProxyPaginatedResult<LocalApiProxyMessageLogRecord>>;
  updateCaptureSettings(
    settings: LocalApiProxyCaptureSettingsRecord,
  ): Promise<LocalApiProxyCaptureSettingsRecord>;
  subscribeRuntimeStateChanged(
    listener: (status: LocalApiProxyHostRuntimeStatus) => void,
  ): Promise<LocalApiProxyEventUnsubscribe>;
  subscribeRequestLogAppended(
    listener: (record: LocalApiProxyRequestLogRecord) => void,
  ): Promise<LocalApiProxyEventUnsubscribe>;
}
