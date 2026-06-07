import { describe, expect, it } from "vitest";
import {
  compileLocalApiProxyRuntimePlan,
  normalizeLocalApiProxyConfig,
  selectLocalApiProxyDefaultRoute,
} from "../src";

describe("@sdkwork/local-api-proxy runtime plan", () => {
  it("compiles a deterministic runtime plan with active capability defaults", () => {
    const config = normalizeLocalApiProxyConfig({
      defaults: {
        defaultRouteByCapability: {
          chat: "chat-primary",
        },
        defaultRouteByProtocol: {
          "openai-compatible": "chat-primary",
        },
      },
      mode: "desktop-local",
      routes: [
        {
          capabilities: [
            {
              capability: "chat",
              enabled: true,
              operationSet: ["openai.v1.chat.completions.create"],
              streaming: true,
            },
            {
              capability: "response",
              enabled: false,
              operationSet: ["openai.v1.responses.create"],
              streaming: true,
            },
          ],
          clientProtocol: "openai-compatible",
          enabled: true,
          exposures: [{ enabled: true, target: "desktop-clients" }],
          id: "chat-primary",
          managedBy: "system",
          modelBindings: [{ capability: "chat", modelId: "gpt-5.4", role: "default" }],
          name: "Chat Primary",
          providerId: "openai",
          tags: ["primary"],
          upstream: {
            baseUrl: "https://api.openai.com/v1",
            protocolKind: "openai-compatible",
            providerId: "openai",
          },
          upstreamProtocol: "openai-compatible",
        },
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
          exposures: [{ enabled: true, target: "internal-sdk" }],
          id: "embeddings-backoffice",
          managedBy: "user",
          modelBindings: [
            {
              capability: "embedding",
              modelId: "text-embedding-3-large",
              role: "embedding",
            },
          ],
          name: "Embeddings Backoffice",
          providerId: "openai",
          tags: ["embeddings"],
          upstream: {
            baseUrl: "https://api.openai.com/v1",
            protocolKind: "openai-compatible",
            providerId: "openai",
          },
          upstreamProtocol: "openai-compatible",
        },
        {
          capabilities: [
            {
              capability: "chat",
              enabled: true,
              operationSet: ["openai.v1.chat.completions.create"],
              streaming: true,
            },
          ],
          clientProtocol: "openai-compatible",
          enabled: false,
          exposures: [{ enabled: true, target: "desktop-clients" }],
          id: "disabled-backup",
          managedBy: "user",
          modelBindings: [],
          name: "Disabled Backup",
          providerId: "openai",
          tags: ["disabled"],
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
        sqlitePath: "C:/sdkwork/data/local-api-proxy.db",
      },
    });

    const plan = compileLocalApiProxyRuntimePlan(config);

    expect(plan.routes.map((route) => route.routeId)).toEqual([
      "chat-primary",
      "embeddings-backoffice",
    ]);
    expect(plan.capabilityIndex.chat).toEqual({
      defaultRouteId: "chat-primary",
      routeIds: ["chat-primary"],
    });
    expect(plan.capabilityIndex.embedding).toEqual({
      defaultRouteId: "embeddings-backoffice",
      routeIds: ["embeddings-backoffice"],
    });
    expect(plan.protocolDefaults).toEqual({
      "openai-compatible": "chat-primary",
    });
    expect(plan.routes[0]).toMatchObject({
      activeCapabilities: [
        {
          capability: "chat",
          operationSet: ["openai.v1.chat.completions.create"],
        },
      ],
      routeId: "chat-primary",
    });
  });

  it("filters the runtime plan to the exposures available to a specific consumer", () => {
    const config = normalizeLocalApiProxyConfig({
      mode: "desktop-local",
      routes: [
        {
          capabilities: [
            {
              capability: "chat",
              enabled: true,
              operationSet: ["openai.v1.chat.completions.create"],
              streaming: true,
            },
          ],
          clientProtocol: "openai-compatible",
          enabled: true,
          exposures: [{ enabled: true, target: "desktop-clients" }],
          id: "desktop-clients-default",
          managedBy: "system",
          modelBindings: [],
          name: "Desktop Clients Default",
          providerId: "openai",
          tags: [],
          upstream: {
            baseUrl: "https://api.openai.com/v1",
            protocolKind: "openai-compatible",
            providerId: "openai",
          },
          upstreamProtocol: "openai-compatible",
        },
        {
          capabilities: [
            {
              capability: "chat",
              enabled: true,
              operationSet: ["openai.v1.chat.completions.create"],
              streaming: true,
            },
          ],
          clientProtocol: "openai-compatible",
          enabled: true,
          exposures: [
            {
              consumerId: "sdkwork-desktop",
              enabled: true,
              label: "Sdkwork Desktop",
              target: "custom",
            },
          ],
          id: "sdkwork-custom",
          managedBy: "user",
          modelBindings: [],
          name: "Sdkwork Custom",
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
        sqlitePath: "C:/sdkwork/data/local-api-proxy.db",
      },
    });

    const plan = compileLocalApiProxyRuntimePlan(config, {
      consumer: {
        consumerId: "sdkwork-desktop",
        target: "custom",
      },
    });

    expect(plan.routes.map((route) => route.routeId)).toEqual(["sdkwork-custom"]);
    expect(selectLocalApiProxyDefaultRoute(plan, { capability: "chat" })?.routeId).toBe(
      "sdkwork-custom",
    );
  });
});
