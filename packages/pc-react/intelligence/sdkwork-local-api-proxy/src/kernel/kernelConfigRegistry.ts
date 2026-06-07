import type {
  KernelConfigDefinition,
  KernelConfigDescriptor,
  KernelConfigProjectionInput,
} from "./kernelConfigTypes.ts";
import { hermesKernelConfigDefinition } from "./hermesKernelConfig.ts";
import { sdkworkKernelConfigDefinition } from "./sdkworkKernelConfig.ts";

function normalizeId(value?: string | null) {
  return (value || "").trim().toLowerCase();
}

export class KernelConfigRegistry {
  private readonly definitions: KernelConfigDefinition[];
  private readonly definitionsByKernelId: Map<string, KernelConfigDefinition>;

  constructor(definitions: KernelConfigDefinition[]) {
    this.definitions = definitions;
    this.definitionsByKernelId = new Map(
      definitions.map((definition) => [normalizeId(definition.kernelId), definition] as const),
    );
  }

  listDefinitions() {
    return [...this.definitions];
  }

  resolveDefinition(input: {
    kernelId?: string | null;
    runtimeKind?: string | null;
  }) {
    const kernelId = normalizeId(input.kernelId);
    if (kernelId && this.definitionsByKernelId.has(kernelId)) {
      return this.definitionsByKernelId.get(kernelId) || null;
    }

    const runtimeKind = normalizeId(input.runtimeKind);
    if (!runtimeKind) {
      return null;
    }

    return this.definitions.find((definition) => definition.matchesRuntimeKind(runtimeKind)) || null;
  }

  project(input: KernelConfigProjectionInput): KernelConfigDescriptor | null {
    const definition = this.resolveDefinition(input);
    return definition ? definition.project(input) : null;
  }
}

export function createKernelConfigRegistry(
  definitions: KernelConfigDefinition[] = [
    sdkworkKernelConfigDefinition,
    hermesKernelConfigDefinition,
  ],
) {
  return new KernelConfigRegistry(definitions);
}

export const kernelConfigRegistry = createKernelConfigRegistry();

export function resolveKernelConfigDefinition(input: {
  kernelId?: string | null;
  runtimeKind?: string | null;
}) {
  return kernelConfigRegistry.resolveDefinition(input);
}

export function projectKernelConfig(input: KernelConfigProjectionInput) {
  return kernelConfigRegistry.project(input);
}
