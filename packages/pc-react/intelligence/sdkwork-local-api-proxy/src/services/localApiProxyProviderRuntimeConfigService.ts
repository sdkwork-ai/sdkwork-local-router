import type {
  LocalApiProxyProviderRuntimeConfig,
  LocalApiProxyRequestAuthMode,
  LocalApiProxyRequestAuthRecord,
  LocalApiProxyRequestOverridesRecord,
  LocalApiProxyRequestProxyMode,
  LocalApiProxyRequestProxyRecord,
  LocalApiProxyRequestTlsRecord,
} from "../types/localApiProxyProviderRuntimeConfig.ts";

const LOCAL_API_PROXY_REQUEST_AUTH_MODES: readonly LocalApiProxyRequestAuthMode[] = [
  "provider-default",
  "authorization-bearer",
  "header",
];

const LOCAL_API_PROXY_REQUEST_PROXY_MODES: readonly LocalApiProxyRequestProxyMode[] = [
  "env-proxy",
  "explicit-proxy",
];

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value && typeof value === "object" && !Array.isArray(value));
}

function readNonEmptyString(value: unknown) {
  return typeof value === "string" && value.trim() ? value.trim() : undefined;
}

function normalizeFiniteNumber(value: unknown, fallback: number) {
  return typeof value === "number" && Number.isFinite(value) ? value : fallback;
}

function normalizeTls(
  value: unknown,
): LocalApiProxyRequestTlsRecord | undefined {
  if (!isRecord(value)) {
    return undefined;
  }

  const nextTls: LocalApiProxyRequestTlsRecord = {};
  const ca = readNonEmptyString(value.ca);
  const cert = readNonEmptyString(value.cert);
  const key = readNonEmptyString(value.key);
  const passphrase = readNonEmptyString(value.passphrase);
  const serverName = readNonEmptyString(value.serverName);

  if (ca) {
    nextTls.ca = ca;
  }
  if (cert) {
    nextTls.cert = cert;
  }
  if (key) {
    nextTls.key = key;
  }
  if (passphrase) {
    nextTls.passphrase = passphrase;
  }
  if (serverName) {
    nextTls.serverName = serverName;
  }
  if (typeof value.insecureSkipVerify === "boolean") {
    nextTls.insecureSkipVerify = value.insecureSkipVerify;
  }

  return Object.keys(nextTls).length > 0 ? nextTls : undefined;
}

function normalizeHeaders(value: unknown) {
  if (!isRecord(value)) {
    return undefined;
  }

  const nextHeaders = Object.fromEntries(
    Object.entries(value).flatMap(([key, entry]) => {
      const normalizedKey = key.trim();
      const normalizedValue = readNonEmptyString(entry);
      return normalizedKey && normalizedValue
        ? [[normalizedKey, normalizedValue]]
        : [];
    }),
  );

  return Object.keys(nextHeaders).length > 0 ? nextHeaders : undefined;
}

function normalizeAuth(
  value: unknown,
): LocalApiProxyRequestAuthRecord | undefined {
  if (!isRecord(value)) {
    return undefined;
  }

  const mode = readNonEmptyString(value.mode) as
    | LocalApiProxyRequestAuthMode
    | undefined;
  if (!mode) {
    return undefined;
  }
  if (!LOCAL_API_PROXY_REQUEST_AUTH_MODES.includes(mode)) {
    throw new Error(
      `request overrides auth.mode must be one of ${LOCAL_API_PROXY_REQUEST_AUTH_MODES.join(", ")}.`,
    );
  }

  const nextAuth: LocalApiProxyRequestAuthRecord = { mode };
  if (mode === "authorization-bearer") {
    const token = readNonEmptyString(value.token);
    if (token) {
      nextAuth.token = token;
    }
  }
  if (mode === "header") {
    const headerName = readNonEmptyString(value.headerName);
    const headerValue = readNonEmptyString(value.value);
    const prefix = readNonEmptyString(value.prefix);

    if (headerName) {
      nextAuth.headerName = headerName;
    }
    if (headerValue) {
      nextAuth.value = headerValue;
    }
    if (prefix) {
      nextAuth.prefix = prefix;
    }
  }

  return nextAuth;
}

function normalizeProxy(
  value: unknown,
): LocalApiProxyRequestProxyRecord | undefined {
  if (!isRecord(value)) {
    return undefined;
  }

  const mode = readNonEmptyString(value.mode) as
    | LocalApiProxyRequestProxyMode
    | undefined;
  if (!mode) {
    return undefined;
  }
  if (!LOCAL_API_PROXY_REQUEST_PROXY_MODES.includes(mode)) {
    throw new Error(
      `request overrides proxy.mode must be one of ${LOCAL_API_PROXY_REQUEST_PROXY_MODES.join(", ")}.`,
    );
  }

  const nextProxy: LocalApiProxyRequestProxyRecord = { mode };
  const url = readNonEmptyString(value.url);
  if (url) {
    nextProxy.url = url;
  }
  const tls = normalizeTls(value.tls);
  if (tls) {
    nextProxy.tls = tls;
  }

  return nextProxy;
}

export function normalizeLocalApiProxyProviderRequestOverrides(
  value: LocalApiProxyRequestOverridesRecord | null | undefined,
): LocalApiProxyRequestOverridesRecord | undefined {
  const headers = normalizeHeaders(value?.headers);
  const auth = normalizeAuth(value?.auth);
  const proxy = normalizeProxy(value?.proxy);
  const tls = normalizeTls(value?.tls);

  return headers || auth || proxy || tls
    ? {
        ...(headers ? { headers } : {}),
        ...(auth ? { auth } : {}),
        ...(proxy ? { proxy } : {}),
        ...(tls ? { tls } : {}),
      }
    : undefined;
}

export function createDefaultLocalApiProxyProviderRuntimeConfig(): LocalApiProxyProviderRuntimeConfig {
  return {
    temperature: 0.2,
    topP: 1,
    maxTokens: 8192,
    timeoutMs: 60000,
    streaming: true,
  };
}

export function normalizeLocalApiProxyProviderRuntimeConfig(
  input?: Partial<LocalApiProxyProviderRuntimeConfig> | null,
): LocalApiProxyProviderRuntimeConfig {
  const defaults = createDefaultLocalApiProxyProviderRuntimeConfig();
  const request = normalizeLocalApiProxyProviderRequestOverrides(input?.request);

  return {
    temperature: normalizeFiniteNumber(input?.temperature, defaults.temperature),
    topP: normalizeFiniteNumber(input?.topP, defaults.topP),
    maxTokens: Math.max(
      1,
      Math.round(normalizeFiniteNumber(input?.maxTokens, defaults.maxTokens)),
    ),
    timeoutMs: Math.max(
      1000,
      Math.round(normalizeFiniteNumber(input?.timeoutMs, defaults.timeoutMs)),
    ),
    streaming:
      typeof input?.streaming === "boolean" ? input.streaming : defaults.streaming,
    ...(request ? { request } : {}),
  };
}
