export const localApiProxyPackageMeta = {
  package: "@sdkwork/local-api-proxy",
  architecture: "pc-react",
  domain: "intelligence",
  capability: "local-api-proxy",
  status: "ready",
} as const;

export type LocalApiProxyPackageMeta = typeof localApiProxyPackageMeta;

export * from "./types/localApiProxyTypes.ts";
export * from "./types/localApiProxyHost.ts";
export * from "./types/localApiProxyProviderRoute.ts";
export * from "./types/localApiProxyProviderRuntimeConfig.ts";
export * from "./types/localApiProxyProviderCatalog.ts";
export * from "./kernel/kernelConfigTypes.ts";
export * from "./kernel/kernelPathUtils.ts";
export * from "./kernel/createUserRootKernelConfigDefinition.ts";
export * from "./kernel/sdkworkKernelConfig.ts";
export * from "./kernel/hermesKernelConfig.ts";
export * from "./kernel/kernelConfigRegistry.ts";
export * from "./kernel/kernelConfigDiscoveryService.ts";
export * from "./domain/localApiProxyConfig.ts";
export * from "./domain/localApiProxyRuntimePlan.ts";
export * from "./gateway/localApiProxyOperations.ts";
export * from "./gateway/localApiProxyRouteGroups.ts";
export * from "./repository/localApiProxyRepository.ts";
export * from "./repository/localApiProxySchema.ts";
export * from "./repository/localApiProxySqlite.ts";
export * from "./repository/localApiProxyPostgresql.ts";
export * from "./host/tauri/localApiProxyTauriBridge.ts";
export * from "./services/localApiProxyHostService.ts";
export * from "./services/localApiProxyConfigService.ts";
export * from "./services/localApiProxyRuntimeService.ts";
export * from "./services/localApiProxyObservabilityService.ts";
export * from "./services/json5Compat.ts";
export * from "./services/localApiProxyProviderRouteService.ts";
export * from "./services/localApiProxyProviderRequestDraftService.ts";
export * from "./services/localApiProxyProviderRuntimeConfigService.ts";
export * from "./services/localApiProxyProviderRoutingCatalogService.ts";
export * from "./services/localApiProxyProjectionService.ts";
export * from "./services/localApiProxyProjectedProviderModelService.ts";
export * from "./services/localApiProxyLogsWorkspaceService.ts";
export * from "./provider-center/localApiProxyProviderCenterTypes.ts";
export * from "./provider-center/localApiProxyProviderCenterActionPolicy.ts";
export * from "./provider-center/localApiProxyProviderCenterPresentation.ts";
export * from "./provider-center/localApiProxyProviderConfigPresets.ts";
export * from "./provider-center/localApiProxyProviderConfigEditorPolicy.ts";
export * from "./provider-center/localApiProxyProviderImportService.ts";
export * from "./provider-center/localApiProxyProviderCenterService.ts";
export * from "./provider-center/localApiProxyProviderCenterWorkspaceService.ts";
export * from "./provider-center/localApiProxyLogsService.ts";
export * from "./components/LocalApiProxyRuntimeSummary.ts";
export * from "./components/LocalApiProxyRouteList.ts";
export * from "./pages/LocalApiProxyPage.ts";
