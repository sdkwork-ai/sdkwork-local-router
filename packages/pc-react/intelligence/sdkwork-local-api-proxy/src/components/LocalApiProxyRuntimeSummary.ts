import { createElement } from "react";
import type { LocalApiProxyRuntimeSummaryView } from "../services/localApiProxyRuntimeService.ts";

export interface LocalApiProxyRuntimeSummaryProps {
  summary: LocalApiProxyRuntimeSummaryView;
}

export function LocalApiProxyRuntimeSummary(props: LocalApiProxyRuntimeSummaryProps) {
  const { summary } = props;
  const stateLabel = summary.state.charAt(0).toUpperCase() + summary.state.slice(1);

  return createElement(
    "section",
    {
      "aria-label": "Local API Proxy Runtime Summary",
      style: {
        border: "1px solid #d6d9de",
        borderRadius: 12,
        padding: 16,
        background: "#ffffff",
      },
    },
    createElement(
      "div",
      { style: { display: "flex", justifyContent: "space-between", gap: 16 } },
      createElement(
        "div",
        null,
        createElement("h2", { style: { margin: 0, fontSize: 20, lineHeight: 1.2 } }, `Runtime ${stateLabel}`),
        createElement(
          "p",
          { style: { margin: "8px 0 0", color: "#4f5b67" } },
          `${summary.activeRouteCount} active of ${summary.routeCount} routes`,
        ),
      ),
      createElement(
        "div",
        { style: { textAlign: "right", color: "#1f2933" } },
        summary.publicBaseUrl,
      ),
    ),
  );
}
