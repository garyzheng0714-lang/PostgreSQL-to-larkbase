import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import gsap from "gsap";
import { RotateCcw } from "lucide-react";
import { ConnectStep } from "./components/ConnectStep";
import { TableStep } from "./components/TableStep";
import { ErrorBanner } from "./components/ErrorBanner";
import { useBitable } from "./hooks/useBitable";
import { useConfig } from "./hooks/useConfig";
import { listTables, testConnection } from "./api/helper";
import type { ConnectionInfo, TableInfo } from "./types";

const FRAME_WIDTH = 620;
type Phase = "connect" | "table";

export default function App() {
  const bitable = useBitable();
  const config = useConfig();

  const [phase, setPhase] = useState<Phase>("connect");
  const [connecting, setConnecting] = useState(false);
  const [connectError, setConnectError] = useState<string | null>(null);

  const [tables, setTables] = useState<TableInfo[]>([]);
  const [loadingTables, setLoadingTables] = useState(false);
  const [tablesError, setTablesError] = useState<string | null>(null);
  const [serverMeta, setServerMeta] = useState<string | null>(null);

  const [syncing, setSyncing] = useState(false);
  const [syncError, setSyncError] = useState<string | null>(null);
  const syncingRef = useRef(false);

  const appRef = useRef<HTMLDivElement>(null);
  const cardRef = useRef<HTMLDivElement>(null);

  /* 回填已有配置（只执行一次，带 cancellation，避免慢返回覆盖用户已开始的输入） */
  const hydratedRef = useRef(false);
  useEffect(() => {
    if (!bitable.ready || hydratedRef.current) return;
    hydratedRef.current = true;
    let cancelled = false;
    bitable.getExistingConfig().then((existing) => {
      if (!cancelled && existing) config.loadFromConfig(existing);
    });
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [bitable.ready]);

  /* 动态高度：内容变化 → 通知飞书调整 iframe（覆盖展开/加载等所有变化）。
     仅在高度真正变化时调用，避免 ResizeObserver 抖动与冗余 SDK 调用。 */
  const lastHRef = useRef(0);
  useLayoutEffect(() => {
    const el = appRef.current;
    if (!el) return;
    const sync = () => {
      const raw = Math.ceil(el.getBoundingClientRect().height) + 8;
      const h = Math.max(226, Math.min(606, raw));
      if (h === lastHRef.current) return;
      lastHRef.current = h;
      bitable.resizeContainer(FRAME_WIDTH, h);
    };
    sync();
    const ro = new ResizeObserver(sync);
    ro.observe(el);
    return () => ro.disconnect();
  }, [bitable]);

  /* 动画守卫：标签页隐藏时 gsap 的 rAF 会冻结，可能把元素卡在中途透明度。
     隐藏或用户偏好减少动效时，直接跳过动画，元素留在 CSS 默认（opacity 1 可见态）。 */
  const canAnimate = () =>
    typeof document !== "undefined" &&
    !document.hidden &&
    !window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;

  /* gsap：卡片入场（clearProps 确保结束/中断后回到干净的静止态，绝不卡在低透明度） */
  useEffect(() => {
    const el = cardRef.current;
    if (!el || !canAnimate()) return;
    const tw = gsap.from(el, {
      opacity: 0,
      y: 10,
      duration: 0.45,
      ease: "power2.out",
      clearProps: "opacity,transform",
    });
    return () => {
      tw.kill();
      gsap.set(el, { clearProps: "opacity,transform" });
    };
  }, []);

  /* tab 隐藏时杀掉动画并清属性 —— 防止 gsap tween 冻结在中间透明度 */
  useEffect(() => {
    const onVis = () => {
      if (!document.hidden) return;
      const el = cardRef.current;
      if (!el) return;
      const inner = el.querySelector<HTMLElement>(".db-card__inner");
      gsap.killTweensOf(el);
      gsap.set(el, { clearProps: "opacity,transform" });
      if (inner) {
        gsap.killTweensOf(inner);
        gsap.set(inner, { clearProps: "opacity,transform" });
      }
    };
    document.addEventListener("visibilitychange", onVis);
    return () => document.removeEventListener("visibilitychange", onVis);
  }, []);

  /* gsap：相位切换时新内容淡入上滑 */
  const animatePhase = useCallback(() => {
    const inner = cardRef.current?.querySelector<HTMLElement>(".db-card__inner");
    if (!inner || !canAnimate()) return;
    gsap.fromTo(
      inner,
      { opacity: 0, y: 8 },
      {
        opacity: 1,
        y: 0,
        duration: 0.32,
        ease: "power2.out",
        overwrite: true,
        clearProps: "opacity,transform",
      }
    );
  }, []);

  const handleConnect = async (conn: ConnectionInfo) => {
    // 用传入的（与显示的 URI 一致的）connection 作为权威，并固化到全局状态。
    config.setConnection(conn);
    config.setSelectedTable(null); // 新连接：清掉上一次连接的旧表选择，杜绝表/连接错配
    setConnecting(true);
    setConnectError(null);
    setTablesError(null);
    try {
      const result = await testConnection(conn);
      if (!result.success) {
        setConnectError(result.message || "连接失败，请检查连接串");
        return;
      }
      setServerMeta(
        [result.server_version, result.table_count ? `${result.table_count} 张表` : ""]
          .filter(Boolean)
          .join(" · ")
      );

      // 进入选表相位并拉取表
      setPhase("table");
      setTables([]);
      setLoadingTables(true);
      requestAnimationFrame(animatePhase);
      try {
        const list = await listTables({
          ...conn,
          schema_name: config.schemaName,
        });
        setTables(list);
      } catch {
        setTablesError("加载数据表失败，请重试");
      } finally {
        setLoadingTables(false);
      }
    } catch {
      setConnectError("网络错误，无法连接后端服务");
    } finally {
      setConnecting(false);
    }
  };

  const handleEditConnection = () => {
    setPhase("connect");
    setTablesError(null);
    requestAnimationFrame(animatePhase);
  };

  const handleReset = () => {
    config.reset();
    setPhase("connect");
    setTables([]);
    setServerMeta(null);
    setConnectError(null);
    setTablesError(null);
    setSyncError(null);
    requestAnimationFrame(animatePhase);
  };

  const handleSync = async () => {
    if (syncingRef.current) return;
    syncingRef.current = true;
    setSyncing(true);
    setSyncError(null);
    try {
      const cfg = config.buildConfig();
      if (!cfg) {
        setSyncError("请先选择要同步的表");
        return;
      }
      await bitable.saveConfig(cfg);
    } catch {
      setSyncError("同步配置保存失败，请重试");
    } finally {
      setSyncing(false);
      syncingRef.current = false;
    }
  };

  const dirty =
    phase !== "connect" ||
    !!config.connection.host ||
    !!config.selectedTable;

  return (
    <div className="db-app" ref={appRef}>
      <div className="db-head">
        <span className="db-head__title">数据同步</span>
        <button
          type="button"
          className="db-head__reset"
          onClick={handleReset}
          disabled={!dirty || syncing || connecting}
        >
          <RotateCcw />
          重置
        </button>
      </div>

      <div className="db-card" ref={cardRef}>
        <div className="db-card__inner">
          {syncError && (
            <ErrorBanner message={syncError} onClose={() => setSyncError(null)} />
          )}

          {phase === "connect" ? (
            <ConnectStep
              sourceType={config.sourceType}
              onSourceTypeChange={config.setSourceType}
              connection={config.connection}
              onConnectionChange={config.setConnection}
              onConnect={handleConnect}
              connecting={connecting}
              error={connectError}
              onClearError={() => setConnectError(null)}
            />
          ) : (
            <TableStep
              sourceType={config.sourceType}
              connection={config.connection}
              serverMeta={serverMeta}
              tables={tables}
              loadingTables={loadingTables}
              tablesError={tablesError}
              selectedTable={config.selectedTable}
              onSelectedTableChange={config.setSelectedTable}
              onEditConnection={handleEditConnection}
              onSync={handleSync}
              syncing={syncing}
            />
          )}
        </div>
      </div>
    </div>
  );
}
