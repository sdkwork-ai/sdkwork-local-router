import type { LocalApiProxyObservabilityRepository } from "../repository/localApiProxyRepository.ts";
import type { LocalApiProxyHostPort } from "../types/localApiProxyHost.ts";

export function createLocalApiProxyObservabilityService(dependencies: {
  repository: Pick<LocalApiProxyObservabilityRepository, "listRequestLogs" | "listMessageLogs">;
  host: Pick<LocalApiProxyHostPort, "updateCaptureSettings">;
  defaultPageSize?: number;
}) {
  const defaultPageSize = dependencies.defaultPageSize ?? 50;

  return {
    listRequestLogs(
      query: Parameters<LocalApiProxyObservabilityRepository["listRequestLogs"]>[0],
    ) {
      return dependencies.repository.listRequestLogs({
        ...query,
        limit: query.limit ?? defaultPageSize,
      });
    },
    listMessageLogs(
      query: Parameters<LocalApiProxyObservabilityRepository["listMessageLogs"]>[0],
    ) {
      return dependencies.repository.listMessageLogs({
        ...query,
        limit: query.limit ?? defaultPageSize,
      });
    },
    updateCaptureSettings(
      settings: Parameters<LocalApiProxyHostPort["updateCaptureSettings"]>[0],
    ) {
      return dependencies.host.updateCaptureSettings(settings);
    },
  };
}
