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

        mod.bitable.bridge
          .getTheme()
          .then((theme) => {
            applyTheme(theme === "DARK");
          })
          .catch(() => {});

        mod.bitable.bridge.onThemeChange((ev) => {
          applyTheme(ev.data.theme === "DARK");
        });
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

  const saveConfig = useCallback(
    async (config: DatasourceConfig): Promise<void> => {
      if (!sdkRef.current) {
        return;
      }
      await sdkRef.current.saveConfigAndGoNext({
        datasourceConfig: JSON.stringify(config),
      });
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

function applyTheme(isDark: boolean): void {
  if (isDark) {
    document.body.setAttribute("theme-mode", "dark");
  } else {
    document.body.removeAttribute("theme-mode");
  }
}
