import type { LocalApiProxyConfig } from "../types/localApiProxyTypes.ts";
import type {
  LocalApiProxyHostPort,
  LocalApiProxyValidationResult,
} from "../types/localApiProxyHost.ts";

export function createLocalApiProxyConfigService(dependencies: {
  host: Pick<LocalApiProxyHostPort, "loadConfig" | "saveConfig" | "validateConfig">;
}) {
  return {
    load() {
      return dependencies.host.loadConfig();
    },
    validate(config: LocalApiProxyConfig): Promise<LocalApiProxyValidationResult> {
      return dependencies.host.validateConfig(config);
    },
    async save(config: LocalApiProxyConfig) {
      const validation = await dependencies.host.validateConfig(config);
      if (!validation.valid) {
        throw new Error(
          `Local API proxy config validation failed: ${validation.errors.join("; ") || "unknown error"}`,
        );
      }

      return dependencies.host.saveConfig(config);
    },
  };
}
