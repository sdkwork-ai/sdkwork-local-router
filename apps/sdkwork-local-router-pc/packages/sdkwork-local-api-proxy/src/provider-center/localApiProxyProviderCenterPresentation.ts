import type { LocalApiProxyRouteHealth } from "../types/localApiProxyTypes.ts";
import type { LocalApiProxyProviderConfigRecord } from "./localApiProxyProviderCenterTypes.ts";

export interface LocalApiProxyProviderCenterPresentationLabels {
  health: Record<LocalApiProxyRouteHealth, string>;
  states: {
    notTested: string;
  };
  table: {
    totalTokensShort: string;
    inputTokensShort: string;
    outputTokensShort: string;
    cacheTokensShort: string;
  };
  testStatus: {
    passed: string;
    failed: string;
  };
}

export interface FormatLatestTestPresentationOptions {
  formatUpdatedAt?: (updatedAt: number) => string;
}

export function formatLocalApiProxyRouteUpdatedAt(updatedAt: number) {
  return Number.isFinite(updatedAt) && updatedAt > 0
    ? new Date(updatedAt).toLocaleString()
    : "--";
}

function formatCompactNumber(value?: number | null) {
  if (!Number.isFinite(value ?? NaN)) {
    return "--";
  }

  const normalized = Number(value);
  if (normalized >= 1_000_000) {
    return `${(normalized / 1_000_000).toFixed(1)}M`;
  }
  if (normalized >= 1_000) {
    return `${(normalized / 1_000).toFixed(1)}k`;
  }

  return `${normalized}`;
}

export function formatLocalApiProxyRouteLatency(value?: number | null) {
  return Number.isFinite(value ?? NaN) ? `${Math.round(Number(value))} ms` : "--";
}

type RouteRecordWithRuntime = LocalApiProxyProviderConfigRecord & {
  runtimeMetrics?: {
    health?: LocalApiProxyRouteHealth;
    totalTokens?: number;
    inputTokens?: number;
    outputTokens?: number;
    cacheTokens?: number;
  };
  latestTest?: {
    status: "passed" | "failed";
    latencyMs?: number | null;
    testedAt: number;
    error?: string | null;
  } | null;
};

export function resolveLocalApiProxyProviderRouteHealth(
  record: RouteRecordWithRuntime,
): LocalApiProxyRouteHealth {
  if (!record.enabled) {
    return "disabled";
  }

  return record.runtimeMetrics?.health || "degraded";
}

export function resolveLocalApiProxyRouteHealthToneClasses(
  health: LocalApiProxyRouteHealth,
) {
  switch (health) {
    case "healthy":
      return "border-emerald-200 bg-emerald-50 text-emerald-700 dark:border-emerald-500/30 dark:bg-emerald-500/10 dark:text-emerald-200";
    case "failed":
      return "border-rose-200 bg-rose-50 text-rose-700 dark:border-rose-500/30 dark:bg-rose-500/10 dark:text-rose-200";
    case "disabled":
      return "border-zinc-200 bg-zinc-100 text-zinc-600 dark:border-zinc-700 dark:bg-zinc-800 dark:text-zinc-300";
    default:
      return "border-amber-200 bg-amber-50 text-amber-700 dark:border-amber-500/30 dark:bg-amber-500/10 dark:text-amber-200";
  }
}

export function formatLocalApiProxyRouteHealthLabel(
  health: LocalApiProxyRouteHealth,
  labels: LocalApiProxyProviderCenterPresentationLabels["health"],
) {
  return labels[health] || health;
}

export function formatLocalApiProxyRouteUsageSummary(
  record: RouteRecordWithRuntime,
  labels: LocalApiProxyProviderCenterPresentationLabels,
) {
  const metrics = record.runtimeMetrics;
  if (!metrics) {
    return [labels.states.notTested];
  }

  return [
    `${labels.table.totalTokensShort} ${formatCompactNumber(metrics.totalTokens)}`,
    `${labels.table.inputTokensShort} ${formatCompactNumber(metrics.inputTokens)}`,
    `${labels.table.outputTokensShort} ${formatCompactNumber(metrics.outputTokens)}`,
    `${labels.table.cacheTokensShort} ${formatCompactNumber(metrics.cacheTokens)}`,
  ];
}

export function formatLocalApiProxyLatestTestPresentation(
  record: RouteRecordWithRuntime,
  labels: LocalApiProxyProviderCenterPresentationLabels,
  options: FormatLatestTestPresentationOptions = {},
) {
  if (!record.latestTest) {
    return {
      label: labels.states.notTested,
      tone: "border-zinc-200 bg-zinc-100 text-zinc-600 dark:border-zinc-700 dark:bg-zinc-800 dark:text-zinc-300",
      detail: "--",
      error: "",
    };
  }

  const passed = record.latestTest.status === "passed";
  const formatUpdatedAt =
    options.formatUpdatedAt || formatLocalApiProxyRouteUpdatedAt;

  return {
    label: passed ? labels.testStatus.passed : labels.testStatus.failed,
    tone: passed
      ? "border-emerald-200 bg-emerald-50 text-emerald-700 dark:border-emerald-500/30 dark:bg-emerald-500/10 dark:text-emerald-200"
      : "border-rose-200 bg-rose-50 text-rose-700 dark:border-rose-500/30 dark:bg-rose-500/10 dark:text-rose-200",
    detail: `${formatLocalApiProxyRouteLatency(record.latestTest.latencyMs)} / ${formatUpdatedAt(record.latestTest.testedAt)}`,
    error: record.latestTest.error || "",
  };
}
