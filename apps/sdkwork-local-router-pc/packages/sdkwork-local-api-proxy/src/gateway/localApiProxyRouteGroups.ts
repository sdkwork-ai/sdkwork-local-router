import type { LocalApiCapability } from "../types/localApiProxyTypes.ts";

export type LocalApiProxyRouteGroupId =
  | "text-and-chat"
  | "compat-and-model"
  | "embedding-and-moderation"
  | "files-and-batches"
  | "vector-and-rerank"
  | "custom";

export interface LocalApiProxyRouteGroup {
  id: LocalApiProxyRouteGroupId;
  capabilityFamilies: LocalApiCapability[];
  operationIds: string[];
}

export const LOCAL_API_PROXY_ROUTE_GROUPS = [
  {
    id: "text-and-chat",
    capabilityFamilies: ["chat", "response"],
  },
  {
    id: "compat-and-model",
    capabilityFamilies: ["model-catalog"],
  },
  {
    id: "embedding-and-moderation",
    capabilityFamilies: ["embedding", "moderation"],
  },
  {
    id: "files-and-batches",
    capabilityFamilies: ["file-transfer", "batch"],
  },
  {
    id: "vector-and-rerank",
    capabilityFamilies: ["vector-store", "rerank"],
  },
  {
    id: "custom",
    capabilityFamilies: ["custom"],
  },
] as const satisfies readonly Omit<LocalApiProxyRouteGroup, "operationIds">[];
