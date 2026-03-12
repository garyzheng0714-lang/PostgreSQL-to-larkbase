import { Button, Input, InputNumber, Spin } from "@douyinfe/semi-ui";
import { IconLink, IconTick } from "@douyinfe/semi-icons";
import { useState } from "react";
import { testConnection } from "../api/helper";
import { ErrorBanner } from "./ErrorBanner";
import type { ConnectionInfo } from "../types";

interface ConnectionFormProps {
  connection: ConnectionInfo;
  onChange: (conn: ConnectionInfo) => void;
  onNext: () => void;
}

export function ConnectionForm({
  connection,
  onChange,
  onNext,
}: ConnectionFormProps) {
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<
    "success" | "error" | null
  >(null);
  const [error, setError] = useState<string | null>(null);

  const handleTest = async () => {
    setTesting(true);
    setError(null);
    setTestResult(null);
    try {
      const result = await testConnection(connection);
      if (result.success) {
        setTestResult("success");
      } else {
        setTestResult("error");
        setError(result.message ?? "连接失败");
      }
    } catch {
      setTestResult("error");
      setError("网络错误，无法连接后端服务");
    } finally {
      setTesting(false);
    }
  };

  const handleNext = () => {
    if (testResult === "success") {
      onNext();
    } else {
      handleTest().then(() => {});
    }
  };

  const updateField = (
    field: keyof ConnectionInfo,
    value: string | number,
  ) => {
    onChange({ ...connection, [field]: value });
    setTestResult(null);
  };

  const isFormComplete =
    connection.host &&
    connection.port &&
    connection.username &&
    connection.password &&
    connection.database;

  return (
    <div>
      <ErrorBanner message={error} onClose={() => setError(null)} />

      <div className="form-row-inline">
        <div className="form-row">
          <label className="form-label">
            <span className="required">*</span>
            {"主机地址"}
          </label>
          <Input
            placeholder="例如 db.example.com"
            value={connection.host}
            onChange={(v) => updateField("host", v)}
            size="default"
          />
        </div>
        <div className="form-row">
          <label className="form-label">
            <span className="required">*</span>
            {"端口"}
          </label>
          <InputNumber
            value={connection.port}
            onChange={(v) => updateField("port", v as number)}
            min={1}
            max={65535}
            style={{ width: "100%" }}
            size="default"
          />
        </div>
      </div>

      <div className="form-row">
        <label className="form-label">
          <span className="required">*</span>
          {"用户名"}
        </label>
        <Input
          placeholder="建议使用只读用户"
          value={connection.username}
          onChange={(v) => updateField("username", v)}
          size="default"
        />
      </div>

      <div className="form-row">
        <label className="form-label">
          <span className="required">*</span>
          {"密码"}
        </label>
        <Input
          placeholder="输入密码"
          mode="password"
          value={connection.password}
          onChange={(v) => updateField("password", v)}
          size="default"
        />
      </div>

      <div className="form-row">
        <label className="form-label">
          <span className="required">*</span>
          {"数据库名"}
        </label>
        <Input
          placeholder="输入数据库名称"
          value={connection.database}
          onChange={(v) => updateField("database", v)}
          size="default"
        />
      </div>

      <div className="footer-actions">
        <Button
          icon={testing ? <Spin size="small" /> : <IconLink />}
          onClick={handleTest}
          disabled={!isFormComplete || testing}
        >
          {testResult === "success" ? (
            <>
              {"已连接"}
              <IconTick
                style={{
                  marginLeft: 4,
                  color: "var(--semi-color-success)",
                }}
              />
            </>
          ) : (
            "测试连接"
          )}
        </Button>

        <Button
          theme="solid"
          onClick={handleNext}
          disabled={!isFormComplete || testing}
        >
          {"下一步"}
        </Button>
      </div>
    </div>
  );
}
