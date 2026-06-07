import { describe, expect, it, vi } from "vitest";
import {
  createDefaultLocalApiProxyConfig,
  createLocalApiProxyConfigService,
  createLocalApiProxyObservabilityService,
  createLocalApiProxyRuntimeService,
} from "../src";

describe("@sdkwork/local-api-proxy services", () => {
  it("loads and saves config through the host boundary with validation", async () => {
    const config = createDefaultLocalApiProxyConfig({
      storage: {
        dialect: "sqlite",
        sqlitePath: "C:/sdkwork/data/local-api-proxy.db",
      },
    });
    const loadConfig = vi.fn(async () => config);
    const validateConfig = vi.fn(async () => ({
      errors: [],
      valid: true,
      warnings: [],
    }));
    const saveConfig = vi.fn(async () => config);

    const service = createLocalApiProxyConfigService({
      host: {
        loadConfig,
        saveConfig,
        validateConfig,
      },
    });

    await expect(service.load()).resolves.toEqual(config);
    await expect(service.save(config)).resolves.toEqual(config);
    expect(validateConfig).toHaveBeenCalledWith(config);
    expect(saveConfig).toHaveBeenCalledWith(config);
  });

  it("composes runtime status with repository routes into a package-owned summary view", async () => {
    const service = createLocalApiProxyRuntimeService({
      host: {
        getRuntimeStatus: vi.fn(async () => ({
          activeRouteIds: ["chat-primary"],
          bindHost: "127.0.0.1",
          bindPort: 21281,
          publicBaseUrl: "http://127.0.0.1:21281",
          state: "running" as const,
        })),
      },
      repository: {
        listRoutes: vi.fn(async () => [
          {
            capabilities: [],
            clientProtocol: "openai-compatible" as const,
            enabled: true,
            exposures: [],
            id: "chat-primary",
            managedBy: "system" as const,
            modelBindings: [],
            name: "Chat Primary",
            providerId: "openai",
            tags: ["chat"],
            upstream: {
              baseUrl: "https://api.openai.com/v1",
              protocolKind: "openai-compatible" as const,
              providerId: "openai",
            },
            upstreamProtocol: "openai-compatible" as const,
          },
          {
            capabilities: [],
            clientProtocol: "openai-compatible" as const,
            enabled: true,
            exposures: [],
            id: "embeddings-backoffice",
            managedBy: "user" as const,
            modelBindings: [],
            name: "Embeddings Backoffice",
            providerId: "openai",
            tags: ["embedding"],
            upstream: {
              baseUrl: "https://api.openai.com/v1",
              protocolKind: "openai-compatible" as const,
              providerId: "openai",
            },
            upstreamProtocol: "openai-compatible" as const,
          },
        ]),
      },
    });

    await expect(service.getRuntimeSummary()).resolves.toEqual({
      activeRouteCount: 1,
      publicBaseUrl: "http://127.0.0.1:21281",
      routeCount: 2,
      routes: [
        {
          active: true,
          enabled: true,
          id: "chat-primary",
          name: "Chat Primary",
        },
        {
          active: false,
          enabled: true,
          id: "embeddings-backoffice",
          name: "Embeddings Backoffice",
        },
      ],
      state: "running",
    });
  });

  it("applies stable pagination defaults when listing request logs", async () => {
    const listRequestLogs = vi.fn(async (query: { limit?: number; routeId?: string }) => ({
      items: [],
      nextCursor: undefined,
      total: query.limit ?? 0,
    }));

    const service = createLocalApiProxyObservabilityService({
      host: {
        updateCaptureSettings: vi.fn(async (settings) => settings),
      },
      repository: {
        listMessageLogs: vi.fn(async () => ({
          items: [],
          nextCursor: undefined,
          total: 0,
        })),
        listRequestLogs,
      },
    });

    await expect(service.listRequestLogs({ routeId: "chat-primary" })).resolves.toEqual({
      items: [],
      nextCursor: undefined,
      total: 50,
    });
    expect(listRequestLogs).toHaveBeenCalledWith({
      limit: 50,
      routeId: "chat-primary",
    });
  });
});
