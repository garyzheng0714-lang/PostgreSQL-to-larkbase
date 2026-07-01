import { useCallback, useEffect, useRef, useState } from "react";
import type { DatasourceConfig } from "../types";

type BitableApp =
  typeof import("@lark-base-open/connector-api")["bitable"];

export function useBitable() {
  const sdkRef = useRef<BitableApp | null>(null);
  const [ready, setReady] = useState(false);

  useEffect(() => {
    import("@lark-base-open/connector-api")
      .then((mod) => {
        sdkRef.current = mod.bitable;
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
            String(raw.datasourceConfig),
          ) as DatasourceConfig;
        }
      } catch {
        // No existing config saved yet
      }
      return null;
    }, []);

  const savingRef = useRef(false);

  const saveConfig = useCallback(
    async (config: DatasourceConfig): Promise<void> => {
      // 防并发重复提交；但失败后必须允许重试 —— 故仅在成功后保持锁，失败时释放。
      if (savingRef.current) return;
      savingRef.current = true;
      try {
        if (!sdkRef.current) return;
        await sdkRef.current.saveConfigAndGoNext({
          datasourceConfig: JSON.stringify(config),
        });
        // 成功：保持锁，避免 SDK 跳转后重复保存
      } catch (e) {
        savingRef.current = false; // 失败：释放锁，允许重试
        throw e;
      }
    },
    [],
  );

  const resizeContainer = useCallback(
    async (width: number, height: number): Promise<void> => {
      if (!sdkRef.current) return;
      try {
        await sdkRef.current.ui.setHostContainerDetailSize({
          width,
          height,
        });
      } catch {
        // Resize not supported in some environments
      }
    },
    [],
  );

  return { ready, getExistingConfig, saveConfig, resizeContainer };
}
