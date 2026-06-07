import {
  dirnameKernelPath,
  isAbsoluteKernelPath,
  joinKernelPath,
  normalizeKernelPath,
} from "./kernelPathUtils.ts";
import type {
  KernelConfigDefinition,
  KernelConfigDescriptor,
  KernelInstallConfigPathCandidatesInput,
  KernelConfigProjectionInput,
  UserRootKernelConfigDefinitionOptions,
} from "./kernelConfigTypes.ts";

function normalizeId(value?: string | null) {
  return (value || "").trim().toLowerCase();
}

export interface UserRootKernelConfigDefinition extends KernelConfigDefinition {
  buildStandardStateRoot(userRoot?: string | null): string;
  buildStandardConfigFilePath(userRoot?: string | null): string;
  buildStandardWorkspacePath(userRoot?: string | null): string;
  listInstallConfigPathCandidates(input: KernelInstallConfigPathCandidatesInput): string[];
  resolveUserRootFromWorkspacePath(workspacePath?: string | null): string;
  normalizeConfigFile(input: KernelConfigProjectionInput): string;
  resolveStateRootFromConfigFile(configFile: string): string;
  resolveUserRootFromConfigFile(configFile: string): string;
  resolveUserPathFromConfigFile(configFile: string, rawPath?: string | null): string;
}

