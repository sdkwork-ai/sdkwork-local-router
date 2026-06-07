import { describe, expect, it } from "vitest";
import {
  createLocalApiProxyOperationCatalog,
  createLocalApiProxyRouteGroups,
  findLocalApiProxyOperation,
  listLocalApiProxyOperationsByCapability,
} from "../src";

describe("@sdkwork/local-api-proxy operation catalog", () => {
  it("registers stable operation ids with capability metadata and route groups", () => {
    const operations = createLocalApiProxyOperationCatalog();
    const groups = createLocalApiProxyRouteGroups(operations);

    expect(findLocalApiProxyOperation("openai.v1.chat.completions.create")).toMatchObject({
      capability: "chat",
      groupId: "text-and-chat",
      id: "openai.v1.chat.completions.create",
      method: "POST",
      pathPattern: "/v1/chat/completions",
      probeSupport: true,
      streaming: true,
    });
    expect(findLocalApiProxyOperation("openai.v1.responses.create")).toMatchObject({
      capability: "response",
      groupId: "text-and-chat",
      id: "openai.v1.responses.create",
      method: "POST",
      pathPattern: "/v1/responses",
      probeSupport: true,
      streaming: true,
    });
    expect(findLocalApiProxyOperation("openai.v1.models.list")).toMatchObject({
      capability: "model-catalog",
      groupId: "compat-and-model",
      method: "GET",
      pathPattern: "/v1/models",
      streaming: false,
    });
    expect(findLocalApiProxyOperation("openai.v1.embeddings.create")).toMatchObject({
      capability: "embedding",
      groupId: "embedding-and-moderation",
      method: "POST",
      pathPattern: "/v1/embeddings",
      streaming: false,
    });
    expect(findLocalApiProxyOperation("openai.v1.files.create")).toMatchObject({
      capability: "file-transfer",
      groupId: "files-and-batches",
      method: "POST",
      pathPattern: "/v1/files",
      streaming: false,
    });
    expect(findLocalApiProxyOperation("openai.v1.batches.create")).toMatchObject({
      capability: "batch",
      groupId: "files-and-batches",
      method: "POST",
      pathPattern: "/v1/batches",
      streaming: false,
    });

    expect(groups.find((group) => group.id === "text-and-chat")).toEqual({
      capabilityFamilies: ["chat", "response"],
      id: "text-and-chat",
      operationIds: [
        "openai.v1.chat.completions.create",
        "openai.v1.responses.create",
      ],
    });
    expect(groups.find((group) => group.id === "compat-and-model")).toEqual({
      capabilityFamilies: ["model-catalog"],
      id: "compat-and-model",
      operationIds: ["openai.v1.models.list", "openai.v1.models.retrieve"],
    });
    expect(groups.find((group) => group.id === "embedding-and-moderation")).toEqual({
      capabilityFamilies: ["embedding", "moderation"],
      id: "embedding-and-moderation",
      operationIds: [
        "openai.v1.embeddings.create",
        "openai.v1.moderations.create",
      ],
    });
    expect(groups.find((group) => group.id === "files-and-batches")).toEqual({
      capabilityFamilies: ["file-transfer", "batch"],
      id: "files-and-batches",
      operationIds: [
        "openai.v1.files.create",
        "openai.v1.files.retrieve",
        "openai.v1.batches.create",
      ],
    });
    expect(groups.find((group) => group.id === "vector-and-rerank")).toEqual({
      capabilityFamilies: ["vector-store", "rerank"],
      id: "vector-and-rerank",
      operationIds: ["openai.v1.vector-stores.create"],
    });
    expect(findLocalApiProxyOperation("openai.v1.search.query")).toBeUndefined();
  });

  it("lists operations by capability without leaking unrelated route groups", () => {
    expect(listLocalApiProxyOperationsByCapability("response")).toEqual([
      {
        capability: "response",
        consumerProtocol: "openai-compatible",
        groupId: "text-and-chat",
        id: "openai.v1.responses.create",
        method: "POST",
        pathPattern: "/v1/responses",
        probeSupport: true,
        streaming: true,
      },
    ]);
  });
});
