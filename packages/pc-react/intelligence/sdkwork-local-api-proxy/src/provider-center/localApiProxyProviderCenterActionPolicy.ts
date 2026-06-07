import type { LocalApiProxyProviderConfigRecord } from "./localApiProxyProviderCenterTypes.ts";

export type LocalApiProxyProviderCenterRowActionId =
  | "quickApply"
  | "edit"
  | "delete"
  | "test";

export function listLocalApiProxyProviderCenterRowActionIds(
  record: Pick<LocalApiProxyProviderConfigRecord, "managedBy">,
): LocalApiProxyProviderCenterRowActionId[] {
  const actions: LocalApiProxyProviderCenterRowActionId[] = ["quickApply"];

  if (record.managedBy === "user") {
    actions.push("edit", "delete");
  }

  actions.push("test");
  return actions;
}
