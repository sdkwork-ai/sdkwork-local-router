import assert from "node:assert/strict";

async function runTest(name: string, callback: () => Promise<void> | void) {
  try {
    await callback();
    console.log(`ok - ${name}`);
  } catch (error) {
    console.error(`not ok - ${name}`);
    throw error;
  }
}

await runTest(
  "kernelConfigDiscoveryService resolves Sdkwork config files from standardized roots before legacy data roots",
  async () => {
    const serviceModule = await import("./kernelConfigDiscoveryService.ts");

    const checks: string[] = [];
    const resolved = await serviceModule.resolveKernelInstallConfigPath({
      kernelId: "sdkwork",
      homeRoots: ["D:/Users/admin"],
      workRoot: "D:/Sdkwork/work",
      installRoot: "D:/Sdkwork/install",
      dataRoot: "D:/Sdkwork/data",
      pathExists: async (candidate) => {
        checks.push(candidate);
        return candidate === "D:/Users/admin/.sdkwork/sdkwork.json";
      },
    });

    assert.equal(resolved, "D:/Users/admin/.sdkwork/sdkwork.json");
    assert.deepEqual(checks, ["D:/Users/admin/.sdkwork/sdkwork.json"]);
  },
);

await runTest(
  "kernelConfigDiscoveryService falls back through Hermes legacy and standard roots using the same kernel definition standard",
  async () => {
    const serviceModule = await import("./kernelConfigDiscoveryService.ts");

    const checks: string[] = [];
    const resolved = await serviceModule.resolveKernelInstallConfigPath({
      kernelId: "hermes",
      workRoot: "D:/Hermes/work",
      installRoot: "D:/Hermes/install",
      dataRoot: "D:/Hermes/data",
      pathExists: async (candidate) => {
        checks.push(candidate);
        return candidate === "D:/Hermes/data/config/config.yaml";
      },
    });

    assert.equal(resolved, "D:/Hermes/data/config/config.yaml");
    assert.deepEqual(checks, [
      "D:/Hermes/work/.hermes/config.yaml",
      "D:/Hermes/install/.hermes/config.yaml",
      "D:/Hermes/data/config/config.yaml",
    ]);
  },
);

await runTest(
  "kernelConfigDiscoveryService exposes deterministic discovery candidates for future kernel composition",
  async () => {
    const serviceModule = await import("./kernelConfigDiscoveryService.ts");

    const candidates = serviceModule.listKernelInstallConfigPathCandidates({
      kernelId: "sdkwork",
      homeRoots: ["D:/Users/admin"],
      workRoot: "D:/Sdkwork/work",
      installRoot: "D:/Sdkwork/install",
      dataRoot: "D:/Sdkwork/data",
    });

    assert.deepEqual(candidates, [
      "D:/Users/admin/.sdkwork/sdkwork.json",
      "D:/Sdkwork/work/.sdkwork/sdkwork.json",
      "D:/Sdkwork/install/.sdkwork/sdkwork.json",
      "D:/Sdkwork/data/config/sdkwork.json",
      "D:/Sdkwork/data/managed-config/sdkwork.json",
      "D:/Sdkwork/data/sdkwork.json",
      "D:/Sdkwork/data/.sdkwork/sdkwork.json",
    ]);
  },
);
