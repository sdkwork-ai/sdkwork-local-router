import type { LocalApiProxyRequestOverridesRecord } from "../types/localApiProxyProviderRuntimeConfig.ts";
import { parseJson5, stringifyJson5 } from "./json5Compat.ts";
import {
  normalizeLocalApiProxyProviderRequestOverrides,
} from "./localApiProxyProviderRuntimeConfigService.ts";

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value && typeof value === "object" && !Array.isArray(value));
}

export function formatLocalApiProxyProviderRequestOverridesDraft(
  request: LocalApiProxyRequestOverridesRecord | undefined,
) {
  if (!request) {
    return "";
  }

  return stringifyJson5(request, 2);
}

export function parseLocalApiProxyProviderRequestOverridesDraft(
  input: string,
): LocalApiProxyRequestOverridesRecord | undefined {
  const normalizedInput = input.trim();
  if (!normalizedInput) {
    return undefined;
  }

  const parsed = parseJson5<unknown>(normalizedInput);
  if (!isRecord(parsed)) {
    throw new Error("request overrides must be a JSON object.");
  }

  return normalizeLocalApiProxyProviderRequestOverrides(
    parsed as LocalApiProxyRequestOverridesRecord,
  );
}
