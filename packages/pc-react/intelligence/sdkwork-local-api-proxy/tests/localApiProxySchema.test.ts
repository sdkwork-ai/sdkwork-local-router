import { describe, expect, it } from "vitest";
import {
  buildLocalApiProxyPostgresqlSchema,
  buildLocalApiProxySqliteSchema,
  createLocalApiProxySchemaTableNames,
} from "../src";

describe("@sdkwork/local-api-proxy schema builders", () => {
  it("declares the canonical lap_ table set once for every dialect", () => {
    expect(createLocalApiProxySchemaTableNames()).toEqual([
      "lap_schema_migrations",
      "lap_config",
      "lap_routes",
      "lap_route_capabilities",
      "lap_route_models",
      "lap_route_exposures",
      "lap_runtime_settings",
      "lap_probe_records",
      "lap_credentials",
      "lap_request_logs",
      "lap_message_logs",
      "lap_capture_settings",
      "lap_runtime_events",
    ]);
  });

  it("builds sqlite ddl for the single-file local topology", () => {
    const schema = buildLocalApiProxySqliteSchema({
      dialect: "sqlite",
      sqlitePath: "C:/sdkwork/data/local-api-proxy.db",
    });

    const ddl = schema.statements.join("\n");

    expect(schema.databasePath).toBe("C:/sdkwork/data/local-api-proxy.db");
    expect(schema.dialect).toBe("sqlite");
    expect(schema.tableNames).toEqual(createLocalApiProxySchemaTableNames());
    expect(ddl).toContain("CREATE TABLE IF NOT EXISTS lap_config");
    expect(ddl).toContain("storage_config TEXT NOT NULL");
    expect(ddl).toContain("request_preview TEXT");
    expect(ddl).toContain("updated_at_ms INTEGER NOT NULL");
  });

  it("builds postgresql ddl for the single-schema server topology", () => {
    const schema = buildLocalApiProxyPostgresqlSchema({
      dialect: "postgresql",
      postgresUrl: "postgres://localhost:5432/local_api_proxy",
    });

    const ddl = schema.statements.join("\n");

    expect(schema.dialect).toBe("postgresql");
    expect(schema.schemaName).toBe("local_api_proxy");
    expect(schema.tableNames).toEqual(createLocalApiProxySchemaTableNames());
    expect(ddl).toContain("CREATE SCHEMA IF NOT EXISTS local_api_proxy;");
    expect(ddl).toContain("CREATE TABLE IF NOT EXISTS local_api_proxy.lap_config");
    expect(ddl).toContain("storage_config JSONB NOT NULL");
    expect(ddl).toContain("updated_at_ms BIGINT NOT NULL");
    expect(ddl).toContain("CREATE INDEX IF NOT EXISTS idx_lap_request_logs_route_created_at");
  });
});
