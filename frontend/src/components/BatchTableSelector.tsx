import { Button, Checkbox, Spin, Tag, Typography } from "@douyinfe/semi-ui";
import { useEffect, useState } from "react";
import { listTables } from "../api/helper";
import { ErrorBanner } from "./ErrorBanner";
import type { BatchSyncItem, ConnectionInfo, TableInfo } from "../types";

const { Text } = Typography;

interface DatabaseTables {
  database: string;
  tables: TableInfo[];
  loading: boolean;
  error: string | null;
}

interface BatchTableSelectorProps {
  connection: ConnectionInfo;
  selectedDatabases: string[];
  selectedTables: BatchSyncItem[];
  onSelectedTablesChange: (tables: BatchSyncItem[]) => void;
  onBack: () => void;
  onSync: () => void;
  syncing: boolean;
}

export function BatchTableSelector({
  connection,
  selectedDatabases,
  selectedTables,
  onSelectedTablesChange,
  onBack,
  onSync,
  syncing,
}: BatchTableSelectorProps) {
  const [dbTables, setDbTables] = useState<DatabaseTables[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const initial: DatabaseTables[] = selectedDatabases.map((db) => ({
      database: db,
      tables: [],
      loading: true,
      error: null,
    }));
    setDbTables(initial);

    selectedDatabases.forEach((db) => {
      const conn: ConnectionInfo = { ...connection, database: db };
      listTables({ ...conn, schema_name: "public" })
        .then((tables) => {
          setDbTables((prev) =>
            prev.map((item) =>
              item.database === db
                ? { ...item, tables, loading: false }
                : item
            )
          );
        })
        .catch(() => {
          setDbTables((prev) =>
            prev.map((item) =>
              item.database === db
                ? { ...item, loading: false, error: `加载 ${db} 的表失败` }
                : item
            )
          );
        });
    });
  }, [connection, selectedDatabases]);

  const isSelected = (db: string, tableName: string) =>
    selectedTables.some(
      (t) => t.database === db && t.tableName === tableName
    );

  const toggleTable = (db: string, table: TableInfo) => {
    const exists = isSelected(db, table.name);
    if (exists) {
      onSelectedTablesChange(
        selectedTables.filter(
          (t) => !(t.database === db && t.tableName === table.name)
        )
      );
    } else {
      onSelectedTablesChange([
        ...selectedTables,
        { database: db, tableName: table.name, tableType: table.type },
      ]);
    }
  };

  const toggleDatabase = (db: string, tables: TableInfo[]) => {
    const allSelected = tables.every((t) => isSelected(db, t.name));
    if (allSelected) {
      onSelectedTablesChange(
        selectedTables.filter((t) => t.database !== db)
      );
    } else {
      const existing = selectedTables.filter((t) => t.database !== db);
      const newItems: BatchSyncItem[] = tables.map((t) => ({
        database: db,
        tableName: t.name,
        tableType: t.type,
      }));
      onSelectedTablesChange([...existing, ...newItems]);
    }
  };

  const selectAll = () => {
    const all: BatchSyncItem[] = dbTables.flatMap((db) =>
      db.tables.map((t) => ({
        database: db.database,
        tableName: t.name,
        tableType: t.type,
      }))
    );
    onSelectedTablesChange(all);
  };

  const deselectAll = () => {
    onSelectedTablesChange([]);
  };

  const totalTables = dbTables.reduce((sum, db) => sum + db.tables.length, 0);
  const allLoaded = dbTables.every((db) => !db.loading);

  return (
    <div>
      <ErrorBanner message={error} onClose={() => setError(null)} />

      {allLoaded && totalTables > 0 && (
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            marginBottom: 12,
          }}
        >
          <Text style={{ fontSize: 13 }}>
            {`已选 ${selectedTables.length} / ${totalTables} 个表`}
          </Text>
          <div style={{ display: "flex", gap: 8 }}>
            <Text
              link={{ onClick: selectAll }}
              style={{ fontSize: 12 }}
            >
              {"全选"}
            </Text>
            <Text
              link={{ onClick: deselectAll }}
              style={{ fontSize: 12 }}
            >
              {"清空"}
            </Text>
          </div>
        </div>
      )}

      <div style={{ maxHeight: 360, overflow: "auto" }}>
        {dbTables.map((db) => (
          <div key={db.database} style={{ marginBottom: 16 }}>
            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: 8,
                marginBottom: 8,
                padding: "6px 10px",
                background: "var(--semi-color-fill-0)",
                borderRadius: 6,
              }}
            >
              <Checkbox
                checked={
                  db.tables.length > 0 &&
                  db.tables.every((t) => isSelected(db.database, t.name))
                }
                indeterminate={
                  db.tables.some((t) => isSelected(db.database, t.name)) &&
                  !db.tables.every((t) => isSelected(db.database, t.name))
                }
                onChange={() => toggleDatabase(db.database, db.tables)}
                disabled={db.loading || db.tables.length === 0}
              />
              <Text strong style={{ fontSize: 13 }}>
                {db.database}
              </Text>
              {db.loading && <Spin size="small" />}
              {!db.loading && (
                <Text type="tertiary" style={{ fontSize: 12 }}>
                  {`${db.tables.length} 个表/视图`}
                </Text>
              )}
            </div>

            {db.error && (
              <Text type="danger" style={{ fontSize: 12, paddingLeft: 32 }}>
                {db.error}
              </Text>
            )}

            {!db.loading && db.tables.length === 0 && !db.error && (
              <Text
                type="tertiary"
                style={{ fontSize: 12, paddingLeft: 32, display: "block" }}
              >
                {"该数据库下没有表或视图"}
              </Text>
            )}

            {db.tables.map((table) => (
              <div
                key={`${db.database}-${table.name}`}
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 8,
                  padding: "4px 10px 4px 32px",
                  cursor: "pointer",
                  borderRadius: 4,
                }}
                onClick={() => toggleTable(db.database, table)}
              >
                <Checkbox
                  checked={isSelected(db.database, table.name)}
                  onChange={() => toggleTable(db.database, table)}
                />
                <Text style={{ fontSize: 13, flex: 1 }}>{table.name}</Text>
                {table.type === "view" && (
                  <Tag size="small" color="blue" style={{ fontSize: 11 }}>
                    {"视图"}
                  </Tag>
                )}
              </div>
            ))}
          </div>
        ))}
      </div>

      <div className="footer-actions">
        <Button onClick={onBack} disabled={syncing}>
          {"上一步"}
        </Button>
        <Button
          theme="solid"
          type="primary"
          onClick={onSync}
          disabled={selectedTables.length === 0 || syncing}
          icon={syncing ? <Spin size="small" /> : undefined}
        >
          {syncing
            ? "同步中..."
            : `同步 ${selectedTables.length} 个表`}
        </Button>
      </div>
    </div>
  );
}
