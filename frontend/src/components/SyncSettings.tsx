import { Button, Descriptions, Spin, Switch, Typography } from "@douyinfe/semi-ui";
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
      setError("Failed to save configuration");
    } finally {
      setSaving(false);
    }
  };

  const fieldCount =
    config.selected_fields === null
      ? "All (including future)"
      : `${config.selected_fields.length} fields`;

  const renameCount = config.field_renames
    ? Object.keys(config.field_renames).length
    : 0;

  const summaryData = [
    { key: "Host", value: `${config.host}:${config.port}` },
    { key: "Database", value: config.database },
    {
      key: "Data Source",
      value:
        config.mode === "table"
          ? `${config.schema_name}.${config.table_name}`
          : "Custom SQL",
    },
    { key: "Fields", value: fieldCount },
    {
      key: "Renamed Fields",
      value: renameCount > 0 ? `${renameCount} fields` : "None",
    },
  ];

  return (
    <div>
      <ErrorBanner message={error} onClose={() => setError(null)} />

      <Descriptions data={summaryData} style={{ marginBottom: 24 }} />

      {config.mode === "sql" && config.custom_sql && (
        <div
          style={{
            background: "var(--semi-color-fill-0)",
            padding: 12,
            borderRadius: 6,
            marginBottom: 16,
            fontFamily: "monospace",
            fontSize: 12,
            maxHeight: 120,
            overflow: "auto",
            whiteSpace: "pre-wrap",
          }}
        >
          {config.custom_sql}
        </div>
      )}

      <div
        style={{
          display: "flex",
          alignItems: "center",
          marginBottom: 24,
          padding: "12px 16px",
          background: "var(--semi-color-fill-0)",
          borderRadius: 6,
        }}
      >
        <Switch
          checked={autoSync}
          onChange={onAutoSyncChange}
          style={{ marginRight: 12 }}
        />
        <div>
          <Text strong>Auto Sync</Text>
          <br />
          <Text type="tertiary" size="small">
            Automatically sync data every hour
          </Text>
        </div>
      </div>

      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          marginTop: 24,
        }}
      >
        <Button onClick={onBack} disabled={saving}>
          Back
        </Button>
        <Button
          theme="solid"
          type="primary"
          onClick={handleSave}
          disabled={saving}
          icon={saving ? <Spin size="small" /> : undefined}
        >
          {saving ? "Saving..." : "Save & Start Sync"}
        </Button>
      </div>
    </div>
  );
}
