import { describe, expect, expectTypeOf, it } from "vitest";
import type {
  LocalApiCapability,
  LocalApiProxyConfig,
  LocalApiProxyMode,
  LocalApiProxyModelBinding,
  ProxyUpstreamIdentity,
  RouteCapabilityBinding,
} from "../src";
import {
  LOCAL_API_PROXY_DEFAULT_POSTGRES_SCHEMA,
  LOCAL_API_PROXY_DEFAULT_SQLITE_FILENAME,
  LOCAL_API_PROXY_TABLE_PREFIX,
  createDefaultLocalApiProxyConfig,
  normalizeLocalApiProxyConfig,
} from "../src";

describe("@sdkwork/local-api-proxy authority model", () => {
  it("declares the canonical mode, capability, route capability, model binding, and upstream types", () => {
    expectTypeOf<LocalApiProxyMode>().toEqualTypeOf<"desktop-local" | "server-managed">();

    expectTypeOf<LocalApiCapability>().toEqualTypeOf<
      | "chat"
      | "response"
      | "embedding"
      | "model-catalog"
      | "file-transfer"
      | "batch"
      | "moderation"
      | "rerank"
      | "vector-store"
      | "custom"
    >();

    expectTypeOf<RouteCapabilityBinding["capability"]>().toEqualTypeOf<LocalApiCapability>();
    expectTypeOf<LocalApiProxyModelBinding["role"]>().toEqualTypeOf<
      | "default"
      | "reasoning"
      | "embedding"
      | "vision"
      | "rerank"
      | "custom"
    >();
    expectTypeOf<ProxyUpstreamIdentity["protocolKind"]>().toEqualTypeOf<
      | "openai-compatible"
      | "anthropic"
      | "gemini"
      | "ollama"
      | "azure-openai"
      | "openrouter"
      | "sdkwork"
      | "custom-http"
    >();
  });

  it("creates the default desktop-local authority config with the canonical storage defaults", () => {
    const config = createDefaultLocalApiProxyConfig({
      storage: {
        dialect: "sqlite",
        sqlitePath: "C:/sdkwork/data/local-api-proxy.db",
      },
    });

    expect(LOCAL_API_PROXY_TABLE_PREFIX).toBe("lap_");
    expect(LOCAL_API_PROXY_DEFAULT_SQLITE_FILENAME).toBe("local-api-proxy.db");
    expect(config).toMatchObject({
      bind: {
        host: "127.0.0.1",
        port: 21281,
        publicBaseUrl: "http://127.0.0.1:21281",
      },
      capture: {
        enabled: false,
        redactHeaders: ["authorization", "x-api-key"],
      },
      defaults: {
        defaultRouteByCapability: {},
        defaultRouteByProtocol: {},
      },
      mode: "desktop-local",
      routes: [],
      runtime: {
        cleanupIntervalMs: 300000,
        maxConcurrentRequests: 8,
        retryCount: 1,
      },
      schemaVersion: 1,
      storage: {
        dialect: "sqlite",
        sqlitePath: "C:/sdkwork/data/local-api-proxy.db",
      },
    });
  });

  it("normalizes route ids, capability bindings, tags, and exposure targets from the authority config", () => {
    const config = normalizeLocalApiProxyConfig({
      bind: {
        host: "127.0.0.1",
        port: 21281,
      },
      capture: {
        enabled: true,
        redactHeaders: [" Authorization ", "x-api-key", "authorization"],
      },
      mode: "desktop-local",
      routes: [
        {
          capabilities: [
            {
              capability: "chat",
              enabled: true,
              operationSet: [
                " openai.v1.chat.completions.create ",
                "openai.v1.chat.completions.create",
                "",
              ],
              streaming: true,
            },
          ],
          clientProtocol: "openai-compatible",
          enabled: true,
          exposures: [
            {
              consumerId: " Sdkwork Desktop ",
              enabled: true,
              label: " Sdkwork Desktop ",
              target: "custom",
            },
            {
              consumerId: "sdkwork-desktop",
              enabled: true,
              target: "custom",
            },
          ],
          id: " Sdkwork Runtime / Primary ",
          managedBy: "system",
          modelBindings: [
            {
              capability: "chat",
              label: " GPT-5.4 ",
              modelId: "gpt-5.4",
              role: "default",
            },
          ],
          name: " Sdkwork Runtime ",
          notes: " bundled route ",
          providerId: "openai",
          tags: [" bundled ", "openai", "bundled"],
          upstream: {
            baseUrl: "https://api.openai.com/v1/ ",
            credentialRef: " keychain:openai/default ",
            mirrorProtocolIdentity: " openai ",
            protocolKind: "openai-compatible",
            providerId: "openai",
          },
          upstreamProtocol: "openai-compatible",
        },
      ],
      storage: {
        dialect: "sqlite",
        sqlitePath: "C:/sdkwork/data/local-api-proxy.db",
      },
    });

    expect(config.bind.publicBaseUrl).toBe("http://127.0.0.1:21281");
    expect(config.capture.redactHeaders).toEqual(["authorization", "x-api-key"]);
    expect(config.routes).toHaveLength(1);
    expect(config.routes[0]).toMatchObject({
      id: "sdkwork-runtime-primary",
      exposures: [
        {
          consumerId: "sdkwork-desktop",
          enabled: true,
          label: "Sdkwork Desktop",
          target: "custom",
        },
      ],
      modelBindings: [
        {
          capability: "chat",
          label: "GPT-5.4",
          modelId: "gpt-5.4",
          role: "default",
        },
      ],
      tags: ["bundled", "openai"],
      upstream: {
        baseUrl: "https://api.openai.com/v1",
        credentialRef: "keychain:openai/default",
        mirrorProtocolIdentity: "openai",
      },
    });
    expect(config.routes[0]?.capabilities).toEqual([
      {
        capability: "chat",
        enabled: true,
        operationSet: ["openai.v1.chat.completions.create"],
        streaming: true,
      },
    ]);
  });

  it("defaults the canonical postgresql schema for server-managed configs", () => {
    const config = normalizeLocalApiProxyConfig({
      bind: {
        host: "0.0.0.0",
        port: 3210,
      },
      mode: "server-managed",
      routes: [],
      storage: {
        dialect: "postgresql",
        postgresUrl: "postgres://localhost:5432/local_api_proxy",
      },
    });

    expect(LOCAL_API_PROXY_DEFAULT_POSTGRES_SCHEMA).toBe("local_api_proxy");
    expect(config.storage).toEqual({
      dialect: "postgresql",
      postgresUrl: "postgres://localhost:5432/local_api_proxy",
      schema: "local_api_proxy",
    });
    expect(config.bind.publicBaseUrl).toBe("http://0.0.0.0:3210");
  });

  it("keeps LocalApiProxyConfig assignable with the canonical route model", () => {
    const config: LocalApiProxyConfig = normalizeLocalApiProxyConfig({
      mode: "desktop-local",
      routes: [
        {
          capabilities: [
            {
              capability: "embedding",
              enabled: true,
              operationSet: ["openai.v1.embeddings.create"],
              streaming: false,
            },
          ],
          clientProtocol: "openai-compatible",
          enabled: true,
          exposures: [],
          id: "embeddings",
          managedBy: "user",
          modelBindings: [
            {
              capability: "embedding",
              modelId: "text-embedding-3-large",
              role: "embedding",
            },
          ],
          name: "Embeddings",
          providerId: "openai",
          tags: [],
          upstream: {
            baseUrl: "https://api.openai.com/v1",
            protocolKind: "openai-compatible",
            providerId: "openai",
          },
          upstreamProtocol: "openai-compatible",
        },
      ],
      storage: {
        dialect: "sqlite",
        sqlitePath: `C:/sdkwork/data/${LOCAL_API_PROXY_DEFAULT_SQLITE_FILENAME}`,
      },
    });

    expect(config.routes[0]?.capabilities[0]?.capability).toBe("embedding");
    expect(config.routes[0]?.modelBindings[0]?.role).toBe("embedding");
  });
});
