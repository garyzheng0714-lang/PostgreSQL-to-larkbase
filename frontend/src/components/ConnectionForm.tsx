import { Button, Collapsible, Input, InputNumber, RadioGroup, Radio, Select, Spin, TextArea, Typography } from "@douyinfe/semi-ui";
import { useState, useEffect } from "react";
import { listDatabases, testConnection } from "../api/helper";
import { ErrorBanner } from "./ErrorBanner";
import { DEFAULT_HOST, DEFAULT_PORT, DEFAULT_USERNAME } from "../hooks/useConfig";
import type { ConnectionInfo, SslMode } from "../types";

const { Text } = Typography;

/** Resolve empty fields to defaults before connecting */
function resolveDefaults(conn: ConnectionInfo): ConnectionInfo {
  return {
    ...conn,
    host: conn.host || DEFAULT_HOST,
    port: conn.port || DEFAULT_PORT,
    username: conn.username || DEFAULT_USERNAME,
  };
}

interface ConnectionFormProps {
  connection: ConnectionInfo;
  onChange: (conn: ConnectionInfo) => void;
  selectedDatabases: string[];
  onSelectedDatabasesChange: (dbs: string[]) => void;
  showAdvanced: boolean;
  onShowAdvancedChange: (v: boolean) => void;
  onNext: () => void;
}

/** Build a postgres:// URI from connection fields (uses defaults for display) */
function buildUri(conn: ConnectionInfo): string {
  const host = conn.host || DEFAULT_HOST;
  const port = conn.port || DEFAULT_PORT;
  const username = conn.username || DEFAULT_USERNAME;
  const userPart = conn.password
    ? `${username}:${conn.password}`
    : username;
  const dbPart = conn.database ? `/${conn.database}` : "";
  return `postgres://${userPart}@${host}:${port}${dbPart}`;
}

