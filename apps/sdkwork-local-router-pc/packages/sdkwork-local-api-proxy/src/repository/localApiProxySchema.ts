import { LOCAL_API_PROXY_DEFAULT_POSTGRES_SCHEMA } from "../types/localApiProxyTypes.ts";

type LocalApiProxySchemaDialect = "sqlite" | "postgresql";

interface LocalApiProxySchemaTableDefinition {
  name: string;
  columns: Array<{
    name: string;
    sqlite: string;
    postgresql: string;
  }>;
  constraints?: string[];
  indexes?: Array<{
    name: string;
    columns: string;
  }>;
}

function jsonColumn(sqliteColumn = "TEXT NOT NULL", postgresqlColumn = "JSONB NOT NULL") {
  return {
    sqlite: sqliteColumn,
    postgresql: postgresqlColumn,
  };
}

const LOCAL_API_PROXY_SCHEMA_TABLES: readonly LocalApiProxySchemaTableDefinition[] = [
  {
    name: "lap_schema_migrations",
    columns: [
      { name: "version", sqlite: "TEXT PRIMARY KEY", postgresql: "TEXT PRIMARY KEY" },
      { name: "checksum", sqlite: "TEXT NOT NULL", postgresql: "TEXT NOT NULL" },
      { name: "applied_at_ms", sqlite: "INTEGER NOT NULL", postgresql: "BIGINT NOT NULL" },
    ],
  },
  {
    name: "lap_config",
    columns: [
      { name: "config_id", sqlite: "TEXT PRIMARY KEY", postgresql: "TEXT PRIMARY KEY" },
      { name: "schema_version", sqlite: "INTEGER NOT NULL", postgresql: "INTEGER NOT NULL" },
      { name: "mode", sqlite: "TEXT NOT NULL", postgresql: "TEXT NOT NULL" },
      { name: "bind_host", sqlite: "TEXT NOT NULL", postgresql: "TEXT NOT NULL" },
      { name: "bind_port", sqlite: "INTEGER NOT NULL", postgresql: "INTEGER NOT NULL" },
      { name: "public_base_url", sqlite: "TEXT NOT NULL", postgresql: "TEXT NOT NULL" },
      { name: "storage_config", ...jsonColumn() },
      { name: "capture_config", ...jsonColumn() },
      { name: "defaults_config", ...jsonColumn() },
      { name: "policies_config", ...jsonColumn() },
      { name: "runtime_config", ...jsonColumn() },
      { name: "updated_at_ms", sqlite: "INTEGER NOT NULL", postgresql: "BIGINT NOT NULL" },
    ],
  },
  {
    name: "lap_routes",
    columns: [
      { name: "route_id", sqlite: "TEXT PRIMARY KEY", postgresql: "TEXT PRIMARY KEY" },
      { name: "route_name", sqlite: "TEXT NOT NULL", postgresql: "TEXT NOT NULL" },
      { name: "enabled", sqlite: "INTEGER NOT NULL", postgresql: "BOOLEAN NOT NULL" },
      { name: "managed_by", sqlite: "TEXT NOT NULL", postgresql: "TEXT NOT NULL" },
      { name: "provider_id", sqlite: "TEXT NOT NULL", postgresql: "TEXT NOT NULL" },
      { name: "client_protocol", sqlite: "TEXT NOT NULL", postgresql: "TEXT NOT NULL" },
      { name: "upstream_protocol", sqlite: "TEXT NOT NULL", postgresql: "TEXT NOT NULL" },
      { name: "upstream_config", ...jsonColumn() },
      { name: "tags_json", ...jsonColumn() },
      { name: "notes", sqlite: "TEXT", postgresql: "TEXT" },
      { name: "updated_at_ms", sqlite: "INTEGER NOT NULL", postgresql: "BIGINT NOT NULL" },
    ],
  },
  {
    name: "lap_route_capabilities",
    columns: [
      { name: "route_id", sqlite: "TEXT NOT NULL", postgresql: "TEXT NOT NULL" },
      { name: "capability", sqlite: "TEXT NOT NULL", postgresql: "TEXT NOT NULL" },
      { name: "enabled", sqlite: "INTEGER NOT NULL", postgresql: "BOOLEAN NOT NULL" },
      { name: "operation_set", ...jsonColumn() },
      { name: "streaming", sqlite: "INTEGER NOT NULL", postgresql: "BOOLEAN NOT NULL" },
      { name: "timeout_ms", sqlite: "INTEGER", postgresql: "BIGINT" },
      { name: "path_override", sqlite: "TEXT", postgresql: "TEXT" },
      { name: "method_override", sqlite: "TEXT", postgresql: "TEXT" },
      { name: "request_policy_ref", sqlite: "TEXT", postgresql: "TEXT" },
      { name: "response_policy_ref", sqlite: "TEXT", postgresql: "TEXT" },
    ],
    constraints: ["PRIMARY KEY (route_id, capability)"],
  },
  {
    name: "lap_route_models",
    columns: [
      { name: "route_id", sqlite: "TEXT NOT NULL", postgresql: "TEXT NOT NULL" },
      { name: "role", sqlite: "TEXT NOT NULL", postgresql: "TEXT NOT NULL" },
      { name: "model_id", sqlite: "TEXT NOT NULL", postgresql: "TEXT NOT NULL" },
      { name: "capability", sqlite: "TEXT", postgresql: "TEXT" },
      { name: "label", sqlite: "TEXT", postgresql: "TEXT" },
    ],
    constraints: ["PRIMARY KEY (route_id, role, model_id)"],
  },
  {
    name: "lap_route_exposures",
    columns: [
      { name: "route_id", sqlite: "TEXT NOT NULL", postgresql: "TEXT NOT NULL" },
      { name: "target", sqlite: "TEXT NOT NULL", postgresql: "TEXT NOT NULL" },
      { name: "enabled", sqlite: "INTEGER NOT NULL", postgresql: "BOOLEAN NOT NULL" },
      { name: "consumer_id", sqlite: "TEXT", postgresql: "TEXT" },
      { name: "label", sqlite: "TEXT", postgresql: "TEXT" },
    ],
    constraints: ["PRIMARY KEY (route_id, target, consumer_id)"],
  },
  {
    name: "lap_runtime_settings",
    columns: [
      { name: "settings_id", sqlite: "TEXT PRIMARY KEY", postgresql: "TEXT PRIMARY KEY" },
      { name: "retry_count", sqlite: "INTEGER NOT NULL", postgresql: "INTEGER NOT NULL" },
      { name: "cleanup_interval_ms", sqlite: "INTEGER NOT NULL", postgresql: "BIGINT NOT NULL" },
      { name: "max_concurrent_requests", sqlite: "INTEGER NOT NULL", postgresql: "INTEGER NOT NULL" },
      { name: "startup_probe_timeout_ms", sqlite: "INTEGER NOT NULL", postgresql: "BIGINT NOT NULL" },
      { name: "updated_at_ms", sqlite: "INTEGER NOT NULL", postgresql: "BIGINT NOT NULL" },
    ],
  },
  {
    name: "lap_probe_records",
    columns: [
      { name: "probe_id", sqlite: "TEXT PRIMARY KEY", postgresql: "TEXT PRIMARY KEY" },
      { name: "route_id", sqlite: "TEXT NOT NULL", postgresql: "TEXT NOT NULL" },
      { name: "capability", sqlite: "TEXT", postgresql: "TEXT" },
      { name: "operation_id", sqlite: "TEXT", postgresql: "TEXT" },
      { name: "status", sqlite: "TEXT NOT NULL", postgresql: "TEXT NOT NULL" },
      { name: "latency_ms", sqlite: "INTEGER", postgresql: "BIGINT" },
      { name: "detail_json", ...jsonColumn("TEXT", "JSONB") },
      { name: "probed_at_ms", sqlite: "INTEGER NOT NULL", postgresql: "BIGINT NOT NULL" },
    ],
  },
  {
    name: "lap_credentials",
    columns: [
      { name: "credential_ref", sqlite: "TEXT PRIMARY KEY", postgresql: "TEXT PRIMARY KEY" },
      { name: "provider_id", sqlite: "TEXT NOT NULL", postgresql: "TEXT NOT NULL" },
      { name: "display_name", sqlite: "TEXT", postgresql: "TEXT" },
      { name: "secret_kind", sqlite: "TEXT NOT NULL", postgresql: "TEXT NOT NULL" },
      { name: "redacted_value", sqlite: "TEXT", postgresql: "TEXT" },
      { name: "metadata_json", ...jsonColumn("TEXT", "JSONB") },
      { name: "updated_at_ms", sqlite: "INTEGER NOT NULL", postgresql: "BIGINT NOT NULL" },
    ],
  },
  {
    name: "lap_request_logs",
    columns: [
      { name: "request_id", sqlite: "TEXT PRIMARY KEY", postgresql: "TEXT PRIMARY KEY" },
      { name: "trace_id", sqlite: "TEXT NOT NULL", postgresql: "TEXT NOT NULL" },
      { name: "route_id", sqlite: "TEXT NOT NULL", postgresql: "TEXT NOT NULL" },
      { name: "capability", sqlite: "TEXT NOT NULL", postgresql: "TEXT NOT NULL" },
      { name: "operation_id", sqlite: "TEXT NOT NULL", postgresql: "TEXT NOT NULL" },
      { name: "consumer", sqlite: "TEXT NOT NULL", postgresql: "TEXT NOT NULL" },
      { name: "status", sqlite: "TEXT NOT NULL", postgresql: "TEXT NOT NULL" },
      { name: "streaming", sqlite: "INTEGER NOT NULL", postgresql: "BOOLEAN NOT NULL" },
      { name: "latency_ms", sqlite: "INTEGER", postgresql: "BIGINT" },
      { name: "ttft_ms", sqlite: "INTEGER", postgresql: "BIGINT" },
      { name: "input_tokens", sqlite: "INTEGER", postgresql: "BIGINT" },
      { name: "output_tokens", sqlite: "INTEGER", postgresql: "BIGINT" },
      { name: "total_tokens", sqlite: "INTEGER", postgresql: "BIGINT" },
      { name: "request_preview", sqlite: "TEXT", postgresql: "TEXT" },
      { name: "response_preview", sqlite: "TEXT", postgresql: "TEXT" },
      { name: "error_summary", sqlite: "TEXT", postgresql: "TEXT" },
      { name: "created_at_ms", sqlite: "INTEGER NOT NULL", postgresql: "BIGINT NOT NULL" },
    ],
    indexes: [
      {
        name: "idx_lap_request_logs_route_created_at",
        columns: "route_id, created_at_ms DESC",
      },
    ],
  },
  {
    name: "lap_message_logs",
    columns: [
      { name: "message_id", sqlite: "TEXT PRIMARY KEY", postgresql: "TEXT PRIMARY KEY" },
      { name: "request_id", sqlite: "TEXT NOT NULL", postgresql: "TEXT NOT NULL" },
      { name: "role", sqlite: "TEXT NOT NULL", postgresql: "TEXT NOT NULL" },
      { name: "content_preview", sqlite: "TEXT", postgresql: "TEXT" },
      { name: "redaction_state", sqlite: "TEXT NOT NULL", postgresql: "TEXT NOT NULL" },
      { name: "created_at_ms", sqlite: "INTEGER NOT NULL", postgresql: "BIGINT NOT NULL" },
    ],
    indexes: [
      {
        name: "idx_lap_message_logs_request_created_at",
        columns: "request_id, created_at_ms DESC",
      },
    ],
  },
  {
    name: "lap_capture_settings",
    columns: [
      { name: "settings_id", sqlite: "TEXT PRIMARY KEY", postgresql: "TEXT PRIMARY KEY" },
      { name: "enabled", sqlite: "INTEGER NOT NULL", postgresql: "BOOLEAN NOT NULL" },
      { name: "store_message_content", sqlite: "INTEGER NOT NULL", postgresql: "BOOLEAN NOT NULL" },
      { name: "redact_headers_json", ...jsonColumn() },
      { name: "retention_days", sqlite: "INTEGER", postgresql: "INTEGER" },
      { name: "updated_at_ms", sqlite: "INTEGER NOT NULL", postgresql: "BIGINT NOT NULL" },
    ],
  },
  {
    name: "lap_runtime_events",
    columns: [
      { name: "event_id", sqlite: "TEXT PRIMARY KEY", postgresql: "TEXT PRIMARY KEY" },
      { name: "event_type", sqlite: "TEXT NOT NULL", postgresql: "TEXT NOT NULL" },
      { name: "route_id", sqlite: "TEXT", postgresql: "TEXT" },
      { name: "status", sqlite: "TEXT", postgresql: "TEXT" },
      { name: "detail_json", ...jsonColumn("TEXT", "JSONB") },
      { name: "created_at_ms", sqlite: "INTEGER NOT NULL", postgresql: "BIGINT NOT NULL" },
    ],
  },
] as const;

