import { describe, expect, expectTypeOf, it } from "vitest";
import type {
  LocalApiProxyCaptureSettingsRecord,
  LocalApiProxyConfig,
  LocalApiProxyControlRepository,
  LocalApiProxyMessageLogRecord,
  LocalApiProxyMessageLogsQuery,
  LocalApiProxyObservabilityRepository,
  LocalApiProxyPaginatedResult,
  LocalApiProxyProbeRecord,
  LocalApiProxyRequestLogRecord,
  LocalApiProxyRequestLogsQuery,
  LocalApiProxyRuntimeEventRecord,
  LocalApiProxyRuntimeSettings,
  LocalApiProxyRoute,
} from "../src";

describe("@sdkwork/local-api-proxy repository ports", () => {
  it("declares a dialect-agnostic control repository contract", async () => {
    expectTypeOf<LocalApiProxyControlRepository["loadConfig"]>().toEqualTypeOf<
      () => Promise<LocalApiProxyConfig | null>
    >();
    expectTypeOf<LocalApiProxyControlRepository["listRoutes"]>().toEqualTypeOf<
      () => Promise<LocalApiProxyRoute[]>
    >();
    expectTypeOf<LocalApiProxyControlRepository["saveRuntimeSettings"]>().toEqualTypeOf<
      (runtime: LocalApiProxyRuntimeSettings) => Promise<void>
    >();

    const repository: LocalApiProxyControlRepository = {
      async deleteRoute() {},
      async listRoutes() {
        return [];
      },
      async loadConfig() {
        return null;
      },
      async loadRuntimeSettings() {
        return null;
      },
      async saveConfig() {},
      async saveRuntimeSettings() {},
      async upsertRoute() {},
    };

    await expect(repository.loadConfig()).resolves.toBeNull();
    await expect(repository.listRoutes()).resolves.toEqual([]);
  });

  it("declares an observability repository contract for probes, logs, and runtime events", async () => {
    expectTypeOf<LocalApiProxyObservabilityRepository["listRequestLogs"]>().toEqualTypeOf<
      (
        query: LocalApiProxyRequestLogsQuery,
      ) => Promise<LocalApiProxyPaginatedResult<LocalApiProxyRequestLogRecord>>
    >();
    expectTypeOf<LocalApiProxyObservabilityRepository["listMessageLogs"]>().toEqualTypeOf<
      (
        query: LocalApiProxyMessageLogsQuery,
      ) => Promise<LocalApiProxyPaginatedResult<LocalApiProxyMessageLogRecord>>
    >();
    expectTypeOf<LocalApiProxyObservabilityRepository["recordProbeResult"]>().toEqualTypeOf<
      (record: LocalApiProxyProbeRecord) => Promise<void>
    >();
    expectTypeOf<LocalApiProxyObservabilityRepository["appendRuntimeEvent"]>().toEqualTypeOf<
      (record: LocalApiProxyRuntimeEventRecord) => Promise<void>
    >();
    expectTypeOf<LocalApiProxyObservabilityRepository["updateCaptureSettings"]>().toEqualTypeOf<
      (
        settings: LocalApiProxyCaptureSettingsRecord,
      ) => Promise<LocalApiProxyCaptureSettingsRecord>
    >();

    const repository: LocalApiProxyObservabilityRepository = {
      async appendRuntimeEvent() {},
      async getCaptureSettings() {
        return {
          enabled: false,
          redactHeaders: ["authorization"],
          settingsId: "capture",
          storeMessageContent: false,
        };
      },
      async listMessageLogs() {
        return {
          items: [],
          nextCursor: undefined,
          total: 0,
        };
      },
      async listRequestLogs() {
        return {
          items: [],
          nextCursor: undefined,
          total: 0,
        };
      },
      async recordProbeResult() {},
      async updateCaptureSettings(settings) {
        return settings;
      },
    };

    await expect(repository.listRequestLogs({ limit: 20 })).resolves.toEqual({
      items: [],
      nextCursor: undefined,
      total: 0,
    });
    await expect(repository.getCaptureSettings()).resolves.toMatchObject({
      enabled: false,
      settingsId: "capture",
    });
  });
});
