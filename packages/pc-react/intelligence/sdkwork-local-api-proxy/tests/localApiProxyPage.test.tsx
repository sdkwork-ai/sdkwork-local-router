import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import {
  LocalApiProxyPage,
  LocalApiProxyRouteList,
  LocalApiProxyRuntimeSummary,
} from "../src";

const summary = {
  activeRouteCount: 1,
  publicBaseUrl: "http://127.0.0.1:21281",
  routeCount: 2,
  routes: [
    {
      active: true,
      enabled: true,
      id: "chat-primary",
      name: "Chat Primary",
    },
    {
      active: false,
      enabled: true,
      id: "embeddings-backoffice",
      name: "Embeddings Backoffice",
    },
  ],
  state: "running" as const,
};

describe("@sdkwork/local-api-proxy page surfaces", () => {
  it("renders a route list with active and standby states", () => {
    render(<LocalApiProxyRouteList routes={summary.routes} />);

    expect(screen.getByText("Chat Primary")).toBeInTheDocument();
    expect(screen.getByText("Embeddings Backoffice")).toBeInTheDocument();
    expect(screen.getByText("Active")).toBeInTheDocument();
    expect(screen.getByText("Standby")).toBeInTheDocument();
  });

  it("renders the runtime summary with package-owned diagnostics copy", () => {
    render(<LocalApiProxyRuntimeSummary summary={summary} />);

    expect(screen.getByText("Runtime Running")).toBeInTheDocument();
    expect(screen.getByText("1 active of 2 routes")).toBeInTheDocument();
    expect(screen.getByText("http://127.0.0.1:21281")).toBeInTheDocument();
  });

  it("renders the composed page shell without product-owned wrappers", () => {
    render(<LocalApiProxyPage summary={summary} routes={summary.routes} />);

    expect(screen.getByText("Local API Proxy")).toBeInTheDocument();
    expect(screen.getByText("Runtime Running")).toBeInTheDocument();
    expect(screen.getByText("Chat Primary")).toBeInTheDocument();
  });
});
