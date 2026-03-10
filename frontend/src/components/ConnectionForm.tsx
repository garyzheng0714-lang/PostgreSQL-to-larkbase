import { Button, Input, InputNumber, Spin, Typography } from "@douyinfe/semi-ui";
import { IconLink, IconTick } from "@douyinfe/semi-icons";
import { useState } from "react";
import { testConnection } from "../api/helper";
import { ErrorBanner } from "./ErrorBanner";
import type { ConnectionInfo } from "../types";

const { Text } = Typography;

interface ConnectionFormProps {
  connection: ConnectionInfo;
  onChange: (conn: ConnectionInfo) => void;
  onNext: () => void;
}

function FieldLabel({ text, required }: { text: string; required?: boolean }) {
  return (
    <div style={{ marginBottom: 4, marginTop: 12 }}>
      {required && <Text type="danger">* </Text>}
      <Text strong>{text}</Text>
    </div>
  );
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
        setError(result.message ?? "Connection failed");
      }
    } catch {
      setTestResult("error");
      setError("Network error, cannot reach backend");
    } finally {
      setTesting(false);
    }
  };

  const handleNext = () => {
    if (testResult === "success") {
      onNext();
    } else {
      handleTest().then(() => {
        // Check handled by state
      });
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

      <div style={{ padding: "0 4px" }}>
        <FieldLabel text="Database Host" required />
        <Input
          placeholder="e.g. db.example.com (public IP or domain)"
          value={connection.host}
          onChange={(v) => updateField("host", v)}
        />

        <FieldLabel text="Port" required />
        <InputNumber
          placeholder="5432"
          value={connection.port}
          onChange={(v) => updateField("port", v as number)}
          min={1}
          max={65535}
          style={{ width: "100%" }}
        />

        <FieldLabel text="Username" required />
        <Input
          placeholder="Recommend using a read-only user"
          value={connection.username}
          onChange={(v) => updateField("username", v)}
        />

        <FieldLabel text="Password" required />
        <Input
          placeholder="Enter password"
          mode="password"
          value={connection.password}
          onChange={(v) => updateField("password", v)}
        />

        <FieldLabel text="Database" required />
        <Input
          placeholder="Database name"
          value={connection.database}
          onChange={(v) => updateField("database", v)}
        />
      </div>

      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          marginTop: 24,
        }}
      >
        <Button
          icon={testing ? <Spin size="small" /> : <IconLink />}
          onClick={handleTest}
          disabled={!isFormComplete || testing}
        >
          {testResult === "success" ? "Connected" : "Test Connection"}
          {testResult === "success" && (
            <IconTick
              style={{
                marginLeft: 4,
                color: "var(--semi-color-success)",
              }}
            />
          )}
        </Button>

        <Button
          theme="solid"
          onClick={handleNext}
          disabled={!isFormComplete || testing}
        >
          Next
        </Button>
      </div>
    </div>
  );
}
