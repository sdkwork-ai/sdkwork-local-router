import {
  createLocalApiProxyLogsWorkspaceService,
  type LocalApiProxyLogsWorkspaceRuntimeEvidence,
} from "../services/localApiProxyLogsWorkspaceService.ts";

interface LocalApiProxyRequestQueryShape {
  page?: number;
  page_size?: number;
  q?: string;
  providerId?: string;
  modelId?: string;
  routeId?: string;
  status?: unknown;
}

interface LocalApiProxyMessageQueryShape {
  page?: number;
  page_size?: number;
  q?: string;
  providerId?: string;
  modelId?: string;
  routeId?: string;
}

export interface CreateLocalApiProxyLogsServiceDependencies<
  RequestQuery extends LocalApiProxyRequestQueryShape,
  RequestPage,
  MessageQuery extends LocalApiProxyMessageQueryShape,
  MessagePage,
  CaptureSettings,
> {
  listRequestLogs(query: RequestQuery): Promise<RequestPage>;
  listMessageLogs(query: MessageQuery): Promise<MessagePage>;
  getCaptureSettings(): Promise<CaptureSettings>;
  updateCaptureSettings(enabled: boolean): Promise<CaptureSettings>;
  getRuntimeEvidence(): Promise<LocalApiProxyLogsWorkspaceRuntimeEvidence | null | undefined>;
}

function normalizePage(value: number | undefined, fallback: number) {
  return Number.isFinite(value) && Number(value) > 0
    ? Math.round(Number(value))
    : fallback;
}

function normalizePageSize(value: number | undefined, fallback: number) {
  const normalized = normalizePage(value, fallback);
  return Math.max(1, Math.min(100, normalized));
}

function normalizeOptionalText(value: string | null | undefined) {
  const normalized = value?.trim();
  return normalized ? normalized : undefined;
}

function normalizeStatus(value: unknown) {
  return value === "succeeded" || value === "failed" ? value : undefined;
}

function compactObject<T extends object>(value: T): T {
  return Object.fromEntries(
    Object.entries(value as Record<string, unknown>).filter(([, entry]) => entry !== undefined),
  ) as T;
}

export function createLocalApiProxyLogsService<
  RequestQuery extends LocalApiProxyRequestQueryShape,
  RequestPage,
  MessageQuery extends LocalApiProxyMessageQueryShape,
  MessagePage,
  CaptureSettings,
>(
  dependencies: CreateLocalApiProxyLogsServiceDependencies<
    RequestQuery,
    RequestPage,
    MessageQuery,
    MessagePage,
    CaptureSettings
  >,
) {
  return createLocalApiProxyLogsWorkspaceService<
    RequestQuery,
    RequestPage,
    MessageQuery,
    MessagePage,
    CaptureSettings
  >({
    listRequestLogs: (query) => dependencies.listRequestLogs(query),
    listMessageLogs: (query) => dependencies.listMessageLogs(query),
    getCaptureSettings: () => dependencies.getCaptureSettings(),
    updateCaptureSettings: (enabled) =>
      dependencies.updateCaptureSettings(enabled),
    getRuntimeEvidence: () => dependencies.getRuntimeEvidence(),
    normalizeRequestQuery: (query) =>
      compactObject({
        page: normalizePage(query.page as number | undefined, 1),
        page_size: normalizePageSize(query.page_size as number | undefined, 20),
        q: normalizeOptionalText(query.q as string | undefined),
        providerId: normalizeOptionalText(query.providerId as string | undefined),
        modelId: normalizeOptionalText(query.modelId as string | undefined),
        routeId: normalizeOptionalText(query.routeId as string | undefined),
        status: normalizeStatus(query.status),
      }) as unknown as RequestQuery,
    normalizeMessageQuery: (query) =>
      compactObject({
        page: normalizePage(query.page as number | undefined, 1),
        page_size: normalizePageSize(query.page_size as number | undefined, 20),
        q: normalizeOptionalText(query.q as string | undefined),
        providerId: normalizeOptionalText(query.providerId as string | undefined),
        modelId: normalizeOptionalText(query.modelId as string | undefined),
        routeId: normalizeOptionalText(query.routeId as string | undefined),
      }) as unknown as MessageQuery,
  });
}
