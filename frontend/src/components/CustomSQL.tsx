import { Button, Table, TextArea, Typography } from "@douyinfe/semi-ui";
import { useState } from "react";
import { previewSQL } from "../api/helper";
import { ErrorBanner } from "./ErrorBanner";
import type { ConnectionInfo, SQLPreviewResult } from "../types";

const { Text } = Typography;

interface CustomSQLProps {
  connection: ConnectionInfo;
  sql: string;
  onSQLChange: (sql: string) => void;
}

export function CustomSQL({ connection, sql, onSQLChange }: CustomSQLProps) {
  const [previewing, setPreviewing] = useState(false);
  const [preview, setPreview] = useState<SQLPreviewResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handlePreview = async () => {
    if (!sql.trim()) return;
    setPreviewing(true);
    setError(null);
    setPreview(null);
    try {
      const result = await previewSQL({ ...connection, sql });
      if (result.success && result.data) {
        setPreview(result.data);
      } else {
        setError(result.message ?? "SQL execution failed");
      }
    } catch {
      setError("Failed to execute SQL preview");
    } finally {
      setPreviewing(false);
    }
  };

  const previewColumns = preview?.columns.map((col) => ({
    title: col.name,
    dataIndex: col.name,
    key: col.name,
    width: 150,
    render: (v: unknown) => (v === null ? <Text type="quaternary">NULL</Text> : String(v)),
  }));

  return (
    <div>
      <ErrorBanner message={error} onClose={() => setError(null)} />

      <TextArea
        value={sql}
        onChange={(v) => onSQLChange(v)}
        placeholder="SELECT * FROM your_table WHERE ..."
        autosize={{ minRows: 4, maxRows: 10 }}
        style={{ fontFamily: "monospace", marginBottom: 12 }}
      />

      <Button
        onClick={handlePreview}
        loading={previewing}
        disabled={!sql.trim()}
        style={{ marginBottom: 16 }}
      >
        Preview (max 10 rows)
      </Button>

      {preview && previewColumns && (
        <Table
          columns={previewColumns}
          dataSource={preview.rows.map((r, i) => ({ ...r, _key: i }))}
          rowKey="_key"
          size="small"
          pagination={false}
          scroll={{ x: "max-content" }}
          style={{ marginTop: 8 }}
        />
      )}
    </div>
  );
}
