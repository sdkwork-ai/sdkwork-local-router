import type { UserRootKernelConfigDefinition } from "./createUserRootKernelConfigDefinition.ts";
import {
  createKernelConfigRegistry,
  kernelConfigRegistry,
  type KernelConfigRegistry,
} from "./kernelConfigRegistry.ts";
import type { KernelInstallConfigPathCandidatesInput } from "./kernelConfigTypes.ts";

export interface KernelInstallConfigPathResolutionInput
  extends KernelInstallConfigPathCandidatesInput {
  kernelId?: string | null;
  runtimeKind?: string | null;
  pathExists(candidate: string): Promise<boolean>;
  registry?: KernelConfigRegistry;
}

export interface ListKernelInstallConfigPathCandidatesInput
  extends KernelInstallConfigPathCandidatesInput {
  kernelId?: string | null;
  runtimeKind?: string | null;
  registry?: KernelConfigRegistry;
}

function isUserRootKernelConfigDefinition(
  definition: unknown,
): definition is UserRootKernelConfigDefinition {
  return Boolean(
    definition
    && typeof definition === "object"
    && "listInstallConfigPathCandidates" in definition
    && typeof (definition as UserRootKernelConfigDefinition).listInstallConfigPathCandidates === "function",
  );
}

function resolveDiscoveryDefinition(input: {
  kernelId?: string | null;
  runtimeKind?: string | null;
  registry?: KernelConfigRegistry;
}) {
  const registry = input.registry || kernelConfigRegistry || createKernelConfigRegistry();
  const definition = registry.resolveDefinition({
    kernelId: input.kernelId,
    runtimeKind: input.runtimeKind,
  });

  return isUserRootKernelConfigDefinition(definition) ? definition : null;
}

export function listKernelInstallConfigPathCandidates(
  input: ListKernelInstallConfigPathCandidatesInput,
) {
  const definition = resolveDiscoveryDefinition(input);
  if (!definition) {
    return [];
  }

  return definition.listInstallConfigPathCandidates(input);
}

export async function resolveKernelInstallConfigPath(
  input: KernelInstallConfigPathResolutionInput,
) {
  for (const candidate of listKernelInstallConfigPathCandidates(input)) {
    if (await input.pathExists(candidate)) {
      return candidate;
    }
  }

  return null;
}
