import { describe, expect, it } from "vitest";

import { createLocalApiProxyLogsService } from "../src";

interface TestRequestQuery {
  page?: number;
  page_size?: number;
  q?: string;
  providerId?: string;
  modelId?: string;
  routeId?: string;
  status?: unknown;
}

interface TestMessageQuery {
  page?: number;
  page_size?: number;
  q?: string;
  providerId?: string;
  modelId?: string;
  routeId?: string;
}

interface TestCaptureSettings {
  enabled: boolean;
  updatedAt: number | null;
}

describe("@sdkwork/local-api-proxy logs service", () => {
  it("normalizes list queries to standard page_size and q query names", async () => {
    const calls: Array<
      | { kind: "requests"; query: TestRequestQuery }
      | { kind: "messages"; query: TestMessageQuery }
    > = [];

    const service = createLocalApiProxyLogsService({
      listRequestLogs: async (query: TestRequestQuery) => {
        calls.push({ kind: "requests", query });
        return {
          hasMore: false,
          items: [],
          page: 1,
          page_size: 20,
          total: 0,
        };
      },
      listMessageLogs: async (query: TestMessageQuery) => {
        calls.push({ kind: "messages", query });
        return {
          hasMore: false,
          items: [],
          page: 1,
          page_size: 20,
          total: 0,
        };
      },
      getCaptureSettings: async (): Promise<TestCaptureSettings> => ({
        enabled: false,
        updatedAt: null,
      }),
      updateCaptureSettings: async (enabled): Promise<TestCaptureSettings> => ({
        enabled,
        updatedAt: 1,
      }),
      getRuntimeEvidence: async () => ({
        lifecycle: "running",
        logPath: "D:/logs/local-api-proxy.log",
      }),
    });

    await service.listRequestLogs({
      page: 0,
      page_size: 999,
      providerId: " openai ",
      q: "  openai  ",
      status: "all",
    });
    await service.listMessageLogs({
      page: -1,
      page_size: 0,
      providerId: " openai ",
      q: " summarize ",
    });

    expect(calls).toEqual([
      {
        kind: "requests",
        query: {
          page: 1,
          page_size: 100,
          providerId: "openai",
          q: "openai",
        },
      },
      {
        kind: "messages",
        query: {
          page: 1,
          page_size: 20,
          providerId: "openai",
          q: "summarize",
        },
      },
    ]);
  });
});
