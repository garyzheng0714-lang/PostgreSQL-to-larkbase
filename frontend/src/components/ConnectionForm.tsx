import { Button, Collapsible, Input, InputNumber, RadioGroup, Radio, Select, Spin, TextArea, Typography } from "@douyinfe/semi-ui";
import { useState } from "react";
import { listDatabases, testConnection } from "../api/helper";
import { ErrorBanner } from "./ErrorBanner";
import type { ConnectionInfo, SslMode } from "../types";

const { Text } = Typography;

interface ConnectionFormProps {
  connection: ConnectionInfo;
  onChange: (conn: ConnectionInfo) => void;
  selectedDatabases: string[];
  onSelectedDatabasesChange: (dbs: string[]) => void;
  showAdvanced: boolean;
  onShowAdvancedChange: (v: boolean) => void;
  onNext: () => void;
}

export function ConnectionForm({
  connection,
  onChange,
  selectedDatabases,
  onSelectedDatabasesChange,
  showAdvanced,
  onShowAdvancedChange,
  onNext,
}: ConnectionFormProps) {
  const [testing, setTesting] = useState(false);
  const [connected, setConnected] = useState(false);
  const [databases, setDatabases] = useState<string[]>([]);
  const [loadingDbs, setLoadingDbs] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [serverInfo, setServerInfo] = useState<{
    version: string;
    size: string;
    tableCount: number;
  } | null>(null);

  const resetState = () => {
    setConnected(false);
    setDatabases([]);
    setServerInfo(null);
    onSelectedDatabasesChange([]);
  };

  const updateField = (field: keyof ConnectionInfo, value: string | number | null) => {
    onChange({ ...connection, [field]: value });
    resetState();
  };

  const handleConnect = async () => {
    setTesting(true);
    setError(null);
    resetState();

    const tempConn = { ...connection, database: "postgres" };
    try {
      const result = await testConnection(tempConn);
      if (!result.success) {
        setError(result.message ?? "连接失败");
        setTesting(false);
        return;
      }
      setConnected(true);
      if (result.server_version || result.database_size) {
        setServerInfo({
          version: result.server_version ?? "",
          size: result.database_size ?? "",
          tableCount: result.table_count ?? 0,
        });
      }

      setLoadingDbs(true);
      const dbs = await listDatabases(tempConn);
      setDatabases(dbs.filter((db) => !["template0", "template1", "postgres"].includes(db)));
    } catch {
      setError("网络错误，无法连接后端服务");
    } finally {
      setTesting(false);
      setLoadingDbs(false);
    }
  };

  return (
    <div>
      <ErrorBanner message={error} onClose={() => setError(null)} />

      <div style={{ display: "flex", gap: 8 }}>
        <Input
          placeholder="密码（可留空）"
          mode="password"
          value={connection.password}
          onChange={(v) => updateField("password", v)}
          style={{ flex: 1 }}
          onEnterPress={handleConnect}
        />
        <Button
          theme={connected ? "light" : "solid"}
          type={connected ? "tertiary" : "primary"}
          loading={testing}
          onClick={handleConnect}
          style={{
            minWidth: 72,
            ...(connected ? { color: "var(--semi-color-success)", borderColor: "var(--semi-color-success)" } : {}),
          }}
        >
          {connected ? "已连接" : "连接"}
        </Button>
      </div>

      {connected && serverInfo && (
        <Text
          type="tertiary"
          size="small"
          style={{ display: "block", marginTop: 8, letterSpacing: "0.02em" }}
        >
          {[serverInfo.version, serverInfo.size, serverInfo.tableCount > 0 ? `${serverInfo.tableCount} 张表` : ""].filter(Boolean).join("  ·  ")}
        </Text>
      )}

      {connected && (
        <div style={{ marginTop: 16 }}>
          <Text size="small" strong style={{ display: "block", marginBottom: 4 }}>
            <span style={{ color: "var(--semi-color-danger)", marginRight: 2 }}>*</span>
            选择数据库
            {loadingDbs && <Spin size="small" style={{ marginLeft: 8 }} />}
          </Text>
          <Select
            placeholder="选择要同步的数据库"
            multiple
            value={selectedDatabases}
            onChange={(v) => onSelectedDatabasesChange(v as string[])}
            optionList={databases.map((db) => ({ value: db, label: db }))}
            style={{ width: "100%" }}
            filter
            maxTagCount={3}
          />
          {databases.length === 0 && !loadingDbs && (
            <Text type="tertiary" size="small" style={{ display: "block", marginTop: 4 }}>
              未找到可用数据库
            </Text>
          )}
        </div>
      )}

      <div
        style={{ marginTop: 16, cursor: "pointer", display: "inline-block" }}
        onClick={() => onShowAdvancedChange(!showAdvanced)}
      >
        <Text type="tertiary" size="small" style={{ userSelect: "none" }}>
          {showAdvanced ? "收起设置 ↑" : "更多设置 ↓"}
        </Text>
      </div>

      <Collapsible isOpen={showAdvanced}>
        <div style={{ paddingTop: 12, display: "flex", flexDirection: "column", gap: 12 }}>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 100px", gap: 12 }}>
            <div>
              <Text size="small" type="tertiary" style={{ display: "block", marginBottom: 2 }}>主机</Text>
              <Input value={connection.host} onChange={(v) => updateField("host", v)} />
            </div>
            <div>
              <Text size="small" type="tertiary" style={{ display: "block", marginBottom: 2 }}>端口</Text>
              <InputNumber
                value={connection.port}
                onChange={(v) => updateField("port", typeof v === "number" ? v : 5432)}
                min={1} max={65535}
                style={{ width: "100%" }}
              />
            </div>
          </div>

          <div>
            <Text size="small" type="tertiary" style={{ display: "block", marginBottom: 2 }}>用户名</Text>
            <Input value={connection.username} onChange={(v) => updateField("username", v)} />
          </div>

          <div>
            <Text size="small" type="tertiary" style={{ display: "block", marginBottom: 2 }}>SSL</Text>
            <RadioGroup
              value={connection.ssl_mode ?? "disable"}
              onChange={(e) => { onChange({ ...connection, ssl_mode: e.target.value as SslMode }); resetState(); }}
              type="button"
            >
              <Radio value="disable">关闭</Radio>
              <Radio value="require">加密</Radio>
              <Radio value="verify-full">证书验证</Radio>
            </RadioGroup>
          </div>

          {connection.ssl_mode === "verify-full" && (
            <div>
              <Text size="small" type="tertiary" style={{ display: "block", marginBottom: 2 }}>CA 证书</Text>
              <TextArea
                placeholder="-----BEGIN CERTIFICATE-----"
                value={connection.ssl_root_cert ?? ""}
                onChange={(v) => onChange({ ...connection, ssl_root_cert: v || null })}
                rows={3}
                style={{ fontFamily: "var(--semi-font-family-code, monospace)", fontSize: 12 }}
              />
            </div>
          )}
        </div>
      </Collapsible>

      <div style={{ display: "flex", justifyContent: "flex-end", marginTop: 20, paddingTop: 16, borderTop: "1px solid var(--semi-color-border)" }}>
        <Button
          theme="solid"
          onClick={onNext}
          disabled={!connected || selectedDatabases.length === 0}
        >
          下一步
        </Button>
      </div>
    </div>
  );
}
