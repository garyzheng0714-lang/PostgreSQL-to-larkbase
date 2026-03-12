import { useEffect } from "react";
import { ConnectionForm } from "./components/ConnectionForm";
import { FieldConfig } from "./components/FieldConfig";
import { StepIndicator } from "./components/StepIndicator";
import { SyncSettings } from "./components/SyncSettings";
import { TableSelector } from "./components/TableSelector";
import { useBitable } from "./hooks/useBitable";
import { useConfig } from "./hooks/useConfig";

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
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [bitable.ready]);

  const STEP_HEIGHTS: Record<string, number> = {
    connection: 420,
    table: 500,
    fields: 580,
    confirm: 400,
  };

  useEffect(() => {
    const height = STEP_HEIGHTS[config.stepKey] ?? 520;
    bitable.resizeContainer(620, height);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [config.stepKey]);

  const handleSave = async () => {
    const finalConfig = config.buildConfig();
    await bitable.saveConfig(finalConfig);
  };

  return (
    <div style={{ maxWidth: 620, margin: "0 auto", padding: "12px 20px" }}>
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
    </div>
  );
}
