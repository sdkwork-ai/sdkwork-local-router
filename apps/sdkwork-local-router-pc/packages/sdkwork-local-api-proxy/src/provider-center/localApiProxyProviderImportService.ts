import {
  normalizeLocalApiProxyLegacyProviderId,
  normalizeLocalApiProxyProviderKey,
} from "../services/localApiProxyProviderRouteService.ts";
import { normalizeLocalApiProxyProviderRoutingDraft } from "../services/localApiProxyProviderRoutingCatalogService.ts";
import { listLocalApiProxyProviderConfigPresets } from "./localApiProxyProviderConfigPresets.ts";
import type {
  LocalApiProxyProviderConfigDraft,
  LocalApiProxyProviderConfigPreset,
} from "./localApiProxyProviderCenterTypes.ts";

export type LocalApiProxyProviderImportSource = "claude-code" | "codex" | "opencode";

export interface ImportedLocalApiProxyProviderDraft {
  source: LocalApiProxyProviderImportSource;
  sourceLabel: string;
  draft: LocalApiProxyProviderConfigDraft;
  sourcePaths: string[];
  warnings: string[];
}

export interface LocalApiProxyProviderImportResult {
  source: LocalApiProxyProviderImportSource;
  sourceLabel: string;
  drafts: ImportedLocalApiProxyProviderDraft[];
  sourcePaths: string[];
  warnings: string[];
}

interface RuntimeInfoLike {
  paths?: {
    userRoot?: string;
  } | null;
}

interface LocalApiProxyProviderImportPlatformApi {
  getPlatform(): string;
  pathExistsForUserTooling(filePath: string): Promise<boolean>;
  readFileForUserTooling(filePath: string): Promise<string>;
}

interface LocalApiProxyProviderImportRuntimeApi {
  getRuntimeInfo(): Promise<RuntimeInfoLike>;
}

interface LocalApiProxyProviderImportServiceDependencies {
  platformApi: LocalApiProxyProviderImportPlatformApi;
  runtimeApi: LocalApiProxyProviderImportRuntimeApi;
  presets: LocalApiProxyProviderConfigPreset[];
}

export interface LocalApiProxyProviderImportServiceOverrides {
  platformApi?: Partial<LocalApiProxyProviderImportPlatformApi>;
  runtimeApi?: Partial<LocalApiProxyProviderImportRuntimeApi>;
  presets?: LocalApiProxyProviderConfigPreset[];
}

interface ProviderImportContext {
  homeDir: string;
  presets: LocalApiProxyProviderConfigPreset[];
}

interface ProviderImportDraftInput {
  source: LocalApiProxyProviderImportSource;
  sourceLabel: string;
  providerId: string;
  name?: string;
  apiKey: string;
  baseUrl?: string;
  defaultModelId?: string;
  reasoningModelId?: string;
  embeddingModelId?: string;
  models?: Array<{ id: string; name: string }>;
  enabled?: boolean;
  isDefault?: boolean;
  sourcePaths: string[];
  notes?: string;
}

type JsonRecord = Record<string, unknown>;

const PROVIDER_LABELS: Record<string, string> = {
  openai: "OpenAI",
  "azure-openai": "Azure OpenAI",
  anthropic: "Anthropic",
  google: "Google",
  xai: "xAI",
  groq: "Groq",
  ollama: "Ollama",
  openrouter: "OpenRouter",
  "vercel-ai-gateway": "Vercel AI Gateway",
  litellm: "LiteLLM",
  sglang: "SGLang",
  together: "Together",
  deepseek: "DeepSeek",
  qwen: "Qwen",
  fireworks: "Fireworks",
  kilocode: "Kilo Code",
  mistral: "Mistral",
  cohere: "Cohere",
  venice: "Venice",
  vllm: "vLLM",
  "amazon-bedrock-mantle": "Amazon Bedrock Mantle",
};

