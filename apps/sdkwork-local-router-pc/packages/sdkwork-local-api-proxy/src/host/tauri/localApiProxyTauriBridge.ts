import type {
  LocalApiProxyCaptureSettingsRecord,
  LocalApiProxyMessageLogsQuery,
  LocalApiProxyPaginatedResult,
  LocalApiProxyProbeRecord,
  LocalApiProxyRequestLogRecord,
  LocalApiProxyRequestLogsQuery,
  LocalApiProxyMessageLogRecord,
} from "../../repository/localApiProxyRepository.ts";
import type {
  LocalApiProxyConfig,
  LocalApiCapability,
} from "../../types/localApiProxyTypes.ts";
import type {
  LocalApiProxyEventUnsubscribe,
  LocalApiProxyHostPort,
  LocalApiProxyHostRuntimeStatus,
  LocalApiProxyValidationResult,
} from "../../types/localApiProxyHost.ts";

export const LOCAL_API_PROXY_TAURI_COMMANDS = {
  loadConfig: "localApiProxy.loadConfig",
  saveConfig: "localApiProxy.saveConfig",
  validateConfig: "localApiProxy.validateConfig",
  startRuntime: "localApiProxy.startRuntime",
  stopRuntime: "localApiProxy.stopRuntime",
  restartRuntime: "localApiProxy.restartRuntime",
  getRuntimeStatus: "localApiProxy.getRuntimeStatus",
  probeRoute: "localApiProxy.probeRoute",
  listRequestLogs: "localApiProxy.listRequestLogs",
  listMessageLogs: "localApiProxy.listMessageLogs",
  updateCaptureSettings: "localApiProxy.updateCaptureSettings",
} as const;

export const LOCAL_API_PROXY_TAURI_EVENTS = {
  runtimeStateChanged: "localApiProxy.runtimeStateChanged",
  requestLogAppended: "localApiProxy.requestLogAppended",
} as const;

export interface LocalApiProxyTauriBridgeOptions {
  invoke<TResult>(command: string, args?: Record<string, unknown>): Promise<TResult>;
  listen<TPayload>(
    event: string,
    handler: (payload: TPayload) => void,
  ): Promise<LocalApiProxyEventUnsubscribe>;
}

export interface LocalApiProxyTauriBridge extends LocalApiProxyHostPort {}

export function createLocalApiProxyTauriBridge(
  options: LocalApiProxyTauriBridgeOptions,
): LocalApiProxyTauriBridge {
  return {
    loadConfig() {
      return options.invoke<LocalApiProxyConfig>(LOCAL_API_PROXY_TAURI_COMMANDS.loadConfig);
    },
    saveConfig(config) {
      return options.invoke<LocalApiProxyConfig>(LOCAL_API_PROXY_TAURI_COMMANDS.saveConfig, {
        config,
      });
    },
    validateConfig(config) {
      return options.invoke<LocalApiProxyValidationResult>(
        LOCAL_API_PROXY_TAURI_COMMANDS.validateConfig,
        { config },
      );
    },
    startRuntime() {
      return options.invoke<LocalApiProxyHostRuntimeStatus>(
        LOCAL_API_PROXY_TAURI_COMMANDS.startRuntime,
      );
    },
    stopRuntime() {
      return options.invoke<LocalApiProxyHostRuntimeStatus>(
        LOCAL_API_PROXY_TAURI_COMMANDS.stopRuntime,
      );
    },
    restartRuntime() {
      return options.invoke<LocalApiProxyHostRuntimeStatus>(
        LOCAL_API_PROXY_TAURI_COMMANDS.restartRuntime,
      );
    },
    getRuntimeStatus() {
      return options.invoke<LocalApiProxyHostRuntimeStatus>(
        LOCAL_API_PROXY_TAURI_COMMANDS.getRuntimeStatus,
      );
    },
    probeRoute(request: {
      routeId: string;
      capability?: LocalApiCapability;
      operationId?: string;
    }) {
      return options.invoke<LocalApiProxyProbeRecord | null>(
        LOCAL_API_PROXY_TAURI_COMMANDS.probeRoute,
        { request },
      );
    },
    listRequestLogs(query: LocalApiProxyRequestLogsQuery) {
      return options.invoke<LocalApiProxyPaginatedResult<LocalApiProxyRequestLogRecord>>(
        LOCAL_API_PROXY_TAURI_COMMANDS.listRequestLogs,
        { query },
      );
    },
    listMessageLogs(query: LocalApiProxyMessageLogsQuery) {
      return options.invoke<LocalApiProxyPaginatedResult<LocalApiProxyMessageLogRecord>>(
        LOCAL_API_PROXY_TAURI_COMMANDS.listMessageLogs,
        { query },
      );
    },
    updateCaptureSettings(settings: LocalApiProxyCaptureSettingsRecord) {
      return options.invoke<LocalApiProxyCaptureSettingsRecord>(
        LOCAL_API_PROXY_TAURI_COMMANDS.updateCaptureSettings,
        { settings },
      );
    },
    subscribeRuntimeStateChanged(listener) {
      return options.listen<LocalApiProxyHostRuntimeStatus>(
        LOCAL_API_PROXY_TAURI_EVENTS.runtimeStateChanged,
        listener,
      );
    },
    subscribeRequestLogAppended(listener) {
      return options.listen<LocalApiProxyRequestLogRecord>(
        LOCAL_API_PROXY_TAURI_EVENTS.requestLogAppended,
        listener,
      );
    },
  };
}
