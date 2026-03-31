import { useCallback, useState } from "react";
import type {
  BatchSyncItem,
  ConnectionInfo,
  DatasourceConfig,
  StepKey,
} from "../types";

export const DEFAULT_HOST = import.meta.env.VITE_DEFAULT_HOST ?? "shared-postgres";
export const DEFAULT_PORT = Number(import.meta.env.VITE_DEFAULT_PORT ?? 5432);
export const DEFAULT_USERNAME = import.meta.env.VITE_DEFAULT_USERNAME ?? "admin";
const DEFAULT_SCHEMA = import.meta.env.VITE_DEFAULT_SCHEMA ?? "public";

const STEPS: StepKey[] = ["connection", "tables"];

export function useConfig() {
  const [currentStep, setCurrentStep] = useState(0);
  const [connection, setConnection] = useState<ConnectionInfo>({
    host: "",
    port: DEFAULT_PORT,
    username: "",
    password: "",
    database: "",
    ssl_mode: "disable",
  });
  const [selectedDatabases, setSelectedDatabases] = useState<string[]>([]);
  const [selectedTables, setSelectedTables] = useState<BatchSyncItem[]>([]);
  const [showAdvanced, setShowAdvanced] = useState(false);

  const stepKey = STEPS[currentStep];

  const goNext = useCallback(() => {
    setCurrentStep((s) => Math.min(s + 1, STEPS.length - 1));
  }, []);

  const goBack = useCallback(() => {
    setCurrentStep((s) => Math.max(s - 1, 0));
  }, []);

  const buildConfigs = useCallback((): DatasourceConfig[] => {
    return selectedTables.map((item) => ({
      host: connection.host || DEFAULT_HOST,
      port: connection.port || DEFAULT_PORT,
      username: connection.username || DEFAULT_USERNAME,
      password: connection.password,
      database: item.database,
      mode: "table" as const,
      schema_name: DEFAULT_SCHEMA,
      table_name: item.tableName,
      selected_fields: null,
      custom_sql: null,
      field_renames: null,
      number_formats: null,
      auto_sync: true,
      ssl_mode: connection.ssl_mode,
      ssl_root_cert: connection.ssl_root_cert,
      ssl_cert: connection.ssl_cert,
      ssl_key: connection.ssl_key,
      connect_timeout: connection.connect_timeout,
      query_timeout: connection.query_timeout,
    }));
  }, [connection, selectedTables]);

  const loadFromConfig = useCallback((config: DatasourceConfig) => {
    setConnection({
      host: config.host,
      port: config.port,
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
    if (config.database) {
      setSelectedDatabases([config.database]);
    }
    if (config.table_name) {
      setSelectedTables([
        {
          database: config.database,
          tableName: config.table_name,
          tableType: "table",
        },
      ]);
    }
  }, []);

  return {
    currentStep,
    stepKey,
    connection,
    setConnection,
    selectedDatabases,
    setSelectedDatabases,
    selectedTables,
    setSelectedTables,
    showAdvanced,
    setShowAdvanced,
    goNext,
    goBack,
    buildConfigs,
    loadFromConfig,
  };
}