/** Parse a postgres:// URI into connection fields. Returns null if invalid. */
function parseUri(uri: string): Partial<ConnectionInfo> | null {
  const trimmed = uri.trim();
  // Accept postgres:// or postgresql://
  const match = trimmed.match(
    /^postgres(?:ql)?:\/\/(?:([^:@]+)(?::([^@]*))?@)?([^/:]+)(?::(\d+))?(\/([^?]*))?/
  );
  if (!match) return null;
  const [, user, pass, host, port, , db] = match;
  return {
    username: user ?? "postgres",
    password: pass ?? "",
    host: host,
    port: port ? Number(port) : 5432,
    database: db ?? "",
  };
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
  const [uri, setUri] = useState(() => buildUri(connection));

  // Sync URI when connection fields change from manual input
  const [manualEditing, setManualEditing] = useState(false);
  useEffect(() => {
    if (manualEditing) {
      setUri(buildUri(connection));
      setManualEditing(false);
    }
  }, [connection, manualEditing]);

  const resetState = () => {
    setConnected(false);
    setDatabases([]);
    setServerInfo(null);
    onSelectedDatabasesChange([]);
  };

  const updateField = (field: keyof ConnectionInfo, value: string | number | null) => {
    onChange({ ...connection, [field]: value });
    setManualEditing(true);
    resetState();
  };

  const handleUriChange = (v: string) => {
    setUri(v);
    resetState();
    const parsed = parseUri(v);
    if (parsed) {
      onChange({ ...connection, ...parsed });
    }
  };

  const handleConnect = async () => {
    setTesting(true);
    setError(null);
    resetState();

    const resolved = resolveDefaults(connection);
    const tempConn = { ...resolved, database: "postgres" };
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

      {/* URI input — primary entry */}
      <div style={{ display: "flex", gap: 8 }}>
        <Input
          placeholder="postgres://user:password@host:5432/dbname"
          value={uri}
          onChange={handleUriChange}
          onEnterPress={handleConnect}
          style={{
            flex: 1,
            fontFamily: "var(--semi-font-family-code, monospace)",
            fontSize: 13,
          }}
        />
        <Button
          theme={connected ? "light" : "solid"}
          type={connected ? "tertiary" : "primary"}
          loading={testing}
          onClick={handleConnect}
          style={{
            minWidth: 80,
            ...(connected
              ? { color: "var(--semi-color-success)", borderColor: "var(--semi-color-success)" }
              : {}),
          }}
        >
          {connected ? "✓ 已连接" : "连接"}
        </Button>
      </div>

      {/* Server info line */}
      {connected && serverInfo && (
        <Text
          type="tertiary"
          size="small"
          style={{ display: "block", marginTop: 8, letterSpacing: "0.02em" }}
        >
          {[
            serverInfo.version,
            serverInfo.size ? `🐘 ${serverInfo.size}` : "",
            serverInfo.tableCount > 0 ? `${serverInfo.tableCount} 张表` : "",
          ]
            .filter(Boolean)
            .join("   ")}
        </Text>
      )}

      {/* Database selector */}
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

      {/* Manual input toggle */}
      <div
        style={{ marginTop: 16, cursor: "pointer", display: "inline-block" }}
        onClick={() => onShowAdvancedChange(!showAdvanced)}
      >
        <Text type="tertiary" size="small" style={{ userSelect: "none" }}>
          {showAdvanced ? "收起 ↑" : "▽ 手动输入"}
        </Text>
      </div>

      <Collapsible isOpen={showAdvanced}>
        <div style={{ paddingTop: 12, display: "flex", flexDirection: "column", gap: 12 }}>
          {/* Host + Port */}
          <div style={{ display: "grid", gridTemplateColumns: "1fr 100px", gap: 12 }}>
            <div>
              <Text size="small" type="tertiary" style={{ display: "block", marginBottom: 2 }}>
                主机地址
              </Text>
              <Input
                placeholder={DEFAULT_HOST}
                value={connection.host}
                onChange={(v) => updateField("host", v)}
              />
            </div>
            <div>
              <Text size="small" type="tertiary" style={{ display: "block", marginBottom: 2 }}>
                端口
              </Text>
              <InputNumber
                value={connection.port}
                onChange={(v) => updateField("port", typeof v === "number" ? v : 5432)}
                min={1}
                max={65535}
                style={{ width: "100%" }}
              />
            </div>
          </div>

          {/* Username + Password */}
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
            <div>
              <Text size="small" type="tertiary" style={{ display: "block", marginBottom: 2 }}>
                用户名
              </Text>
              <Input
                placeholder={DEFAULT_USERNAME}
                value={connection.username}
                onChange={(v) => updateField("username", v)}
              />
            </div>
            <div>
              <Text size="small" type="tertiary" style={{ display: "block", marginBottom: 2 }}>
                密码
              </Text>
              <Input
                mode="password"
                placeholder="可留空"
                value={connection.password}
                onChange={(v) => updateField("password", v)}
              />
            </div>
          </div>

          {/* SSL */}
          <div>
            <Text size="small" type="tertiary" style={{ display: "block", marginBottom: 2 }}>
              SSL
            </Text>
            <RadioGroup
              value={connection.ssl_mode ?? "disable"}
              onChange={(e) => {
                onChange({ ...connection, ssl_mode: e.target.value as SslMode });
                resetState();
              }}
            >
              <Radio value="disable">关闭</Radio>
              <Radio value="require">加密</Radio>
              <Radio value="verify-full">证书验证</Radio>
            </RadioGroup>
          </div>

          {connection.ssl_mode === "verify-full" && (
            <div>
              <Text size="small" type="tertiary" style={{ display: "block", marginBottom: 2 }}>
                CA 证书
              </Text>
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

      {/* Next button */}
      <div
        style={{
          display: "flex",
          justifyContent: "flex-end",
          marginTop: 20,
          paddingTop: 16,
          borderTop: "1px solid var(--semi-color-border)",
        }}
      >
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
