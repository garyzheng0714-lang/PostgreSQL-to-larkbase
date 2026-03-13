import { useCallback, useState } from "react";
import type {
  BatchSyncItem,
  ConnectionInfo,
  DatasourceConfig,
  StepKey,
} from "../types";

const DEFAULT_HOST = import.meta.env.VITE_DEFAULT_HOST ?? "112.124.103.65";
const DEFAULT_PORT = Number(import.meta.env.VITE_DEFAULT_PORT ?? 5433);
const DEFAULT_USERNAME = import.meta.env.VITE_DEFAULT_USERNAME ?? "postgres";
const DEFAULT_SCHEMA = import.meta.env.VITE_DEFAULT_SCHEMA ?? "public";

const STEPS: StepKey[] = ["connection", "tables"];

export function useConfig() {
  const [currentStep, setCurrentStep] = useState(0);
  const [connection, setConnection] = useState<ConnectionInfo>({
    host: DEFAULT_HOST,
    port: DEFAULT_PORT,
    username: DEFAULT_USERNAME,
    password: "",
    database: "",
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
      host: connection.host,
      port: connection.port,
      username: connection.username,
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
    }));
  }, [connection, selectedTables]);

  const loadFromConfig = useCallback((config: DatasourceConfig) => {
    setConnection({
      host: config.host,
      port: config.port,
      username: config.username,
      password: config.password,
      database: config.database,
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
