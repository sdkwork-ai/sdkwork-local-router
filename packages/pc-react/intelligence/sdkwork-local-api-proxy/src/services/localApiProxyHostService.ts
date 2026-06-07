import type { LocalApiProxyConfig } from "../types/localApiProxyTypes.ts";
import type { LocalApiProxyHostPort } from "../types/localApiProxyHost.ts";

export interface LocalApiProxyRuntimeSnapshot {
  config: LocalApiProxyConfig;
  runtimeStatus: Awaited<ReturnType<LocalApiProxyHostPort["getRuntimeStatus"]>>;
}

export function createLocalApiProxyHostService(hostPort: LocalApiProxyHostPort) {
  return {
    loadConfig() {
      return hostPort.loadConfig();
    },
    async saveConfig(config: LocalApiProxyConfig) {
      const validation = await hostPort.validateConfig(config);
      if (!validation.valid) {
        throw new Error(
          `Local API proxy config validation failed: ${validation.errors.join("; ") || "unknown error"}`,
        );
      }

      return hostPort.saveConfig(config);
    },
    validateConfig(config: LocalApiProxyConfig) {
      return hostPort.validateConfig(config);
    },
    startRuntime() {
      return hostPort.startRuntime();
    },
    stopRuntime() {
      return hostPort.stopRuntime();
    },
    restartRuntime() {
      return hostPort.restartRuntime();
    },
    getRuntimeStatus() {
      return hostPort.getRuntimeStatus();
    },
    probeRoute(request: Parameters<LocalApiProxyHostPort["probeRoute"]>[0]) {
      return hostPort.probeRoute(request);
    },
    listRequestLogs(query: Parameters<LocalApiProxyHostPort["listRequestLogs"]>[0]) {
      return hostPort.listRequestLogs(query);
    },
    listMessageLogs(query: Parameters<LocalApiProxyHostPort["listMessageLogs"]>[0]) {
      return hostPort.listMessageLogs(query);
    },
    updateCaptureSettings(settings: Parameters<LocalApiProxyHostPort["updateCaptureSettings"]>[0]) {
      return hostPort.updateCaptureSettings(settings);
    },
    subscribeRuntimeStateChanged(
      listener: Parameters<LocalApiProxyHostPort["subscribeRuntimeStateChanged"]>[0],
    ) {
      return hostPort.subscribeRuntimeStateChanged(listener);
    },
    subscribeRequestLogAppended(
      listener: Parameters<LocalApiProxyHostPort["subscribeRequestLogAppended"]>[0],
    ) {
      return hostPort.subscribeRequestLogAppended(listener);
    },
    async loadRuntimeSnapshot(): Promise<LocalApiProxyRuntimeSnapshot> {
      const [config, runtimeStatus] = await Promise.all([
        hostPort.loadConfig(),
        hostPort.getRuntimeStatus(),
      ]);

      return {
        config,
        runtimeStatus,
      };
    },
  };
}
