import type {
  LocalApiCapability,
  LocalApiClientProtocol,
  LocalApiProxyHttpMethod,
} from "../types/localApiProxyTypes.ts";
import {
  LOCAL_API_PROXY_ROUTE_GROUPS,
  type LocalApiProxyRouteGroup,
  type LocalApiProxyRouteGroupId,
} from "./localApiProxyRouteGroups.ts";

export interface LocalApiProxyGatewayOperation {
  id: string;
  capability: LocalApiCapability;
  consumerProtocol: LocalApiClientProtocol;
  method: LocalApiProxyHttpMethod;
  pathPattern: string;
  streaming: boolean;
  groupId: LocalApiProxyRouteGroupId;
  probeSupport: boolean;
}

const LOCAL_API_PROXY_OPERATION_CATALOG = [
  {
    id: "openai.v1.chat.completions.create",
    capability: "chat",
    consumerProtocol: "openai-compatible",
    method: "POST",
    pathPattern: "/v1/chat/completions",
    streaming: true,
    groupId: "text-and-chat",
    probeSupport: true,
  },
  {
    id: "openai.v1.responses.create",
    capability: "response",
    consumerProtocol: "openai-compatible",
    method: "POST",
    pathPattern: "/v1/responses",
    streaming: true,
    groupId: "text-and-chat",
    probeSupport: true,
  },
  {
    id: "openai.v1.models.list",
    capability: "model-catalog",
    consumerProtocol: "openai-compatible",
    method: "GET",
    pathPattern: "/v1/models",
    streaming: false,
    groupId: "compat-and-model",
    probeSupport: false,
  },
  {
    id: "openai.v1.models.retrieve",
    capability: "model-catalog",
    consumerProtocol: "openai-compatible",
    method: "GET",
    pathPattern: "/v1/models/:model",
    streaming: false,
    groupId: "compat-and-model",
    probeSupport: false,
  },
  {
    id: "openai.v1.embeddings.create",
    capability: "embedding",
    consumerProtocol: "openai-compatible",
    method: "POST",
    pathPattern: "/v1/embeddings",
    streaming: false,
    groupId: "embedding-and-moderation",
    probeSupport: true,
  },
  {
    id: "openai.v1.moderations.create",
    capability: "moderation",
    consumerProtocol: "openai-compatible",
    method: "POST",
    pathPattern: "/v1/moderations",
    streaming: false,
    groupId: "embedding-and-moderation",
    probeSupport: true,
  },
  {
    id: "openai.v1.files.create",
    capability: "file-transfer",
    consumerProtocol: "openai-compatible",
    method: "POST",
    pathPattern: "/v1/files",
    streaming: false,
    groupId: "files-and-batches",
    probeSupport: false,
  },
  {
    id: "openai.v1.files.retrieve",
    capability: "file-transfer",
    consumerProtocol: "openai-compatible",
    method: "GET",
    pathPattern: "/v1/files/:fileId",
    streaming: false,
    groupId: "files-and-batches",
    probeSupport: false,
  },
  {
    id: "openai.v1.batches.create",
    capability: "batch",
    consumerProtocol: "openai-compatible",
    method: "POST",
    pathPattern: "/v1/batches",
    streaming: false,
    groupId: "files-and-batches",
    probeSupport: false,
  },
  {
    id: "openai.v1.vector-stores.create",
    capability: "vector-store",
    consumerProtocol: "openai-compatible",
    method: "POST",
    pathPattern: "/v1/vector_stores",
    streaming: false,
    groupId: "vector-and-rerank",
    probeSupport: false,
  },
  {
    id: "custom.v1.invoke",
    capability: "custom",
    consumerProtocol: "custom-http",
    method: "POST",
    pathPattern: "/custom/:operation",
    streaming: false,
    groupId: "custom",
    probeSupport: true,
  },
] as const satisfies readonly LocalApiProxyGatewayOperation[];

export function createLocalApiProxyOperationCatalog(): LocalApiProxyGatewayOperation[] {
  return LOCAL_API_PROXY_OPERATION_CATALOG.map((operation) => ({ ...operation }));
}

export function findLocalApiProxyOperation(operationId: string) {
  return createLocalApiProxyOperationCatalog().find((operation) => operation.id === operationId);
}

export function listLocalApiProxyOperationsByCapability(capability: LocalApiCapability) {
  return createLocalApiProxyOperationCatalog().filter(
    (operation) => operation.capability === capability,
  );
}

export function createLocalApiProxyRouteGroups(
  operations = createLocalApiProxyOperationCatalog(),
): LocalApiProxyRouteGroup[] {
  return LOCAL_API_PROXY_ROUTE_GROUPS.map((group) => ({
    id: group.id,
    capabilityFamilies: [...group.capabilityFamilies],
    operationIds: operations
      .filter((operation) => operation.groupId === group.id)
      .map((operation) => operation.id),
  })).filter((group) => group.operationIds.length > 0);
}
