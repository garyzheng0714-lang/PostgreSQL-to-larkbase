import { Button, Collapsible, Input, InputNumber, RadioGroup, Radio, Select, Spin, TextArea, Typography } from "@douyinfe/semi-ui";
import { IconLink, IconLock, IconSetting, IconTick } from "@douyinfe/semi-icons";
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
  const [showSsl, setShowSsl] = useState(false);

  const updateField = (field: keyof ConnectionInfo, value: string | number | null) => {
    onChange({ ...connection, [field]: value });
    setConnected(false);
    setDatabases([]);
    onSelectedDatabasesChange([]);
  };

  const handleConnect = async () => {
    setTesting(true);
    setError(null);
    setConnected(false);

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
      const filtered = dbs.filter(
        (db) => !["template0", "template1", "postgres"].includes(db)
      );
      setDatabases(filtered);
    } catch {
      setError("网络错误，无法连接后端服务");
    } finally {
      setTesting(false);
      setLoadingDbs(false);
    }
  };

  const canProceed = connected && selectedDatabases.length > 0;

  return (
    <div>
      <ErrorBanner message={error} onClose={() => setError(null)} />

      <div className="form-row">
        <label className="form-label">
          {"密码"}
        </label>
        <div style={{ display: "flex", gap: 8 }}>
          <Input
            placeholder="输入密码（trust 认证可留空）"
            mode="password"
            value={connection.password}
            onChange={(v) => updateField("password", v)}
            style={{ flex: 1 }}
            size="default"
            onEnterPress={handleConnect}
          />
          <Button
            icon={testing ? <Spin size="small" /> : connected ? <IconTick style={{ color: "var(--semi-color-success)" }} /> : <IconLink />}
            onClick={handleConnect}
            disabled={testing}
          >
            {connected ? "已连接" : "连接"}
          </Button>
        </div>
      </div>

      {connected && serverInfo && (
        <div
          style={{
            display: "flex",
            gap: 16,
            padding: "8px 12px",
            background: "var(--semi-color-fill-0)",
            borderRadius: 6,
            marginTop: 8,
            fontSize: 12,
            color: "var(--semi-color-text-2)",
          }}
        >
          {serverInfo.version && <span>{serverInfo.version}</span>}
          {serverInfo.size && <span>{"📦 "}{serverInfo.size}</span>}
          {serverInfo.tableCount > 0 && <span>{serverInfo.tableCount}{" 张表"}</span>}
        </div>
      )}

      {connected && (
        <div className="form-row">
          <label className="form-label">
            <span className="required">*</span>
            {"选择数据库"}
            {loadingDbs && <Spin size="small" style={{ marginLeft: 8 }} />}
          </label>
          <Select
            placeholder="选择要同步的数据库（可多选）"
            multiple
            value={selectedDatabases}
            onChange={(v) => onSelectedDatabasesChange(v as string[])}
            optionList={databases.map((db) => ({ value: db, label: db }))}
            style={{ width: "100%" }}
            size="default"
            filter
            maxTagCount={3}
          />
          {databases.length === 0 && !loadingDbs && (
            <Text type="tertiary" style={{ fontSize: 12, marginTop: 4, display: "block" }}>
              {"未找到可用数据库"}
            </Text>
          )}
        </div>
      )}

      <div
        style={{ marginTop: 8, cursor: "pointer", display: "inline-flex", alignItems: "center", gap: 4 }}
        onClick={() => setShowSsl(!showSsl)}
      >
        <IconLock size="small" style={{ color: "var(--semi-color-text-2)" }} />
        <Text type="tertiary" style={{ fontSize: 12 }}>
          {showSsl ? "收起 SSL 设置" : "SSL 设置"}
        </Text>
      </div>

      <Collapsible isOpen={showSsl}>
        <div style={{ padding: "12px 0 0" }}>
          <div className="form-row">
            <label className="form-label">SSL 模式</label>
            <RadioGroup
              value={connection.ssl_mode ?? "disable"}
              onChange={(e) => {
                onChange({ ...connection, ssl_mode: e.target.value as SslMode });
                setConnected(false);
              }}
              direction="vertical"
            >
              <Radio value="disable">
                <span>关闭 SSL</span>
                <Text type="tertiary" style={{ fontSize: 11, marginLeft: 8 }}>内网数据库使用</Text>
              </Radio>
              <Radio value="require">
                <span>SSL 加密</span>
                <Text type="tertiary" style={{ fontSize: 11, marginLeft: 8 }}>推荐，适用于大多数云数据库</Text>
              </Radio>
              <Radio value="verify-full">
                <span>SSL + 证书验证</span>
                <Text type="tertiary" style={{ fontSize: 11, marginLeft: 8 }}>需要 CA 证书，最安全</Text>
              </Radio>
            </RadioGroup>
          </div>
          {connection.ssl_mode === "verify-full" && (
            <>
              <div className="form-row">
                <label className="form-label">CA 证书</label>
                <TextArea
                  placeholder="-----BEGIN CERTIFICATE-----"
                  value={connection.ssl_root_cert ?? ""}
                  onChange={(v) => onChange({ ...connection, ssl_root_cert: v || null })}
                  rows={3}
                  style={{ fontFamily: "monospace", fontSize: 12 }}
                />
              </div>
              <div className="form-row">
                <label className="form-label">客户端证书（可选）</label>
                <TextArea
                  placeholder="-----BEGIN CERTIFICATE-----"
                  value={connection.ssl_cert ?? ""}
                  onChange={(v) => onChange({ ...connection, ssl_cert: v || null })}
                  rows={2}
                  style={{ fontFamily: "monospace", fontSize: 12 }}
                />
              </div>
              <div className="form-row">
                <label className="form-label">客户端密钥（可选）</label>
                <TextArea
                  placeholder="-----BEGIN PRIVATE KEY-----"
                  value={connection.ssl_key ?? ""}
                  onChange={(v) => onChange({ ...connection, ssl_key: v || null })}
                  rows={2}
                  style={{ fontFamily: "monospace", fontSize: 12 }}
                />
              </div>
            </>
          )}
        </div>
      </Collapsible>

      <div
        style={{ marginTop: 8, cursor: "pointer", display: "inline-flex", alignItems: "center", gap: 4 }}
        onClick={() => onShowAdvancedChange(!showAdvanced)}
      >
        <IconSetting size="small" style={{ color: "var(--semi-color-text-2)" }} />
        <Text type="tertiary" style={{ fontSize: 12 }}>
          {showAdvanced ? "收起高级设置" : "高级设置"}
        </Text>
      </div>

      <Collapsible isOpen={showAdvanced}>
        <div style={{ padding: "12px 0 0" }}>
          <div className="form-row-inline">
            <div className="form-row">
              <label className="form-label">{"主机地址"}</label>
              <Input
                value={connection.host}
                onChange={(v) => updateField("host", v)}
                size="default"
              />
            </div>
            <div className="form-row">
              <label className="form-label">{"端口"}</label>
              <InputNumber
                value={connection.port}
                onChange={(v) => updateField("port", typeof v === "number" ? v : 5432)}
                min={1}
                max={65535}
                style={{ width: "100%" }}
                size="default"
              />
            </div>
          </div>
          <div className="form-row">
            <label className="form-label">{"用户名"}</label>
            <Input
              value={connection.username}
              onChange={(v) => updateField("username", v)}
              size="default"
            />
          </div>
          <div className="form-row-inline">
            <div className="form-row">
              <label className="form-label">{"连接超时 (秒)"}</label>
              <InputNumber
                value={connection.connect_timeout ?? undefined}
                placeholder="默认 5"
                onChange={(v) => onChange({ ...connection, connect_timeout: typeof v === "number" ? v : null })}
                min={1}
                max={60}
                style={{ width: "100%" }}
                size="default"
              />
            </div>
            <div className="form-row">
              <label className="form-label">{"查询超时 (秒)"}</label>
              <InputNumber
                value={connection.query_timeout ?? undefined}
                placeholder="默认 15"
                onChange={(v) => onChange({ ...connection, query_timeout: typeof v === "number" ? v : null })}
                min={1}
                max={300}
                style={{ width: "100%" }}
                size="default"
              />
            </div>
          </div>
        </div>
      </Collapsible>

      <div className="footer-actions">
        <div />
        <Button
          theme="solid"
          onClick={onNext}
          disabled={!canProceed}
        >
          {"下一步"}
        </Button>
      </div>
    </div>
  );
}
