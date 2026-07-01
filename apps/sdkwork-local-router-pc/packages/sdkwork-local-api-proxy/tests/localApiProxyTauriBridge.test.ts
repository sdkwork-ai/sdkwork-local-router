import { describe, expect, it, vi } from "vitest";
import type {
  LocalApiProxyHostPort,
  LocalApiProxyTauriBridgeOptions,
} from "../src";
import {
  LOCAL_API_PROXY_TAURI_COMMANDS,
  LOCAL_API_PROXY_TAURI_EVENTS,
  createDefaultLocalApiProxyConfig,
  createLocalApiProxyHostService,
  createLocalApiProxyTauriBridge,
} from "../src";

describe("@sdkwork/local-api-proxy tauri bridge", () => {
  it("maps the canonical tauri commands and events through the bridge adapter", async () => {
    const config = createDefaultLocalApiProxyConfig({
      storage: {
        dialect: "sqlite",
        sqlitePath: "C:/sdkwork/data/local-api-proxy.db",
      },
    });
    const invokeMock = vi.fn(async (command: string, args?: Record<string, unknown>) => {
      switch (command) {
        case LOCAL_API_PROXY_TAURI_COMMANDS.loadConfig:
          return config;
        case LOCAL_API_PROXY_TAURI_COMMANDS.validateConfig:
          return {
            errors: [],
            valid: true,
            warnings: [],
          };
        case LOCAL_API_PROXY_TAURI_COMMANDS.getRuntimeStatus:
        case LOCAL_API_PROXY_TAURI_COMMANDS.startRuntime:
          return {
            activeRouteIds: [],
            bindHost: "127.0.0.1",
            bindPort: 21281,
            publicBaseUrl: "http://127.0.0.1:21281",
            state: "running",
          };
        case LOCAL_API_PROXY_TAURI_COMMANDS.listRequestLogs:
        case LOCAL_API_PROXY_TAURI_COMMANDS.listMessageLogs:
          return {
            items: [],
            nextCursor: undefined,
            total: 0,
          };
        case LOCAL_API_PROXY_TAURI_COMMANDS.updateCaptureSettings:
          return args?.settings;
        case LOCAL_API_PROXY_TAURI_COMMANDS.saveConfig:
          return args?.config;
        default:
          return null;
      }
    });
    const listenMock = vi.fn(
      async (
        event: string,
        handler: (payload: {
          activeRouteIds: string[];
          bindHost: string;
          bindPort: number;
          publicBaseUrl: string;
          state: "running";
        }) => void,
      ) => {
        handler({
          activeRouteIds: [],
          bindHost: "127.0.0.1",
          bindPort: 21281,
          publicBaseUrl: "http://127.0.0.1:21281",
          state: "running",
        });
        return () => {};
      },
    );
    const invoke: LocalApiProxyTauriBridgeOptions["invoke"] = async <TResult>(
      command: string,
      args?: Record<string, unknown>,
    ) => invokeMock(command, args) as Promise<TResult>;
    const listen: LocalApiProxyTauriBridgeOptions["listen"] = async <TPayload>(
      event: string,
      handler: (payload: TPayload) => void,
    ) => listenMock(
      event,
      handler as (payload: {
        activeRouteIds: string[];
        bindHost: string;
        bindPort: number;
        publicBaseUrl: string;
        state: "running";
      }) => void,
    );

    const bridge = createLocalApiProxyTauriBridge({ invoke, listen });

    await expect(bridge.loadConfig()).resolves.toMatchObject({
      mode: "desktop-local",
    });
    await expect(bridge.validateConfig(config)).resolves.toEqual({
      errors: [],
      valid: true,
      warnings: [],
    });
    await expect(bridge.startRuntime()).resolves.toMatchObject({
      state: "running",
    });
    await expect(bridge.getRuntimeStatus()).resolves.toMatchObject({
      state: "running",
    });
    await expect(bridge.listRequestLogs({ limit: 20 })).resolves.toEqual({
      items: [],
      nextCursor: undefined,
      total: 0,
    });
    await expect(
      bridge.updateCaptureSettings({
        enabled: false,
        redactHeaders: ["authorization"],
        settingsId: "capture",
        storeMessageContent: false,
      }),
    ).resolves.toEqual({
      enabled: false,
      redactHeaders: ["authorization"],
      settingsId: "capture",
      storeMessageContent: false,
    });

    const unsubscribe = await bridge.subscribeRuntimeStateChanged(() => {});

    expect(typeof unsubscribe).toBe("function");
    expect(invokeMock).toHaveBeenCalledWith(LOCAL_API_PROXY_TAURI_COMMANDS.loadConfig, undefined);
    expect(invokeMock).toHaveBeenCalledWith(LOCAL_API_PROXY_TAURI_COMMANDS.validateConfig, {
      config,
    });
    expect(invokeMock).toHaveBeenCalledWith(LOCAL_API_PROXY_TAURI_COMMANDS.listRequestLogs, {
      query: { limit: 20 },
    });
    expect(listenMock).toHaveBeenCalledWith(
      LOCAL_API_PROXY_TAURI_EVENTS.runtimeStateChanged,
      expect.any(Function),
    );
  });

  it("validates before save and composes runtime snapshots through the host service", async () => {
    const config = createDefaultLocalApiProxyConfig({
      storage: {
        dialect: "sqlite",
        sqlitePath: "C:/sdkwork/data/local-api-proxy.db",
      },
    });
    const hostPort: LocalApiProxyHostPort = {
      getRuntimeStatus: vi.fn(async () => ({
        activeRouteIds: [],
        bindHost: "127.0.0.1",
        bindPort: 21281,
        publicBaseUrl: "http://127.0.0.1:21281",
        state: "running" as const,
      })),
      listMessageLogs: vi.fn(async () => ({
        items: [],
        nextCursor: undefined,
        total: 0,
      })),
      probeRoute: vi.fn(async () => null),
      listRequestLogs: vi.fn(async () => ({
        items: [],
        nextCursor: undefined,
        total: 0,
      })),
      loadConfig: vi.fn(async () => config),
      restartRuntime: vi.fn(async () => ({
        activeRouteIds: [],
        bindHost: "127.0.0.1",
        bindPort: 21281,
        publicBaseUrl: "http://127.0.0.1:21281",
        state: "running" as const,
      })),
      saveConfig: vi.fn(async () => config),
      startRuntime: vi.fn(async () => ({
        activeRouteIds: [],
        bindHost: "127.0.0.1",
        bindPort: 21281,
        publicBaseUrl: "http://127.0.0.1:21281",
        state: "running" as const,
      })),
      stopRuntime: vi.fn(async () => ({
        activeRouteIds: [],
        bindHost: "127.0.0.1",
        bindPort: 21281,
        publicBaseUrl: "http://127.0.0.1:21281",
        state: "stopped" as const,
      })),
      subscribeRequestLogAppended: vi.fn(async () => () => {}),
      subscribeRuntimeStateChanged: vi.fn(async () => () => {}),
      updateCaptureSettings: vi.fn(async (settings) => settings),
      validateConfig: vi.fn(async () => ({
        errors: [],
        valid: true,
        warnings: [],
      })),
    };
    const service = createLocalApiProxyHostService(hostPort);

    await expect(service.saveConfig(config)).resolves.toEqual(config);
    await expect(service.loadRuntimeSnapshot()).resolves.toEqual({
      config,
      runtimeStatus: {
        activeRouteIds: [],
        bindHost: "127.0.0.1",
        bindPort: 21281,
        publicBaseUrl: "http://127.0.0.1:21281",
        state: "running",
      },
    });
  });
});
