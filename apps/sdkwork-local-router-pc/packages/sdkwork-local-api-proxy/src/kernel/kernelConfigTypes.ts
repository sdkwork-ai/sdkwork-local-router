export type KernelConfigFormat = "json" | "json5" | "yaml" | "unknown";

export type KernelConfigAccessMode =
  | "localFs"
  | "gateway"
  | "bridge"
  | "remoteApi"
  | "unavailable";

export interface KernelConfigProjectionInput {
  kernelId?: string | null;
  runtimeKind?: string | null;
  deploymentMode?: string | null;
  isBuiltIn?: boolean | null;
  configFile?: string | null;
  workspacePath?: string | null;
  configWritable?: boolean;
  schemaVersion?: string | null;
}

export interface KernelInstallConfigPathCandidatesInput {
  homeRoots?: string[];
  workRoot?: string | null;
  installRoot?: string | null;
  dataRoot?: string | null;
}

export interface KernelConfigDescriptor {
  kernelId: string;
  runtimeKind: string;
  configFile: string;
  configRoot: string | null;
  stateRoot: string | null;
  userRoot: string | null;
  standardStateRoot: string | null;
  standardConfigFile: string | null;
  workspacePath: string | null;
  format: KernelConfigFormat;
  access: KernelConfigAccessMode;
  provenance: string;
  writable: boolean;
  resolved: boolean;
  schemaVersion: string | null;
  isStandardUserRootLayout: boolean;
}

export interface KernelConfigDefinition {
  kernelId: string;
  runtimeKinds: string[];
  format: KernelConfigFormat;
  matchesRuntimeKind(runtimeKind?: string | null): boolean;
  project(input: KernelConfigProjectionInput): KernelConfigDescriptor | null;
}

export interface UserRootKernelConfigDefinitionOptions {
  kernelId: string;
  runtimeKinds?: string[];
  format?: KernelConfigFormat;
  stateDirectoryName: string;
  configFileName: string;
  workspaceDirectoryName?: string;
  builtInDeploymentModes?: string[];
  legacyConfigDirectories?: string[];
  standardProvenance?: string;
  externalProvenance?: string;
}
