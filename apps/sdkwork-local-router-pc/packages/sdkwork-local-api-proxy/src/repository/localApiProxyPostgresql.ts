import {
  LOCAL_API_PROXY_DEFAULT_POSTGRES_SCHEMA,
  type LocalApiProxyStorageConfig,
} from "../types/localApiProxyTypes.ts";
import {
  buildLocalApiProxySchemaStatements,
  createLocalApiProxySchemaTableNames,
} from "./localApiProxySchema.ts";

export interface LocalApiProxyPostgresqlSchema {
  dialect: "postgresql";
  postgresUrl: string;
  schemaName: string;
  tableNames: string[];
  statements: string[];
}

export function buildLocalApiProxyPostgresqlSchema(
  storage: Extract<LocalApiProxyStorageConfig, { dialect: "postgresql" }>,
): LocalApiProxyPostgresqlSchema {
  const schemaName = storage.schema || LOCAL_API_PROXY_DEFAULT_POSTGRES_SCHEMA;

  return {
    dialect: "postgresql",
    postgresUrl: storage.postgresUrl,
    schemaName,
    tableNames: createLocalApiProxySchemaTableNames(),
    statements: buildLocalApiProxySchemaStatements("postgresql", { schemaName }),
  };
}
