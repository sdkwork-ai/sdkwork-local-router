import { createElement } from "react";
import type { LocalApiProxyRuntimeRouteView } from "../services/localApiProxyRuntimeService.ts";

export interface LocalApiProxyRouteListProps {
  routes: LocalApiProxyRuntimeRouteView[];
}

export function LocalApiProxyRouteList(props: LocalApiProxyRouteListProps) {
  const { routes } = props;

  return createElement(
    "section",
    { "aria-label": "Local API Proxy Routes" },
    createElement(
      "ul",
      { style: { listStyle: "none", margin: 0, padding: 0, display: "grid", gap: 12 } },
      routes.map((route) =>
        createElement(
          "li",
          {
            key: route.id,
            style: {
              border: "1px solid #d6d9de",
              borderRadius: 12,
              padding: 14,
              background: route.active ? "#f4fbf7" : "#ffffff",
            },
          },
          createElement(
            "div",
            { style: { display: "flex", justifyContent: "space-between", gap: 12 } },
            createElement(
              "div",
              null,
              createElement("div", { style: { fontWeight: 600 } }, route.name),
              createElement("div", { style: { color: "#6b7785", fontSize: 13 } }, route.id),
            ),
            createElement(
              "div",
              {
                style: {
                  alignSelf: "flex-start",
                  borderRadius: 999,
                  padding: "4px 10px",
                  background: route.active ? "#d7f5e3" : "#eef2f6",
                  color: route.active ? "#0f6b3d" : "#52606d",
                  fontSize: 12,
                  fontWeight: 600,
                },
              },
              route.active ? "Active" : "Standby",
            ),
          ),
        ),
      ),
    ),
  );
}
