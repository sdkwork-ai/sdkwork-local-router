import { describe, expect, it } from "vitest";
import { localApiProxyPackageMeta } from "../src";

describe("@sdkwork/local-api-proxy package meta", () => {
  it("exports the canonical package identity for the local api proxy capability", () => {
    expect(localApiProxyPackageMeta).toEqual({
      architecture: "pc-react",
      capability: "local-api-proxy",
      domain: "intelligence",
      package: "@sdkwork/local-api-proxy",
      status: "ready",
    });
  });
});
