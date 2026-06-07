export type LocalApiProxyRequestAuthMode =
  | "provider-default"
  | "authorization-bearer"
  | "header";

export interface LocalApiProxyRequestAuthRecord {
  mode: LocalApiProxyRequestAuthMode;
  token?: string;
  headerName?: string;
  value?: string;
  prefix?: string;
}

export interface LocalApiProxyRequestTlsRecord {
  ca?: string;
  cert?: string;
  key?: string;
  passphrase?: string;
  serverName?: string;
  insecureSkipVerify?: boolean;
}

export type LocalApiProxyRequestProxyMode = "env-proxy" | "explicit-proxy";

export interface LocalApiProxyRequestProxyRecord {
  mode: LocalApiProxyRequestProxyMode;
  url?: string;
  tls?: LocalApiProxyRequestTlsRecord;
}

export interface LocalApiProxyRequestOverridesRecord {
  headers?: Record<string, string>;
  auth?: LocalApiProxyRequestAuthRecord;
  proxy?: LocalApiProxyRequestProxyRecord;
  tls?: LocalApiProxyRequestTlsRecord;
}

export interface LocalApiProxyProviderRuntimeConfig {
  temperature: number;
  topP: number;
  maxTokens: number;
  timeoutMs: number;
  streaming: boolean;
  request?: LocalApiProxyRequestOverridesRecord;
}