function isRecord(value: unknown): value is JsonRecord {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function normalizePath(value: string) {
  return value.replaceAll("\\", "/");
}

function joinPath(...segments: string[]) {
  const normalized = segments.map((segment) => normalizePath(segment)).filter(Boolean);
  return normalized.join("/").replace(/\/{2,}/g, "/").replace(/^([A-Za-z]:)\//, "$1/");
}

function dirnamePath(filePath: string) {
  const normalized = normalizePath(filePath).replace(/\/+$/, "");
  const index = normalized.lastIndexOf("/");
  if (index <= 0) {
    return normalized;
  }

  return normalized.slice(0, index);
}

function deriveHomeDirFromRuntimeInfo(info: RuntimeInfoLike) {
  const userRoot = info.paths?.userRoot?.trim();
  if (!userRoot) {
    return null;
  }

  const normalizedUserRoot = normalizePath(userRoot);
  const sdkworkMarker = "/.sdkwork/";
  const markerIndex = normalizedUserRoot.lastIndexOf(sdkworkMarker);
  if (markerIndex > 0) {
    return normalizedUserRoot.slice(0, markerIndex);
  }

  return dirnamePath(normalizedUserRoot);
}

function inferHostOs(homeDir: string): "windows" | "macos" | "linux" {
  if (/^[A-Za-z]:\//.test(homeDir) || homeDir.startsWith("//")) {
    return "windows";
  }
  if (homeDir.startsWith("/Users/")) {
    return "macos";
  }
  return "linux";
}

function stripJsonComments(input: string) {
  let result = "";
  let inString = false;
  let stringDelimiter = "";
  let escaping = false;
  let lineComment = false;
  let blockComment = false;

  for (let index = 0; index < input.length; index += 1) {
    const current = input[index] ?? "";
    const next = input[index + 1] ?? "";

    if (lineComment) {
      if (current === "\n") {
        lineComment = false;
        result += current;
      }
      continue;
    }

    if (blockComment) {
      if (current === "*" && next === "/") {
        blockComment = false;
        index += 1;
      }
      continue;
    }

    if (inString) {
      result += current;
      if (escaping) {
        escaping = false;
      } else if (current === "\\") {
        escaping = true;
      } else if (current === stringDelimiter) {
        inString = false;
        stringDelimiter = "";
      }
      continue;
    }

    if (current === "\"" || current === "'") {
      inString = true;
      stringDelimiter = current;
      result += current;
      continue;
    }

    if (current === "/" && next === "/") {
      lineComment = true;
      index += 1;
      continue;
    }

    if (current === "/" && next === "*") {
      blockComment = true;
      index += 1;
      continue;
    }

    result += current;
  }

  return result;
}

function stripTomlComment(line: string) {
  let result = "";
  let inString = false;
  let stringDelimiter = "";
  let escaping = false;

  for (let index = 0; index < line.length; index += 1) {
    const current = line[index] ?? "";
    if (!inString && current === "#") {
      break;
    }
    result += current;

    if (escaping) {
      escaping = false;
      continue;
    }
    if (current === "\\") {
      escaping = true;
      continue;
    }
    if (inString) {
      if (current === stringDelimiter) {
        inString = false;
        stringDelimiter = "";
      }
      continue;
    }
    if (current === "\"" || current === "'") {
      inString = true;
      stringDelimiter = current;
    }
  }

  return result.trim();
}

function parseTomlValue(rawValue: string): unknown {
  const value = rawValue.trim();
  if (!value) {
    return "";
  }
  if ((value.startsWith("\"") && value.endsWith("\"")) || (value.startsWith("'") && value.endsWith("'"))) {
    return value.slice(1, -1);
  }
  if (value === "true" || value === "false") {
    return value === "true";
  }
  if (value.startsWith("[") && value.endsWith("]")) {
    return value
      .slice(1, -1)
      .split(",")
      .map((entry) => parseTomlValue(entry))
      .filter((entry) => entry !== "");
  }
  const numericValue = Number(value);
  if (Number.isFinite(numericValue)) {
    return numericValue;
  }
  return value;
}

function setTomlPath(target: JsonRecord, path: string[], value: unknown) {
  let cursor: JsonRecord = target;
  for (let index = 0; index < path.length - 1; index += 1) {
    const segment = path[index];
    if (!segment) {
      continue;
    }
    if (!isRecord(cursor[segment])) {
      cursor[segment] = {};
    }
    cursor = cursor[segment] as JsonRecord;
  }
  const lastSegment = path[path.length - 1];
  if (lastSegment) {
    cursor[lastSegment] = value;
  }
}

function parseMiniToml(input: string): JsonRecord {
  const document: JsonRecord = {};
  let currentTablePath: string[] = [];

  for (const rawLine of input.split(/\r?\n/)) {
    const line = stripTomlComment(rawLine);
    if (!line) {
      continue;
    }
    const tableMatch = line.match(/^\[([^\]]+)\]$/);
    if (tableMatch) {
      currentTablePath = tableMatch[1]
        .split(".")
        .map((segment) => segment.trim())
        .filter(Boolean);
      setTomlPath(document, currentTablePath, {});
      continue;
    }
    const assignmentIndex = line.indexOf("=");
    if (assignmentIndex <= 0) {
      continue;
    }
    const key = line.slice(0, assignmentIndex).trim();
    const rawValue = line.slice(assignmentIndex + 1);
    const path = [...currentTablePath, key];
    setTomlPath(document, path, parseTomlValue(rawValue));
  }

  return document;
}

function parseJsonDocument(input: string): JsonRecord {
  return JSON.parse(stripJsonComments(input)) as JsonRecord;
}

function readString(value: unknown) {
  return typeof value === "string" ? value.trim() : "";
}

function normalizeImportedProviderId(input: string) {
  const normalized = normalizeLocalApiProxyProviderKey(input).trim().toLowerCase();
  switch (normalized) {
    case "azure":
    case "azureopenai":
      return "azure-openai";
    case "claude":
      return "anthropic";
    case "gemini":
      return "google";
    default:
      return normalized;
  }
}

function normalizeImportedDraft(input: LocalApiProxyProviderConfigDraft) {
  return normalizeLocalApiProxyProviderRoutingDraft(input);
}

function labelForProvider(providerId: string) {
  return PROVIDER_LABELS[providerId]
    || providerId.replace(/(^\w|-\w)/g, (value) => value.replace("-", " ").toUpperCase());
}

function findPresetForProviderId(
  providerId: string,
  presets: LocalApiProxyProviderConfigPreset[],
) {
  const normalizedProviderId = normalizeImportedProviderId(providerId);
  return (
    presets.find(
      (preset) =>
        normalizeImportedProviderId(preset.draft.providerId || preset.id) === normalizedProviderId,
    ) || null
  );
}

function mergeModelRecords(
  importedModels: Array<{ id: string; name: string }> | undefined,
  presetModels: Array<{ id: string; name: string }> | undefined,
  referencedModelIds: Array<string | undefined>,
) {
  const merged = new Map<string, { id: string; name: string }>();
  const importedList = importedModels || [];
  const presetList = presetModels || [];

  for (const model of importedList) {
    const id = readString(model.id);
    if (!id) {
      continue;
    }
    merged.set(id, { id, name: readString(model.name) || id });
  }

  if (merged.size === 0) {
    for (const model of presetList) {
      const id = readString(model.id);
      if (!id || merged.has(id)) {
        continue;
      }
      merged.set(id, { id, name: readString(model.name) || id });
    }
  }

  for (const modelId of referencedModelIds) {
    const normalizedModelId = readString(modelId);
    if (!normalizedModelId || merged.has(normalizedModelId)) {
      continue;
    }
    merged.set(normalizedModelId, {
      id: normalizedModelId,
      name: normalizedModelId,
    });
  }

  return [...merged.values()];
}

function readQualifiedModelId(value: string | undefined) {
  const normalized = readString(value);
  if (!normalized) {
    return { providerId: "", modelId: "" };
  }
  const separatorIndex = normalized.indexOf("/");
  if (separatorIndex <= 0) {
    return { providerId: "", modelId: normalized };
  }
  return {
    providerId: normalizeImportedProviderId(normalized.slice(0, separatorIndex)),
    modelId: normalized.slice(separatorIndex + 1).trim(),
  };
}

function collectStringLeaves(
  input: unknown,
  path: string[] = [],
): Array<{ path: string[]; value: string }> {
  if (typeof input === "string") {
    return [{ path, value: input }];
  }
  if (Array.isArray(input)) {
    return input.flatMap((entry, index) => collectStringLeaves(entry, [...path, `${index}`]));
  }
  if (!isRecord(input)) {
    return [];
  }
  return Object.entries(input).flatMap(([key, value]) =>
    collectStringLeaves(value, [...path, key]),
  );
}

function findExactStringValue(input: unknown, targetKey: string) {
  for (const entry of collectStringLeaves(input)) {
    const lastSegment = entry.path[entry.path.length - 1];
    if (lastSegment === targetKey) {
      return entry.value.trim();
    }
  }

  return null;
}

function findCredentialByPatterns(
  input: unknown,
  patterns: RegExp[],
  contextPatterns: RegExp[] = [],
) {
  const entries = collectStringLeaves(input)
    .map((entry) => ({
      ...entry,
      key: entry.path[entry.path.length - 1] || "",
      context: entry.path.join("."),
    }))
    .filter((entry) => entry.value.trim());

  const contextScoped = contextPatterns.length > 0
    ? entries.filter((entry) => contextPatterns.some((pattern) => pattern.test(entry.context)))
    : entries;
  const candidates = contextScoped.length > 0 ? contextScoped : entries;

  return (
    candidates.find((entry) => patterns.some((pattern) => pattern.test(entry.key)))?.value.trim()
    || null
  );
}

function resolveCodexCredential(
  authDocument: unknown,
  providerKey: string,
  providerId: string,
  envKey?: string,
) {
  const normalizedEnvKey = readString(envKey);
  if (normalizedEnvKey) {
    const exactValue = findExactStringValue(authDocument, normalizedEnvKey);
    if (exactValue) {
      return exactValue;
    }
  }

  const normalizedProviderKey = normalizeImportedProviderId(providerKey);
  const normalizedProviderId = normalizeImportedProviderId(providerId);
  const providerToken = normalizedProviderId.replace(/[^a-z0-9]+/g, "_").toUpperCase();
  const providerKeyToken = normalizedProviderKey.replace(/[^a-z0-9]+/g, "_").toUpperCase();

  return (
    findCredentialByPatterns(
      authDocument,
      [
        new RegExp(`^${providerToken}_API_KEY$`, "i"),
        new RegExp(`^${providerKeyToken}_API_KEY$`, "i"),
        /^OPENAI_API_KEY$/i,
        /^api[_-]?key$/i,
        /^access[_-]?token$/i,
        /^token$/i,
        /^bearer[_-]?token$/i,
      ],
      [
        new RegExp(normalizedProviderId, "i"),
        new RegExp(normalizedProviderKey, "i"),
        new RegExp(providerToken, "i"),
      ],
    )
    || findCredentialByPatterns(authDocument, [
      new RegExp(`^${providerToken}_API_KEY$`, "i"),
      new RegExp(`^${providerKeyToken}_API_KEY$`, "i"),
      /^OPENAI_API_KEY$/i,
      /^api[_-]?key$/i,
      /^access[_-]?token$/i,
      /^token$/i,
      /^bearer[_-]?token$/i,
    ])
  );
}

function extractOpenCodeCredential(authDocument: unknown, providerId: string) {
  if (!isRecord(authDocument)) {
    return "";
  }
  const aliases = new Set([
    providerId,
    providerId === "google" ? "gemini" : providerId,
    providerId === "anthropic" ? "claude" : providerId,
  ]);

  for (const alias of aliases) {
    const entry = authDocument[alias];
    if (!isRecord(entry)) {
      continue;
    }
    if (readString(entry.type) && readString(entry.type) !== "api") {
      continue;
    }
    const keyValue = readString(entry.key || entry.apiKey || entry.token);
    if (keyValue) {
      return keyValue;
    }
  }

  return "";
}

function parseOpenCodeModels(input: unknown) {
  if (Array.isArray(input)) {
    return input
      .map((entry) => {
        if (typeof entry === "string") {
          return { id: entry.trim(), name: entry.trim() };
        }
        if (isRecord(entry)) {
          const id = readString(entry.id);
          if (!id) {
            return null;
          }
          return { id, name: readString(entry.name) || id };
        }
        return null;
      })
      .filter((entry): entry is { id: string; name: string } => Boolean(entry?.id));
  }
  if (!isRecord(input)) {
    return [];
  }
  return Object.entries(input)
    .map(([id, value]) => ({
      id: id.trim(),
      name: isRecord(value) ? readString(value.name) || id.trim() : id.trim(),
    }))
    .filter((entry) => entry.id);
}

function parseClaudeModels(input: unknown) {
  if (!Array.isArray(input)) {
    return [];
  }
  return input
    .map((entry) => {
      if (typeof entry === "string") {
        const id = entry.trim();
        return id ? { id, name: id } : null;
      }
      if (isRecord(entry)) {
        const id = readString(entry.id || entry.model);
        if (!id) {
          return null;
        }
        return { id, name: readString(entry.name) || id };
      }
      return null;
    })
    .filter((entry): entry is { id: string; name: string } => Boolean(entry?.id));
}

function createImportedDraft(
  input: ProviderImportDraftInput,
  presets: LocalApiProxyProviderConfigPreset[],
): ImportedLocalApiProxyProviderDraft {
  const normalizedProviderId = normalizeImportedProviderId(input.providerId);
  const preset = findPresetForProviderId(normalizedProviderId, presets);
  const presetDraft = preset?.draft;
  const importedModels = input.models || [];
  const defaultModelId =
    readString(input.defaultModelId) || readString(importedModels[0]?.id);
  const reasoningModelId = readString(input.reasoningModelId) || undefined;
  const embeddingModelId = readString(input.embeddingModelId) || undefined;
  const baseUrl =
    readString(input.baseUrl)
    || readString(presetDraft?.baseUrl)
    || readString(presetDraft?.upstreamBaseUrl);
  const models = mergeModelRecords(importedModels, presetDraft?.models, [
    defaultModelId,
    reasoningModelId,
    embeddingModelId,
  ]);

  const rawDraft: LocalApiProxyProviderConfigDraft = {
    presetId: preset?.id,
    name: input.name || `${input.sourceLabel} / ${preset?.label || labelForProvider(normalizedProviderId)}`,
    providerId: presetDraft?.providerId || normalizedProviderId,
    clientProtocol: presetDraft?.clientProtocol,
    upstreamProtocol: presetDraft?.upstreamProtocol,
    upstreamBaseUrl: baseUrl,
    baseUrl,
    apiKey: input.apiKey.trim(),
    enabled: input.enabled ?? true,
    isDefault: input.isDefault === true,
    managedBy: "user",
    defaultModelId: defaultModelId || "",
    reasoningModelId,
    embeddingModelId,
    models,
    notes: input.notes?.trim() || `Imported from ${input.sourceLabel}.`,
    exposeTo: ["sdkwork"],
    config: presetDraft?.config,
  };

  return {
    source: input.source,
    sourceLabel: input.sourceLabel,
    sourcePaths: input.sourcePaths,
    warnings: [],
    draft: normalizeImportedDraft(rawDraft),
  };
}

async function resolveFirstExistingPath(
  platformApi: Pick<LocalApiProxyProviderImportPlatformApi, "pathExistsForUserTooling">,
  candidates: string[],
) {
  for (const candidate of candidates) {
    if (await platformApi.pathExistsForUserTooling(candidate)) {
      return candidate;
    }
  }
  return null;
}

async function importCodexConfigs(
  dependencies: LocalApiProxyProviderImportServiceDependencies,
  context: ProviderImportContext,
): Promise<LocalApiProxyProviderImportResult> {
  const sourceLabel = "Codex";
  const configPath = joinPath(context.homeDir, ".codex", "config.toml");
  const authPath = joinPath(context.homeDir, ".codex", "auth.json");

  if (!(await dependencies.platformApi.pathExistsForUserTooling(configPath))) {
    return {
      source: "codex",
      sourceLabel,
      drafts: [],
      sourcePaths: [configPath, authPath],
      warnings: ["No Codex config.toml file was found."],
    };
  }

  const hasAuthPath = await dependencies.platformApi.pathExistsForUserTooling(authPath);
  const [configText, authText] = await Promise.all([
    dependencies.platformApi.readFileForUserTooling(configPath),
    hasAuthPath
      ? dependencies.platformApi.readFileForUserTooling(authPath)
      : Promise.resolve("{}"),
  ]);

  const configDocument = parseMiniToml(configText);
  const authDocument = parseJsonDocument(authText);
  const modelProviders: JsonRecord = isRecord(configDocument.model_providers)
    ? configDocument.model_providers
    : {};
  const configuredModelProvider = readString(configDocument.model_provider);
  const configuredModel = readString(configDocument.model);
  const qualifiedModel = readQualifiedModelId(configuredModel);
  const defaultProviderKey = configuredModelProvider || qualifiedModel.providerId || "openai";
  const defaultModelId = qualifiedModel.modelId || configuredModel;

  const providerEntries: Array<[string, JsonRecord]> = Object.entries(modelProviders)
    .filter(([, value]) => isRecord(value))
    .map(([key, value]) => [key, value as JsonRecord]);
  const effectiveEntries: Array<[string, JsonRecord]> =
    providerEntries.length > 0 ? providerEntries : [[defaultProviderKey, {}]];

  const drafts: ImportedLocalApiProxyProviderDraft[] = [];
  const warnings: string[] = [];

  for (const [providerKey, rawProviderConfig] of effectiveEntries) {
    const providerConfig = isRecord(rawProviderConfig) ? rawProviderConfig : {};
    const providerId = normalizeImportedProviderId(readString(providerConfig.name) || providerKey);
    const apiKey =
      readString(providerConfig.experimental_bearer_token)
      || resolveCodexCredential(
        authDocument,
        providerKey,
        providerId,
        readString(providerConfig.env_key),
      )
      || "";

    if (!apiKey) {
      warnings.push(`Skipped Codex provider "${providerKey}" because no API credential was found.`);
      continue;
    }

    drafts.push(
      createImportedDraft(
        {
          source: "codex",
          sourceLabel,
          providerId,
          apiKey,
          baseUrl: readString(providerConfig.base_url),
          defaultModelId:
            normalizeImportedProviderId(providerKey) === normalizeImportedProviderId(defaultProviderKey)
              ? defaultModelId
              : undefined,
          isDefault:
            normalizeImportedProviderId(providerKey) === normalizeImportedProviderId(defaultProviderKey),
          sourcePaths: [configPath, authPath],
        },
        context.presets,
      ),
    );
  }

  return {
    source: "codex",
    sourceLabel,
    drafts,
    sourcePaths: [configPath, authPath],
    warnings,
  };
}

async function importOpenCodeConfigs(
  dependencies: LocalApiProxyProviderImportServiceDependencies,
  context: ProviderImportContext,
): Promise<LocalApiProxyProviderImportResult> {
  const sourceLabel = "OpenCode";
  const hostOs = inferHostOs(context.homeDir);
  const configCandidates = hostOs === "windows"
    ? [
        joinPath(context.homeDir, "AppData", "Roaming", "opencode", "opencode.json"),
        joinPath(context.homeDir, "AppData", "Roaming", "opencode", "opencode.jsonc"),
        joinPath(context.homeDir, ".config", "opencode", "opencode.json"),
        joinPath(context.homeDir, ".config", "opencode", "opencode.jsonc"),
      ]
    : [
        joinPath(context.homeDir, ".config", "opencode", "opencode.json"),
        joinPath(context.homeDir, ".config", "opencode", "opencode.jsonc"),
      ];
  const authCandidates = hostOs === "windows"
    ? [
        joinPath(context.homeDir, "AppData", "Roaming", "opencode", "auth.json"),
        joinPath(context.homeDir, ".config", "opencode", "auth.json"),
      ]
    : hostOs === "macos"
      ? [
          joinPath(context.homeDir, "Library", "Application Support", "opencode", "auth.json"),
          joinPath(context.homeDir, ".config", "opencode", "auth.json"),
          joinPath(context.homeDir, ".local", "share", "opencode", "auth.json"),
        ]
      : [
          joinPath(context.homeDir, ".local", "share", "opencode", "auth.json"),
          joinPath(context.homeDir, ".config", "opencode", "auth.json"),
        ];

  const configPath = await resolveFirstExistingPath(dependencies.platformApi, configCandidates);
  const authPath = await resolveFirstExistingPath(dependencies.platformApi, authCandidates);

  if (!configPath) {
    return {
      source: "opencode",
      sourceLabel,
      drafts: [],
      sourcePaths: [...configCandidates, ...authCandidates],
      warnings: ["No OpenCode config file was found."],
    };
  }

  const [configText, authText] = await Promise.all([
    dependencies.platformApi.readFileForUserTooling(configPath),
    authPath
      ? dependencies.platformApi.readFileForUserTooling(authPath)
      : Promise.resolve("{}"),
  ]);

  const configDocument = parseJsonDocument(configText);
  const authDocument = parseJsonDocument(authText);
  const providerSection = isRecord(configDocument.provider)
    ? configDocument.provider
    : isRecord(configDocument.providers)
      ? configDocument.providers
      : {};
  const defaultQualifiedModel = readQualifiedModelId(readString(configDocument.model));
  const drafts: ImportedLocalApiProxyProviderDraft[] = [];
  const warnings: string[] = [];

  for (const [providerKey, rawProviderConfig] of Object.entries(providerSection)) {
    const providerConfig = isRecord(rawProviderConfig) ? rawProviderConfig : {};
    const options = isRecord(providerConfig.options) ? providerConfig.options : {};
    const providerId = normalizeImportedProviderId(providerKey);
    const apiKey =
      readString(options.apiKey || options.key) || extractOpenCodeCredential(authDocument, providerId);

    if (!apiKey) {
      warnings.push(`Skipped OpenCode provider "${providerKey}" because no API key was found.`);
      continue;
    }

    const models = parseOpenCodeModels(options.models);
    const isDefault = defaultQualifiedModel.providerId
      ? providerId === defaultQualifiedModel.providerId
      : drafts.length === 0;

    drafts.push(
      createImportedDraft(
        {
          source: "opencode",
          sourceLabel,
          providerId,
          apiKey,
          baseUrl: readString(options.baseURL || options.baseUrl),
          defaultModelId: isDefault ? defaultQualifiedModel.modelId || models[0]?.id : undefined,
          models,
          enabled: providerConfig.disabled === true ? false : true,
          isDefault,
          sourcePaths: authPath ? [configPath, authPath] : [configPath],
        },
        context.presets,
      ),
    );
  }

  return {
    source: "opencode",
    sourceLabel,
    drafts,
    sourcePaths: authPath ? [configPath, authPath] : [configPath],
    warnings,
  };
}

async function importClaudeCodeConfigs(
  dependencies: LocalApiProxyProviderImportServiceDependencies,
  context: ProviderImportContext,
): Promise<LocalApiProxyProviderImportResult> {
  const sourceLabel = "Claude Code";
  const settingsPath = joinPath(context.homeDir, ".claude", "settings.json");

  if (!(await dependencies.platformApi.pathExistsForUserTooling(settingsPath))) {
    return {
      source: "claude-code",
      sourceLabel,
      drafts: [],
      sourcePaths: [settingsPath],
      warnings: ["No Claude Code settings.json file was found."],
    };
  }

  const settingsText = await dependencies.platformApi.readFileForUserTooling(settingsPath);
  const settingsDocument = parseJsonDocument(settingsText);
  const env = isRecord(settingsDocument.env) ? settingsDocument.env : {};
  const apiKey = readString(env.ANTHROPIC_API_KEY) || readString(env.ANTHROPIC_AUTH_TOKEN);

  if (!apiKey) {
    const helperNotice = readString(settingsDocument.apiKeyHelper)
      ? "Claude Code is configured with apiKeyHelper, which cannot be imported as a static API key."
      : "No static Anthropic API key was found in Claude Code settings.";

    return {
      source: "claude-code",
      sourceLabel,
      drafts: [],
      sourcePaths: [settingsPath],
      warnings: [helperNotice],
    };
  }

  const defaultModelId = readString(settingsDocument.model);
  const models = parseClaudeModels(settingsDocument.availableModels);

  return {
    source: "claude-code",
    sourceLabel,
    drafts: [
      createImportedDraft(
        {
          source: "claude-code",
          sourceLabel,
          providerId: "anthropic",
          apiKey,
          baseUrl: readString(env.ANTHROPIC_BASE_URL),
          defaultModelId,
          models,
          isDefault: true,
          sourcePaths: [settingsPath],
        },
        context.presets,
      ),
    ],
    sourcePaths: [settingsPath],
    warnings: [],
  };
}

class LocalApiProxyProviderImportService {
  private readonly dependencies: LocalApiProxyProviderImportServiceDependencies;

  constructor(dependencies: LocalApiProxyProviderImportServiceDependencies) {
    this.dependencies = dependencies;
  }

  async importProviderConfigs(
    source: LocalApiProxyProviderImportSource,
  ): Promise<LocalApiProxyProviderImportResult> {
    if (this.dependencies.platformApi.getPlatform() !== "desktop") {
      throw new Error("Provider import is only available in the desktop runtime.");
    }

    const runtimeInfo = await this.dependencies.runtimeApi.getRuntimeInfo();
    const homeDir = deriveHomeDirFromRuntimeInfo(runtimeInfo);
    if (!homeDir) {
      throw new Error("Unable to resolve the current desktop user home directory.");
    }

    const context: ProviderImportContext = {
      homeDir,
      presets: this.dependencies.presets,
    };

    switch (source) {
      case "codex":
        return importCodexConfigs(this.dependencies, context);
      case "opencode":
        return importOpenCodeConfigs(this.dependencies, context);
      case "claude-code":
        return importClaudeCodeConfigs(this.dependencies, context);
      default:
        throw new Error(`Unsupported provider import source: ${source satisfies never}`);
    }
  }
}

function createDependencies(
  overrides: LocalApiProxyProviderImportServiceOverrides = {},
): LocalApiProxyProviderImportServiceDependencies {
  return {
    platformApi: {
      getPlatform: overrides.platformApi?.getPlatform ?? (() => "web"),
      pathExistsForUserTooling:
        overrides.platformApi?.pathExistsForUserTooling
        ?? (async () => false),
      readFileForUserTooling:
        overrides.platformApi?.readFileForUserTooling
        ?? (async () => ""),
    },
    runtimeApi: {
      getRuntimeInfo:
        overrides.runtimeApi?.getRuntimeInfo
        ?? (async () => ({})),
    },
    presets: overrides.presets ?? listLocalApiProxyProviderConfigPresets(),
  };
}

export function createLocalApiProxyProviderImportService(
  overrides: LocalApiProxyProviderImportServiceOverrides = {},
) {
  return new LocalApiProxyProviderImportService(createDependencies(overrides));
}
