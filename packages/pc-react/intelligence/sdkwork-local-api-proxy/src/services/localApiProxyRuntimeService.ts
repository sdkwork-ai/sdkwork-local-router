import type { LocalApiProxyControlRepository } from "../repository/localApiProxyRepository.ts";
import type { LocalApiProxyHostPort } from "../types/localApiProxyHost.ts";

export interface LocalApiProxyRuntimeRouteView {
  id: string;
  name: string;
  enabled: boolean;
  active: boolean;
}

export interface LocalApiProxyRuntimeSummaryView {
  state: Awaited<ReturnType<LocalApiProxyHostPort["getRuntimeStatus"]>>["state"];
  publicBaseUrl: string;
  activeRouteCount: number;
  routeCount: number;
  routes: LocalApiProxyRuntimeRouteView[];
}

export function createLocalApiProxyRuntimeService(dependencies: {
  repository: Pick<LocalApiProxyControlRepository, "listRoutes">;
  host: Pick<LocalApiProxyHostPort, "getRuntimeStatus">;
}) {
  return {
    async getRuntimeSummary(): Promise<LocalApiProxyRuntimeSummaryView> {
      const [routes, runtimeStatus] = await Promise.all([
        dependencies.repository.listRoutes(),
        dependencies.host.getRuntimeStatus(),
      ]);

      return {
        state: runtimeStatus.state,
        publicBaseUrl: runtimeStatus.publicBaseUrl,
        activeRouteCount: runtimeStatus.activeRouteIds.length,
        routeCount: routes.length,
        routes: routes.map((route) => ({
          id: route.id,
          name: route.name,
          enabled: route.enabled,
          active: runtimeStatus.activeRouteIds.includes(route.id),
        })),
      };
    },
  };
}
