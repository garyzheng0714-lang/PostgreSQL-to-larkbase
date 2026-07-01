import { useMemo } from "react";
import { Select } from "@douyinfe/semi-ui";
import { Pencil, Table2, Database } from "lucide-react";
import { ErrorBanner } from "./ErrorBanner";
import { Button } from "./ui/Button";
import { getSource } from "../lib/sourceTypes";
import type { ConnectionInfo, TableInfo } from "../types";

interface TableStepProps {
  sourceType: string;
  connection: ConnectionInfo;
  serverMeta: string | null;
  tables: TableInfo[];
  loadingTables: boolean;
  tablesError: string | null;
  selectedTable: string | null;
  onSelectedTableChange: (name: string) => void;
  onEditConnection: () => void;
  onSync: () => void;
  syncing: boolean;
}

export function TableStep({
  sourceType,
  connection,
  serverMeta,
  tables,
  loadingTables,
  tablesError,
  selectedTable,
  onSelectedTableChange,
  onEditConnection,
  onSync,
  syncing,
}: TableStepProps) {
  const source = getSource(sourceType);
  const byName = useMemo(() => {
    const m = new Map<string, TableInfo>();
    tables.forEach((t) => m.set(t.name, t));
    return m;
  }, [tables]);

  const optionList = useMemo(
    () => tables.map((t) => ({ value: t.name, label: t.name })),
    [tables]
  );

  return (
    <div>
      <div className="db-card__head">
        <h1 className="db-card__title">选择要同步的表</h1>
        <p className="db-card__subtitle">一次同步一张表到多维表格</p>
      </div>

      {/* 连接态横幅 */}
      <div className="db-conn">
        <span className="db-conn__dot" aria-hidden />
        <div className="db-conn__body">
          <div className="db-conn__main">
            <span className="db-conn__text" title={source.name}>
              {source.name}
            </span>
            {connection.database && (
              <span className="db-conn__database" title={connection.database}>
                {connection.database}
              </span>
            )}
          </div>
          {serverMeta && <span className="db-conn__meta">{serverMeta}</span>}
        </div>
        <Button className="db-conn__edit" onClick={onEditConnection}>
          <Pencil />
          编辑
        </Button>
      </div>

      <ErrorBanner message={tablesError} />

      {!tablesError && (
        <div className="db-field">
          <label className="db-label">数据表</label>
          <Select
            className="db-select"
            dropdownClassName="db-dropdown"
            style={{ width: "100%" }}
            placeholder={loadingTables ? "正在加载表…" : "搜索并选择一张表"}
            loading={loadingTables}
            filter
            searchPosition="dropdown"
            emptyContent={loadingTables ? "加载中…" : "没有匹配的表"}
            value={selectedTable ?? undefined}
            onChange={(v) => onSelectedTableChange(v as string)}
            optionList={optionList}
            renderOptionItem={(p: any) => {
              const info = byName.get(p.value);
              return (
                <div
                  className={`db-topt${p.selected ? " is-sel" : ""}${
                    p.focused ? " is-foc" : ""
                  }`}
                  onClick={p.onClick}
                  onMouseEnter={p.onMouseEnter}
                >
                  <Table2 className="db-topt__icon" />
                  <span className="db-topt__name">{p.label}</span>
                  {info?.estimated_rows != null && info.estimated_rows > 0 && (
                    <span className="db-topt__rows">
                      ~{info.estimated_rows.toLocaleString()} 行
                    </span>
                  )}
                  {info?.type === "view" && (
                    <span className="db-topt__tag">视图</span>
                  )}
                </div>
              );
            }}
          />
          {!loadingTables && tables.length > 0 && (
            <p className="db-tablecount">
              共 {tables.length} 张表 / 视图
            </p>
          )}
          {!loadingTables && tables.length === 0 && (
            <div className="db-empty">
              <Database />
              <span className="db-empty__text">该数据库下没有可同步的表</span>
            </div>
          )}
        </div>
      )}

      <div className="db-actions">
        <Button
          className="db-btn db-btn--primary"
          disabled={!selectedTable || syncing || loadingTables}
          onClick={onSync}
        >
          {syncing && <span className="db-spinner" />}
          {syncing ? "同步中" : "同步"}
        </Button>
      </div>
    </div>
  );
}
