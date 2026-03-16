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
  const [loadingSchemas, setLoadingSchemas] = useState(false);
  const [loadingTables, setLoadingTables] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loading = loadingSchemas || loadingTables;

  useEffect(() => {
    let cancelled = false;
    setLoadingSchemas(true);
    listSchemas(connection)
      .then((data) => { if (!cancelled) setSchemas(data); })
      .catch(() => { if (!cancelled) setError("加载 Schema 失败"); })
      .finally(() => { if (!cancelled) setLoadingSchemas(false); });
    return () => { cancelled = true; };
  }, [connection]);

  useEffect(() => {
    if (!schemaName) return;
    let cancelled = false;
    setLoadingTables(true);
    listTables({ ...connection, schema_name: schemaName })
      .then((data) => { if (!cancelled) setTables(data); })
      .catch(() => { if (!cancelled) setError("加载表列表失败"); })
      .finally(() => { if (!cancelled) setLoadingTables(false); });
    return () => { cancelled = true; };
  }, [connection, schemaName]);

  const handleNext = useCallback(async () => {
    setError(null);
    if (mode === "table") {
      if (!tableName) {
        setError("请选择一个表");
        return;
      }
      setLoadingTables(true);
      try {
        const cols = await listColumns({
          ...connection,
          schema_name: schemaName,
          table_name: tableName,
        });
        onColumnsLoaded(cols);
        onNext();
      } catch {
        setError("加载列信息失败");
      } finally {
        setLoadingTables(false);
      }
    } else {
      if (!customSQL.trim()) {
        setError("请输入 SQL 查询语句");
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
    label: `${t.name}${t.type === "view" ? " (视图)" : ""}`,
  }));

  return (
    <div>
      <ErrorBanner message={error} onClose={() => setError(null)} />

      <Tabs
        activeKey={mode}
        onChange={(key) => onModeChange(key as "table" | "sql")}
        size="small"
        style={{ marginBottom: 12 }}
      >
        <TabPane tab="选择表 / 视图" itemKey="table">
          {loading && <Spin style={{ display: "block", margin: "16px auto" }} />}

          <div className="form-row">
            <label className="form-label">{"Schema"}</label>
            <Select
              placeholder="选择 Schema"
              value={schemaName}
              onChange={(v) => onSchemaChange(v as string)}
              optionList={schemaOptions}
              style={{ width: "100%" }}
              size="default"
            />
          </div>

          <div className="form-row">
            <label className="form-label">{"表 / 视图"}</label>
            <Select
              placeholder="选择表或视图"
              value={tableName ?? undefined}
              onChange={(v) => onTableChange(v as string)}
              optionList={tableOptions}
              filter
              style={{ width: "100%" }}
              size="default"
            />
          </div>
        </TabPane>

        <TabPane tab="自定义 SQL" itemKey="sql">
          <CustomSQL
            connection={connection}
            sql={customSQL}
            onSQLChange={onCustomSQLChange}
          />
        </TabPane>
      </Tabs>

      <div className="footer-actions">
        <Button onClick={onBack}>{"上一步"}</Button>
        <Button theme="solid" onClick={handleNext} loading={loading}>
          {"下一步"}
        </Button>
      </div>
    </div>
  );
}
