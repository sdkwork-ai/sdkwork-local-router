import assert from "node:assert/strict";

function runTest(name: string, fn: () => void | Promise<void>) {
  return Promise.resolve()
    .then(fn)
    .then(() => {
      console.log(`ok - ${name}`);
    })
    .catch((error) => {
      console.error(`not ok - ${name}`);
      throw error;
    });
}

let kernelConfigRegistryModule:
  | typeof import("./kernelConfigRegistry.ts")
  | undefined;

try {
  kernelConfigRegistryModule = await import("./kernelConfigRegistry.ts");
} catch {
  kernelConfigRegistryModule = undefined;
}

await runTest("kernelConfigRegistry exposes shared Sdkwork and Hermes kernel definitions", () => {
  assert.ok(kernelConfigRegistryModule, "Expected kernelConfigRegistry.ts to exist");

  const definitions = kernelConfigRegistryModule?.kernelConfigRegistry.listDefinitions() || [];
  assert.deepEqual(
    definitions.map((definition) => definition.kernelId),
    ["sdkwork", "hermes"],
  );
});

await runTest(
  "kernelConfigRegistry canonicalizes built-in Sdkwork drifted config paths to the standard user-root layout",
  () => {
    const projected = kernelConfigRegistryModule?.projectKernelConfig({
      runtimeKind: "sdkwork",
      deploymentMode: "local-managed",
      isBuiltIn: true,
      configFile:
        "C:/ProgramData/SdkWork/CrawStudio/state/kernels/sdkwork/managed-config/sdkwork.json",
      workspacePath: "C:/Users/admin/.sdkwork/workspace",
      configWritable: true,
      schemaVersion: null,
    });

    assert.deepEqual(projected, {
      kernelId: "sdkwork",
      runtimeKind: "sdkwork",
      configFile: "C:/Users/admin/.sdkwork/sdkwork.json",
      configRoot: "C:/Users/admin/.sdkwork",
      stateRoot: "C:/Users/admin/.sdkwork",
      userRoot: "C:/Users/admin",
      standardStateRoot: "C:/Users/admin/.sdkwork",
      standardConfigFile: "C:/Users/admin/.sdkwork/sdkwork.json",
      workspacePath: "C:/Users/admin/.sdkwork/workspace",
      format: "json",
      access: "localFs",
      provenance: "standardUserRoot",
      writable: true,
      resolved: true,
      schemaVersion: null,
      isStandardUserRootLayout: true,
    });
  },
);

await runTest(
  "kernelConfigRegistry projects Hermes kernel config using the same user-root kernel standard",
  () => {
    const projected = kernelConfigRegistryModule?.projectKernelConfig({
      runtimeKind: "hermes",
      deploymentMode: "local-managed",
      isBuiltIn: true,
      configFile: "C:/ProgramData/SdkWork/CrawStudio/state/kernels/hermes/config/hermes.json",
      workspacePath: "C:/Users/admin/.hermes/workspace",
      configWritable: false,
      schemaVersion: "2026.04.19",
    });

    assert.deepEqual(projected, {
      kernelId: "hermes",
      runtimeKind: "hermes",
      configFile: "C:/Users/admin/.hermes/config.yaml",
      configRoot: "C:/Users/admin/.hermes",
      stateRoot: "C:/Users/admin/.hermes",
      userRoot: "C:/Users/admin",
      standardStateRoot: "C:/Users/admin/.hermes",
      standardConfigFile: "C:/Users/admin/.hermes/config.yaml",
      workspacePath: "C:/Users/admin/.hermes/workspace",
      format: "yaml",
      access: "localFs",
      provenance: "standardUserRoot",
      writable: false,
      resolved: true,
      schemaVersion: "2026.04.19",
      isStandardUserRootLayout: true,
    });
  },
);
