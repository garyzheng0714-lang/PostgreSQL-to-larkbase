import { useCallback, useEffect, useRef, useState } from "react";
import type { DatasourceConfig } from "../types";

interface BitableSDK {
  getConfig: () => Promise<Record<string, string>>;
  saveConfigAndGoNext: (
    config: Record<string, string>,
  ) => Promise<void>;
  getUserId: () => Promise<string>;
  getTenantKey: () => Promise<string>;
}

export function useBitable() {
  const sdkRef = useRef<BitableSDK | null>(null);
  const [ready, setReady] = useState(false);

  useEffect(() => {
    import("@lark-base-open/connector-api")
      .then((mod) => {
        sdkRef.current = mod.bitable as unknown as BitableSDK;
        setReady(true);
      })
      .catch(() => {
        setReady(true);
      });
  }, []);

  const getExistingConfig =
    useCallback(async (): Promise<DatasourceConfig | null> => {
      if (!sdkRef.current) return null;
      try {
        const raw = await sdkRef.current.getConfig();
        if (raw?.datasourceConfig) {
          return JSON.parse(
            raw.datasourceConfig,
          ) as DatasourceConfig;
        }
      } catch {
        // No existing config saved yet
      }
      return null;
    }, []);

  const saveConfig = useCallback(
    async (config: DatasourceConfig): Promise<void> => {
      if (!sdkRef.current) {
        alert(
          "Config saved (dev mode). In Bitable this closes the window.",
        );
        return;
      }
      await sdkRef.current.saveConfigAndGoNext({
        datasourceConfig: JSON.stringify(config),
      });
    },
    [],
  );

  return { ready, getExistingConfig, saveConfig };
}
