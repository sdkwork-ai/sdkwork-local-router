import type { LocalApiProxyStorageConfig } from "../types/localApiProxyTypes.ts";
import {
  buildLocalApiProxySchemaStatements,
  createLocalApiProxySchemaTableNames,
} from "./localApiProxySchema.ts";

export interface LocalApiProxySqliteSchema {
  dialect: "sqlite";
  databasePath: string;
  tableNames: string[];
  statements: string[];
}

export function buildLocalApiProxySqliteSchema(
  storage: Extract<LocalApiProxyStorageConfig, { dialect: "sqlite" }>,
): LocalApiProxySqliteSchema {
  return {
    dialect: "sqlite",
    databasePath: storage.sqlitePath,
    tableNames: createLocalApiProxySchemaTableNames(),
    statements: buildLocalApiProxySchemaStatements("sqlite"),
  };
}
