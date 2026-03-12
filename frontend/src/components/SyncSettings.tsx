import { Button, Spin, Switch, Typography } from "@douyinfe/semi-ui";
import { useState } from "react";
import { ErrorBanner } from "./ErrorBanner";
import type { DatasourceConfig } from "../types";

const { Text } = Typography;

interface SyncSettingsProps {
  config: DatasourceConfig;
  autoSync: boolean;
  onAutoSyncChange: (v: boolean) => void;
  onSave: (config: DatasourceConfig) => Promise<void>;
  onBack: () => void;
}

export function SyncSettings({
  config,
  autoSync,
  onAutoSyncChange,
  onSave,
  onBack,
}: SyncSettingsProps) {
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSave = async () => {
    setSaving(true);
    setError(null);
    try {
      await onSave({ ...config, auto_sync: autoSync });
    } catch {
      setError("保存配置失败");
    } finally {
      setSaving(false);
    }
  };

  const fieldCount =
    config.selected_fields === null
      ? "全部（含未来新增）"
      : `${config.selected_fields.length} 个字段`;

  const renameCount = config.field_renames
    ? Object.keys(config.field_renames).length
    : 0;

  const summaryRows = [
    { label: "主机", value: `${config.host}:${config.port}` },
    { label: "数据库", value: config.database },
    {
      label: "数据源",
      value:
        config.mode === "table"
          ? `${config.schema_name}.${config.table_name}`
          : "自定义 SQL",
    },
    { label: "同步字段", value: fieldCount },
    {
      label: "重命名",
      value: renameCount > 0 ? `${renameCount} 个字段` : "无",
    },
  ];

  return (
    <div>
      <ErrorBanner message={error} onClose={() => setError(null)} />

      <div className="config-summary-card">
        {summaryRows.map((row) => (
          <div key={row.label} className="config-summary-row">
            <span className="label">{row.label}</span>
            <span className="value" title={row.value}>
              {row.value}
            </span>
          </div>
        ))}
      </div>

      {config.mode === "sql" && config.custom_sql && (
        <div
          style={{
            background: "var(--semi-color-fill-0)",
            padding: 10,
            borderRadius: 6,
            marginBottom: 14,
            fontFamily: "monospace",
            fontSize: 12,
            maxHeight: 100,
            overflow: "auto",
            whiteSpace: "pre-wrap",
            color: "var(--semi-color-text-1)",
          }}
        >
          {config.custom_sql}
        </div>
      )}

      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 10,
          padding: "10px 14px",
          background: "var(--semi-color-fill-0)",
          borderRadius: 8,
        }}
      >
        <Switch
          checked={autoSync}
          onChange={onAutoSyncChange}
          size="small"
        />
        <div>
          <Text strong style={{ fontSize: 13 }}>
            {"自动同步"}
          </Text>
          <br />
          <Text type="tertiary" style={{ fontSize: 12 }}>
            {"每小时自动同步一次数据"}
          </Text>
        </div>
      </div>

      <div className="footer-actions">
        <Button onClick={onBack} disabled={saving}>
          {"上一步"}
        </Button>
        <Button
          theme="solid"
          type="primary"
          onClick={handleSave}
          disabled={saving}
          icon={saving ? <Spin size="small" /> : undefined}
        >
          {saving ? "保存中..." : "保存并开始同步"}
        </Button>
      </div>
    </div>
  );
}
