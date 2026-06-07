import { createElement } from "react";
import { LocalApiProxyRouteList } from "../components/LocalApiProxyRouteList.ts";
import { LocalApiProxyRuntimeSummary } from "../components/LocalApiProxyRuntimeSummary.ts";
import type {
  LocalApiProxyRuntimeRouteView,
  LocalApiProxyRuntimeSummaryView,
} from "../services/localApiProxyRuntimeService.ts";

export interface LocalApiProxyPageProps {
  summary: LocalApiProxyRuntimeSummaryView;
  routes: LocalApiProxyRuntimeRouteView[];
  title?: string;
}

export function LocalApiProxyPage(props: LocalApiProxyPageProps) {
  const { routes, summary, title = "Local API Proxy" } = props;

  return createElement(
    "main",
    {
      style: {
        display: "grid",
        gap: 18,
        maxWidth: 1120,
        width: "100%",
        margin: "0 auto",
      },
    },
    createElement(
      "header",
      null,
      createElement("h1", { style: { margin: 0, fontSize: 28, lineHeight: 1.1 } }, title),
    ),
    createElement(LocalApiProxyRuntimeSummary, { summary }),
    createElement(LocalApiProxyRouteList, { routes }),
  );
}
