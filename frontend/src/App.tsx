import { Card, Typography } from "@douyinfe/semi-ui";
import { useEffect } from "react";
import { ConnectionForm } from "./components/ConnectionForm";
import { FieldConfig } from "./components/FieldConfig";
import { StepIndicator } from "./components/StepIndicator";
import { SyncSettings } from "./components/SyncSettings";
import { TableSelector } from "./components/TableSelector";
import { useBitable } from "./hooks/useBitable";
import { useConfig } from "./hooks/useConfig";

const { Title } = Typography;

export default function App() {
  const bitable = useBitable();
  const config = useConfig();

  useEffect(() => {
    if (!bitable.ready) return;
    bitable.getExistingConfig().then((existing) => {
      if (existing) {
        config.loadFromConfig(existing);
      }
    });
    // Run when SDK becomes ready
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [bitable.ready]);

  const handleSave = async () => {
    const finalConfig = config.buildConfig();
    await bitable.saveConfig(finalConfig);
  };

  return (
    <Card
      style={{
        maxWidth: 680,
        margin: "0 auto",
        padding: "8px 0",
        minHeight: 400,
      }}
      bodyStyle={{ padding: "16px 24px" }}
    >
      <Title
        heading={5}
        style={{ marginBottom: 20, textAlign: "center" }}
      >
        PostgreSQL Data Sync
      </Title>

      <StepIndicator current={config.currentStep} />

      {config.stepKey === "connection" && (
        <ConnectionForm
          connection={config.connection}
          onChange={config.setConnection}
          onNext={config.goNext}
        />
      )}

      {config.stepKey === "table" && (
        <TableSelector
          connection={config.connection}
          mode={config.mode}
          onModeChange={config.setMode}
          schemaName={config.schemaName}
          onSchemaChange={config.setSchemaName}
          tableName={config.tableName}
          onTableChange={config.setTableName}
          customSQL={config.customSQL}
          onCustomSQLChange={config.setCustomSQL}
          onColumnsLoaded={config.setColumns}
          onNext={config.goNext}
          onBack={config.goBack}
        />
      )}

      {config.stepKey === "fields" && (
        <FieldConfig
          columns={config.columns}
          selectedFields={config.selectedFields}
          onSelectedFieldsChange={config.setSelectedFields}
          fieldRenames={config.fieldRenames}
          onFieldRenamesChange={config.setFieldRenames}
          numberFormats={config.numberFormats}
          onNumberFormatsChange={config.setNumberFormats}
          onNext={config.goNext}
          onBack={config.goBack}
        />
      )}

      {config.stepKey === "confirm" && (
        <SyncSettings
          config={config.buildConfig()}
          autoSync={config.autoSync}
          onAutoSyncChange={config.setAutoSync}
          onSave={handleSave}
          onBack={config.goBack}
        />
      )}
    </Card>
  );
}
