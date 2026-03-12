import { useCallback, useState } from "react";
import type {
  ColumnInfo,
  ConnectionInfo,
  DatasourceConfig,
  NumberFormat,
  StepKey,
} from "../types";

const STEPS: StepKey[] = ["connection", "table", "fields", "confirm"];

export function useConfig() {
  const [currentStep, setCurrentStep] = useState(0);
  const [connection, setConnection] = useState<ConnectionInfo>({
    host: "",
    port: 5432,
    username: "",
    password: "",
    database: "",
  });
  const [mode, setMode] = useState<"table" | "sql">("table");
  const [schemaName, setSchemaName] = useState("public");
  const [tableName, setTableName] = useState<string | null>(null);
  const [customSQL, setCustomSQL] = useState<string>("");
  const [selectedFields, setSelectedFields] = useState<string[] | null>(
    null
  );
  const [columns, setColumns] = useState<ColumnInfo[]>([]);
  const [fieldRenames, setFieldRenames] = useState<Record<string, string>>(
    {}
  );
  const [numberFormats, setNumberFormats] = useState<
    Record<string, NumberFormat>
  >({});
  const [autoSync, setAutoSync] = useState(true);

  const stepKey = STEPS[currentStep];

  const goNext = useCallback(() => {
    setCurrentStep((s) => Math.min(s + 1, STEPS.length - 1));
  }, []);

  const goBack = useCallback(() => {
    setCurrentStep((s) => Math.max(s - 1, 0));
  }, []);

  const buildConfig = useCallback((): DatasourceConfig => {
    return {
      ...connection,
      mode,
      schema_name: schemaName,
      table_name: tableName,
      selected_fields: selectedFields,
      custom_sql: mode === "sql" ? customSQL : null,
      field_renames:
        Object.keys(fieldRenames).length > 0 ? fieldRenames : null,
      number_formats:
        Object.keys(numberFormats).length > 0 ? numberFormats : null,
      auto_sync: autoSync,
    };
  }, [
    connection,
    mode,
    schemaName,
    tableName,
    selectedFields,
    customSQL,
    fieldRenames,
    numberFormats,
    autoSync,
  ]);

  const loadFromConfig = useCallback((config: DatasourceConfig) => {
    setConnection({
      host: config.host,
      port: config.port,
      username: config.username,
      password: config.password,
      database: config.database,
    });
    setMode(config.mode);
    setSchemaName(config.schema_name);
    setTableName(config.table_name);
    setCustomSQL(config.custom_sql ?? "");
    setSelectedFields(config.selected_fields);
    setFieldRenames(config.field_renames ?? {});
    setNumberFormats(config.number_formats ?? {});
    setAutoSync(config.auto_sync);
  }, []);

  return {
    currentStep,
    stepKey,
    connection,
    setConnection,
    mode,
    setMode,
    schemaName,
    setSchemaName,
    tableName,
    setTableName,
    customSQL,
    setCustomSQL,
    selectedFields,
    setSelectedFields,
    columns,
    setColumns,
    fieldRenames,
    setFieldRenames,
    numberFormats,
    setNumberFormats,
    autoSync,
    setAutoSync,
    goNext,
    goBack,
    buildConfig,
    loadFromConfig,
  };
}
