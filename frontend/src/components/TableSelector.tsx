import { Button, Select, Spin, TabPane, Tabs } from "@douyinfe/semi-ui";
import { useCallback, useEffect, useState } from "react";
import { listColumns, listSchemas, listTables } from "../api/helper";
import { CustomSQL } from "./CustomSQL";
import { ErrorBanner } from "./ErrorBanner";
import type { ColumnInfo, ConnectionInfo, TableInfo } from "../types";

interface TableSelectorProps {
  connection: ConnectionInfo;
  mode: "table" | "sql";
  onModeChange: (mode: "table" | "sql") => void;
  schemaName: string;
  onSchemaChange: (schema: string) => void;
  tableName: string | null;
  onTableChange: (table: string | null) => void;
  customSQL: string;
  onCustomSQLChange: (sql: string) => void;
  onColumnsLoaded: (cols: ColumnInfo[]) => void;
  onNext: () => void;
  onBack: () => void;
}

export function TableSelector({
  connection,
  mode,
  onModeChange,
  schemaName,
  onSchemaChange,
  tableName,
  onTableChange,
  customSQL,
  onCustomSQLChange,
  onColumnsLoaded,
  onNext,
  onBack,
}: TableSelectorProps) {
  const [schemas, setSchemas] = useState<string[]>([]);
  const [tables, setTables] = useState<TableInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setLoading(true);
    listSchemas(connection)
      .then(setSchemas)
      .catch(() => setError("Failed to load schemas"))
      .finally(() => setLoading(false));
  }, [connection]);

  useEffect(() => {
    if (!schemaName) return;
    setLoading(true);
    listTables({ ...connection, schema_name: schemaName })
      .then(setTables)
      .catch(() => setError("Failed to load tables"))
      .finally(() => setLoading(false));
  }, [connection, schemaName]);

  const handleNext = useCallback(async () => {
    setError(null);
    if (mode === "table") {
      if (!tableName) {
        setError("Please select a table");
        return;
      }
      setLoading(true);
      try {
        const cols = await listColumns({
          ...connection,
          schema_name: schemaName,
          table_name: tableName,
        });
        onColumnsLoaded(cols);
        onNext();
      } catch {
        setError("Failed to load columns");
      } finally {
        setLoading(false);
      }
    } else {
      if (!customSQL.trim()) {
        setError("Please enter a SQL query");
        return;
      }
      onNext();
    }
  }, [
    mode,
    tableName,
    customSQL,
    connection,
    schemaName,
    onColumnsLoaded,
    onNext,
  ]);

  const schemaOptions = schemas.map((s) => ({ value: s, label: s }));
  const tableOptions = tables.map((t) => ({
    value: t.name,
    label: `${t.name} (${t.type})`,
  }));

  return (
    <div>
      <ErrorBanner message={error} onClose={() => setError(null)} />

      <Tabs
        activeKey={mode}
        onChange={(key) => onModeChange(key as "table" | "sql")}
        style={{ marginBottom: 16 }}
      >
        <TabPane tab="Table / View" itemKey="table">
          {loading && <Spin style={{ display: "block", margin: "20px auto" }} />}

          <Select
            placeholder="Select Schema"
            value={schemaName}
            onChange={(v) => onSchemaChange(v as string)}
            optionList={schemaOptions}
            style={{ width: "100%", marginBottom: 12 }}
          />

          <Select
            placeholder="Select Table or View"
            value={tableName ?? undefined}
            onChange={(v) => onTableChange(v as string)}
            optionList={tableOptions}
            filter
            style={{ width: "100%" }}
          />
        </TabPane>

        <TabPane tab="Custom SQL" itemKey="sql">
          <CustomSQL
            connection={connection}
            sql={customSQL}
            onSQLChange={onCustomSQLChange}
          />
        </TabPane>
      </Tabs>

      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          marginTop: 24,
        }}
      >
        <Button onClick={onBack}>Back</Button>
        <Button theme="solid" onClick={handleNext} loading={loading}>
          Next
        </Button>
      </div>
    </div>
  );
}
