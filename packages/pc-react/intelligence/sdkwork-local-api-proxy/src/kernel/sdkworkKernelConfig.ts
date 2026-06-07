import { createUserRootKernelConfigDefinition } from "./createUserRootKernelConfigDefinition.ts";

export const sdkworkKernelConfigDefinition = createUserRootKernelConfigDefinition({
  kernelId: "sdkwork",
  runtimeKinds: ["sdkwork"],
  format: "json",
  stateDirectoryName: ".sdkwork",
  configFileName: "sdkwork.json",
  workspaceDirectoryName: "workspace",
  legacyConfigDirectories: ["config", "managed-config"],
});

export const buildStandardSdkworkStateRoot =
  sdkworkKernelConfigDefinition.buildStandardStateRoot;
export const buildStandardSdkworkConfigFilePath =
  sdkworkKernelConfigDefinition.buildStandardConfigFilePath;
export const buildStandardSdkworkWorkspacePath =
  sdkworkKernelConfigDefinition.buildStandardWorkspacePath;
export const resolveSdkworkUserRootFromWorkspacePath =
  sdkworkKernelConfigDefinition.resolveUserRootFromWorkspacePath;
export const normalizeSdkworkKernelConfigFilePath =
  sdkworkKernelConfigDefinition.normalizeConfigFile;
export const resolveSdkworkStateRootFromConfigFile =
  sdkworkKernelConfigDefinition.resolveStateRootFromConfigFile;
export const resolveSdkworkUserRootFromConfigFile =
  sdkworkKernelConfigDefinition.resolveUserRootFromConfigFile;
export const resolveSdkworkUserPathFromConfigFile =
  sdkworkKernelConfigDefinition.resolveUserPathFromConfigFile;