export const LOCAL_API_PROXY_SCHEMA_TABLE_NAMES = LOCAL_API_PROXY_SCHEMA_TABLES.map(
  (table) => table.name,
);

function qualifyTableName(
  dialect: LocalApiProxySchemaDialect,
  tableName: string,
  schemaName?: string,
) {
  return dialect === "postgresql" && schemaName ? `${schemaName}.${tableName}` : tableName;
}

function buildCreateTableStatement(
  table: LocalApiProxySchemaTableDefinition,
  dialect: LocalApiProxySchemaDialect,
  schemaName?: string,
) {
  const qualifiedTableName = qualifyTableName(dialect, table.name, schemaName);
  const columns = table.columns.map(
    (column) => `  ${column.name} ${dialect === "sqlite" ? column.sqlite : column.postgresql}`,
  );
  const lines = table.constraints ? [...columns, ...table.constraints.map((item) => `  ${item}`)] : columns;

  return `CREATE TABLE IF NOT EXISTS ${qualifiedTableName} (\n${lines.join(",\n")}\n);`;
}

function buildCreateIndexStatement(
  tableName: string,
  index: NonNullable<LocalApiProxySchemaTableDefinition["indexes"]>[number],
  dialect: LocalApiProxySchemaDialect,
  schemaName?: string,
) {
  const qualifiedTableName = qualifyTableName(dialect, tableName, schemaName);
  return `CREATE INDEX IF NOT EXISTS ${index.name} ON ${qualifiedTableName} (${index.columns});`;
}

export function createLocalApiProxySchemaTableNames() {
  return [...LOCAL_API_PROXY_SCHEMA_TABLE_NAMES];
}

export function buildLocalApiProxySchemaStatements(
  dialect: LocalApiProxySchemaDialect,
  options?: {
    schemaName?: string;
  },
) {
  const statements: string[] = [];
  const schemaName =
    dialect === "postgresql"
      ? options?.schemaName || LOCAL_API_PROXY_DEFAULT_POSTGRES_SCHEMA
      : undefined;

  if (dialect === "postgresql") {
    statements.push(`CREATE SCHEMA IF NOT EXISTS ${schemaName};`);
  }

  for (const table of LOCAL_API_PROXY_SCHEMA_TABLES) {
    statements.push(buildCreateTableStatement(table, dialect, schemaName));

    for (const index of table.indexes ?? []) {
      statements.push(buildCreateIndexStatement(table.name, index, dialect, schemaName));
    }
  }

  return statements;
}
