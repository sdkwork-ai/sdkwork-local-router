export interface LocalApiProxyLogsWorkspaceRuntimeEvidence {
  lifecycle?: string | null;
  observabilityDbPath?: string | null;
  snapshotPath?: string | null;
  logPath?: string | null;
}

export interface LocalApiProxyLogsWorkspaceRuntimeSummary {
  lifecycle: string;
  observabilityDbPath: string | null;
  snapshotPath: string | null;
  logPath: string | null;
}

export interface LocalApiProxyLogsWorkspaceServiceDependencies<
  TRequestQuery,
  TRequestResult,
  TMessageQuery,
  TMessageResult,
  TCaptureSettings,
> {
  listRequestLogs(query: TRequestQuery): Promise<TRequestResult>;
  listMessageLogs(query: TMessageQuery): Promise<TMessageResult>;
  getCaptureSettings(): Promise<TCaptureSettings>;
  updateCaptureSettings(enabled: boolean): Promise<TCaptureSettings>;
  getRuntimeEvidence(): Promise<LocalApiProxyLogsWorkspaceRuntimeEvidence | null | undefined>;
  normalizeRequestQuery?: (query: TRequestQuery) => TRequestQuery;
  normalizeMessageQuery?: (query: TMessageQuery) => TMessageQuery;
}

function normalizeOptionalText(value: string | null | undefined) {
  const normalized = value?.trim();
  return normalized ? normalized : undefined;
}

function normalizeNullableText(value: string | null | undefined) {
  const normalized = value?.trim();
  return normalized ? normalized : null;
}

export function resolveLocalApiProxyLogsWorkspaceRuntimeSummary(
  evidence: LocalApiProxyLogsWorkspaceRuntimeEvidence | null | undefined,
): LocalApiProxyLogsWorkspaceRuntimeSummary {
  return {
    lifecycle: normalizeOptionalText(evidence?.lifecycle) ?? "unavailable",
    observabilityDbPath: normalizeNullableText(evidence?.observabilityDbPath),
    snapshotPath: normalizeNullableText(evidence?.snapshotPath),
    logPath: normalizeNullableText(evidence?.logPath),
  };
}

export function createLocalApiProxyLogsWorkspaceService<
  TRequestQuery,
  TRequestResult,
  TMessageQuery,
  TMessageResult,
  TCaptureSettings,
>(
  dependencies: LocalApiProxyLogsWorkspaceServiceDependencies<
    TRequestQuery,
    TRequestResult,
    TMessageQuery,
    TMessageResult,
    TCaptureSettings
  >,
) {
  return {
    listRequestLogs(query: TRequestQuery): Promise<TRequestResult> {
      return dependencies.listRequestLogs(
        dependencies.normalizeRequestQuery ? dependencies.normalizeRequestQuery(query) : query,
      );
    },

    listMessageLogs(query: TMessageQuery): Promise<TMessageResult> {
      return dependencies.listMessageLogs(
        dependencies.normalizeMessageQuery ? dependencies.normalizeMessageQuery(query) : query,
      );
    },

    getMessageCaptureSettings(): Promise<TCaptureSettings> {
      return dependencies.getCaptureSettings();
    },

    updateMessageCaptureSettings(enabled: boolean): Promise<TCaptureSettings> {
      return dependencies.updateCaptureSettings(enabled);
    },

    async getRuntimeSummary(): Promise<LocalApiProxyLogsWorkspaceRuntimeSummary> {
      return resolveLocalApiProxyLogsWorkspaceRuntimeSummary(
        await dependencies.getRuntimeEvidence(),
      );
    },
  };
}
