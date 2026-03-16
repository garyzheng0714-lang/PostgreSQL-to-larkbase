import { useEffect, useRef, useState } from "react";
import { BatchTableSelector } from "./components/BatchTableSelector";
import { ConnectionForm } from "./components/ConnectionForm";
import { ErrorBanner } from "./components/ErrorBanner";
import { StepIndicator } from "./components/StepIndicator";
import { useBitable } from "./hooks/useBitable";
import { useConfig } from "./hooks/useConfig";

export default function App() {
  const bitable = useBitable();
  const config = useConfig();
  const [syncing, setSyncing] = useState(false);
  const [syncError, setSyncError] = useState<string | null>(null);
  const syncingRef = useRef(false);

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
    connection: 340,
    tables: 520,
  };

  useEffect(() => {
    const height = STEP_HEIGHTS[config.stepKey] ?? 520;
    bitable.resizeContainer(620, height);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [config.stepKey]);

  const handleSync = async () => {
    if (syncingRef.current) return;
    syncingRef.current = true;
    setSyncing(true);

    try {
      setSyncError(null);
      const configs = config.buildConfigs();
      if (configs.length === 0) return;
      await bitable.saveConfig(configs[0]);
      if (configs.length > 1) {
        setSyncError(
          `已开始同步「${configs[0].table_name}」。` +
          `飞书连接器每次只能同步 1 张表，其余 ${configs.length - 1} 张表请重新添加连接器。`
        );
      }
    } catch {
      setSyncError("同步配置保存失败，请重试");
    } finally {
      setSyncing(false);
      syncingRef.current = false;
    }
  };

  return (
    <div style={{ maxWidth: 620, margin: "0 auto", padding: "12px 20px" }}>
      <ErrorBanner message={syncError} onClose={() => setSyncError(null)} />
      <StepIndicator current={config.currentStep} />

      {config.stepKey === "connection" && (
        <ConnectionForm
          connection={config.connection}
          onChange={config.setConnection}
          selectedDatabases={config.selectedDatabases}
          onSelectedDatabasesChange={config.setSelectedDatabases}
          showAdvanced={config.showAdvanced}
          onShowAdvancedChange={config.setShowAdvanced}
          onNext={config.goNext}
        />
      )}

      {config.stepKey === "tables" && (
        <BatchTableSelector
          connection={config.connection}
          selectedDatabases={config.selectedDatabases}
          selectedTables={config.selectedTables}
          onSelectedTablesChange={config.setSelectedTables}
          onBack={config.goBack}
          onSync={handleSync}
          syncing={syncing}
        />
      )}
    </div>
  );
}