export function createUserRootKernelConfigDefinition(
  options: UserRootKernelConfigDefinitionOptions,
): UserRootKernelConfigDefinition {
  const runtimeKinds = Array.from(
    new Set([options.kernelId, ...(options.runtimeKinds || [])].map((entry) => normalizeId(entry))),
  ).filter(Boolean);
  const builtInDeploymentModes = (options.builtInDeploymentModes || ["local-managed"]).map(
    (entry) => normalizeId(entry),
  );
  const legacyConfigDirectories = options.legacyConfigDirectories || ["config"];
  const standardProvenance = options.standardProvenance || "standardUserRoot";
  const externalProvenance = options.externalProvenance || "runtimeReported";
  const standardConfigSuffix = `/${options.stateDirectoryName}/${options.configFileName}`;
  const standardWorkspaceSuffix = options.workspaceDirectoryName
    ? `/${options.stateDirectoryName}/${options.workspaceDirectoryName}`
    : "";

  const matchesRuntimeKind = (runtimeKind?: string | null) =>
    runtimeKinds.includes(normalizeId(runtimeKind));

  const buildStandardStateRoot = (userRoot?: string | null) => {
    const normalizedUserRoot = normalizeKernelPath(userRoot);
    if (!normalizedUserRoot) {
      return "";
    }

    return joinKernelPath(normalizedUserRoot, options.stateDirectoryName);
  };

  const buildStandardConfigFilePath = (userRoot?: string | null) => {
    const stateRoot = buildStandardStateRoot(userRoot);
    return stateRoot ? joinKernelPath(stateRoot, options.configFileName) : "";
  };

  const buildStandardWorkspacePath = (userRoot?: string | null) => {
    const stateRoot = buildStandardStateRoot(userRoot);
    return stateRoot && options.workspaceDirectoryName
      ? joinKernelPath(stateRoot, options.workspaceDirectoryName)
      : "";
  };

  const pushCandidatePath = (candidates: string[], candidate?: string | null) => {
    const normalizedCandidate = normalizeKernelPath(candidate);
    if (!normalizedCandidate || candidates.includes(normalizedCandidate)) {
      return;
    }

    candidates.push(normalizedCandidate);
  };

  const listInstallConfigPathCandidates = (
    input: KernelInstallConfigPathCandidatesInput,
  ): string[] => {
    const candidates: string[] = [];

    for (const homeRoot of input.homeRoots || []) {
      pushCandidatePath(candidates, buildStandardConfigFilePath(homeRoot));
    }

    pushCandidatePath(candidates, buildStandardConfigFilePath(input.workRoot));
    pushCandidatePath(candidates, buildStandardConfigFilePath(input.installRoot));

    for (const directoryName of legacyConfigDirectories) {
      pushCandidatePath(
        candidates,
        joinKernelPath(input.dataRoot, directoryName, options.configFileName),
      );
    }

    pushCandidatePath(candidates, joinKernelPath(input.dataRoot, options.configFileName));
    pushCandidatePath(candidates, buildStandardConfigFilePath(input.dataRoot));

    return candidates;
  };

  const resolveUserRootFromWorkspacePath = (workspacePath?: string | null) => {
    const normalizedWorkspacePath = normalizeKernelPath(workspacePath);
    if (!standardWorkspaceSuffix || !normalizedWorkspacePath.endsWith(standardWorkspaceSuffix)) {
      return "";
    }

    return normalizedWorkspacePath.slice(0, -standardWorkspaceSuffix.length);
  };

  const isBuiltInKernelInput = (input: KernelConfigProjectionInput) =>
    matchesRuntimeKind(input.runtimeKind) &&
    input.isBuiltIn === true &&
    builtInDeploymentModes.includes(normalizeId(input.deploymentMode));

  const normalizeConfigFile = (input: KernelConfigProjectionInput) => {
    const normalizedConfigFile = normalizeKernelPath(input.configFile);
    if (!normalizedConfigFile) {
      return "";
    }

    if (normalizedConfigFile.endsWith(standardConfigSuffix)) {
      return normalizedConfigFile;
    }

    if (!isBuiltInKernelInput(input)) {
      return normalizedConfigFile;
    }

    const userRoot = resolveUserRootFromWorkspacePath(input.workspacePath);
    return buildStandardConfigFilePath(userRoot) || normalizedConfigFile;
  };

  const resolveStateRootFromConfigFile = (configFile: string) => {
    const normalizedConfigFile = normalizeKernelPath(configFile);
    if (!normalizedConfigFile) {
      return "";
    }

    if (normalizedConfigFile.endsWith(standardConfigSuffix)) {
      return dirnameKernelPath(normalizedConfigFile);
    }

    for (const directoryName of legacyConfigDirectories) {
      if (normalizedConfigFile.endsWith(`/${directoryName}/${options.configFileName}`)) {
        return dirnameKernelPath(dirnameKernelPath(normalizedConfigFile));
      }
    }

    return dirnameKernelPath(normalizedConfigFile);
  };

  const resolveUserRootFromConfigFile = (configFile: string) => {
    const stateRoot = resolveStateRootFromConfigFile(configFile);
    if (!stateRoot) {
      return "";
    }

    if (stateRoot.endsWith(`/${options.stateDirectoryName}`)) {
      return stateRoot.slice(0, -`/${options.stateDirectoryName}`.length);
    }

    return dirnameKernelPath(stateRoot);
  };

  const resolveUserPathFromConfigFile = (configFile: string, rawPath?: string | null) => {
    const trimmedPath = rawPath?.trim() || "";
    if (!trimmedPath) {
      return "";
    }

    if (trimmedPath === "~") {
      return resolveUserRootFromConfigFile(configFile);
    }

    if (trimmedPath.startsWith("~/")) {
      return joinKernelPath(resolveUserRootFromConfigFile(configFile), trimmedPath.slice(2));
    }

    if (isAbsoluteKernelPath(trimmedPath)) {
      return normalizeKernelPath(trimmedPath);
    }

    const relativePath = trimmedPath.startsWith("./") ? trimmedPath.slice(2) : trimmedPath;
    return joinKernelPath(resolveStateRootFromConfigFile(configFile), relativePath);
  };

  const project = (input: KernelConfigProjectionInput): KernelConfigDescriptor | null => {
    const normalizedConfigFile = normalizeConfigFile(input);
    if (!normalizedConfigFile) {
      return null;
    }

    const userRoot = resolveUserRootFromConfigFile(normalizedConfigFile) || null;
    const stateRoot = resolveStateRootFromConfigFile(normalizedConfigFile) || null;
    const standardStateRoot = userRoot ? buildStandardStateRoot(userRoot) || null : null;
    const standardConfigFile = userRoot ? buildStandardConfigFilePath(userRoot) || null : null;
    const runtimeKind = normalizeId(input.runtimeKind) || runtimeKinds[0] || normalizeId(options.kernelId);
    const isStandardUserRootLayout =
      Boolean(standardConfigFile) && normalizedConfigFile === standardConfigFile;

    return {
      kernelId: options.kernelId,
      runtimeKind,
      configFile: normalizedConfigFile,
      configRoot: dirnameKernelPath(normalizedConfigFile) || null,
      stateRoot,
      userRoot,
      standardStateRoot,
      standardConfigFile,
      workspacePath: normalizeKernelPath(input.workspacePath) || null,
      format: options.format || "json",
      access: "localFs",
      provenance: isStandardUserRootLayout ? standardProvenance : externalProvenance,
      writable: input.configWritable === true,
      resolved: true,
      schemaVersion: input.schemaVersion || null,
      isStandardUserRootLayout,
    };
  };

  return {
    kernelId: options.kernelId,
    runtimeKinds,
    format: options.format || "json",
    matchesRuntimeKind,
    buildStandardStateRoot,
    buildStandardConfigFilePath,
    buildStandardWorkspacePath,
    listInstallConfigPathCandidates,
    resolveUserRootFromWorkspacePath,
    normalizeConfigFile,
    resolveStateRootFromConfigFile,
    resolveUserRootFromConfigFile,
    resolveUserPathFromConfigFile,
    project,
  };
}
