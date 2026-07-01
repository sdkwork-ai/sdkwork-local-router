import { createUserRootKernelConfigDefinition } from "./createUserRootKernelConfigDefinition.ts";
import { joinKernelPath } from "./kernelPathUtils.ts";

export const hermesKernelConfigDefinition = createUserRootKernelConfigDefinition({
  kernelId: "hermes",
  runtimeKinds: ["hermes"],
  format: "yaml",
  stateDirectoryName: ".hermes",
  configFileName: "config.yaml",
  workspaceDirectoryName: "workspace",
  legacyConfigDirectories: ["config"],
});

export const buildStandardHermesStateRoot =
  hermesKernelConfigDefinition.buildStandardStateRoot;
export const buildStandardHermesConfigFilePath =
  hermesKernelConfigDefinition.buildStandardConfigFilePath;
export const buildStandardHermesWorkspacePath =
  hermesKernelConfigDefinition.buildStandardWorkspacePath;
export const resolveHermesUserRootFromWorkspacePath =
  hermesKernelConfigDefinition.resolveUserRootFromWorkspacePath;
export const normalizeHermesKernelConfigFilePath =
  hermesKernelConfigDefinition.normalizeConfigFile;
export const resolveHermesStateRootFromConfigFile =
  hermesKernelConfigDefinition.resolveStateRootFromConfigFile;
export const resolveHermesUserRootFromConfigFile =
  hermesKernelConfigDefinition.resolveUserRootFromConfigFile;
export const resolveHermesUserPathFromConfigFile =
  hermesKernelConfigDefinition.resolveUserPathFromConfigFile;

export function buildStandardHermesStateDatabasePath(userRoot?: string | null) {
  return joinKernelPath(buildStandardHermesStateRoot(userRoot), "state.db");
}

export function buildStandardHermesSessionsRootPath(userRoot?: string | null) {
  return joinKernelPath(buildStandardHermesStateRoot(userRoot), "sessions");
}

export function buildStandardHermesLogsRootPath(userRoot?: string | null) {
  return joinKernelPath(buildStandardHermesStateRoot(userRoot), "logs");
}

export function resolveHermesStateDatabasePathFromConfigFile(configFile: string) {
  return joinKernelPath(resolveHermesStateRootFromConfigFile(configFile), "state.db");
}

export function resolveHermesSessionsRootFromConfigFile(configFile: string) {
  return joinKernelPath(resolveHermesStateRootFromConfigFile(configFile), "sessions");
}

export function resolveHermesLogsRootFromConfigFile(configFile: string) {
  return joinKernelPath(resolveHermesStateRootFromConfigFile(configFile), "logs");
}
