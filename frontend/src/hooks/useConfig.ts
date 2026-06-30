import { useCallback, useState } from "react";
import type { ConnectionInfo, DatasourceConfig } from "../types";
import { DEFAULT_SOURCE } from "../lib/sourceTypes";

export const DEFAULT_PORT = Number(import.meta.env.VITE_DEFAULT_PORT ?? 5432);
const DEFAULT_SCHEMA = import.meta.env.VITE_DEFAULT_SCHEMA ?? "public";

const EMPTY_CONN: ConnectionInfo = {
  host: "",
  port: DEFAULT_PORT,
  username: "",
  password: "",
  database: "",
  ssl_mode: "disable",
};

export function useConfig() {
  const [sourceType, setSourceType] = useState<string>(DEFAULT_SOURCE.id);
  const [connection, setConnection] = useState<ConnectionInfo>(EMPTY_CONN);
  const [schemaName, setSchemaName] = useState<string>(DEFAULT_SCHEMA);
  const [selectedTable, setSelectedTable] = useState<string | null>(null);

  /** 协议硬事实：一份配置 = 一张同步表，故 buildConfig 只产单个 config。 */
  const buildConfig = useCallback((): DatasourceConfig | null => {
    if (!selectedTable) return null;
    // 所有可空字段显式 ?? null：connection 默认不含这些 key，undefined 会被
    // JSON.stringify 静默丢弃，导致后端逐字段反序列化时缺字段。
    return {
      host: connection.host,
      port: connection.port || DEFAULT_PORT,
      username: connection.username,
      password: connection.password,
      database: connection.database,
      mode: "table",
      schema_name: schemaName || DEFAULT_SCHEMA,
      table_name: selectedTable,
      selected_fields: null,
      custom_sql: null,
      field_renames: null,
      number_formats: null,
      auto_sync: true,
      ssl_mode: connection.ssl_mode ?? "disable",
      ssl_root_cert: connection.ssl_root_cert ?? null,
      ssl_cert: connection.ssl_cert ?? null,
      ssl_key: connection.ssl_key ?? null,
      connect_timeout: connection.connect_timeout ?? null,
      query_timeout: connection.query_timeout ?? null,
    };
  }, [connection, schemaName, selectedTable]);

  const loadFromConfig = useCallback((config: DatasourceConfig) => {
    setConnection({
      host: config.host,
      port: config.port || DEFAULT_PORT,
      username: config.username,
      password: config.password,
      database: config.database,
      ssl_mode: config.ssl_mode ?? "disable",
      ssl_root_cert: config.ssl_root_cert,
      ssl_cert: config.ssl_cert,
      ssl_key: config.ssl_key,
      connect_timeout: config.connect_timeout,
      query_timeout: config.query_timeout,
    });
    if (config.schema_name) setSchemaName(config.schema_name);
    if (config.table_name) setSelectedTable(config.table_name);
  }, []);

  const reset = useCallback(() => {
    setConnection(EMPTY_CONN);
    setSchemaName(DEFAULT_SCHEMA);
    setSelectedTable(null);
    setSourceType(DEFAULT_SOURCE.id);
  }, []);

  return {
    sourceType,
    setSourceType,
    connection,
    setConnection,
    schemaName,
    setSchemaName,
    selectedTable,
    setSelectedTable,
    buildConfig,
    loadFromConfig,
    reset,
  };
}
