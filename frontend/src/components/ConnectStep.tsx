import { useEffect, useRef, useState } from "react";
import { ChevronDown } from "lucide-react";
import { SourceTypePicker } from "./SourceTypePicker";
import { AiHelper } from "./AiHelper";
import { ManualFields } from "./ManualFields";
import { ErrorBanner } from "./ErrorBanner";
import { Button } from "./ui/Button";
import { buildUri, parseUri } from "../lib/connectionUri";
import type { ConnectionInfo } from "../types";

interface ConnectStepProps {
  sourceType: string;
  onSourceTypeChange: (id: string) => void;
  connection: ConnectionInfo;
  onConnectionChange: (conn: ConnectionInfo) => void;
  onConnect: (conn: ConnectionInfo) => void;
  connecting: boolean;
  error: string | null;
  onClearError: () => void;
}

export function ConnectStep({
  sourceType,
  onSourceTypeChange,
  connection,
  onConnectionChange,
  onConnect,
  connecting,
  error,
  onClearError,
}: ConnectStepProps) {
  const [uri, setUri] = useState(() => buildUri(connection));
  const [manualOpen, setManualOpen] = useState(false);
  // 用户是否手动改过 URI 框。未改过时，外部回填的 connection 会镜像进 URI 框。
  const uriDirtyRef = useRef(false);

  // 回填/外部修改 connection 时，若用户没手动编辑过 URI，则同步显示（修复回填后 URI 框为空连不上）。
  useEffect(() => {
    if (!uriDirtyRef.current) {
      setUri(buildUri(connection));
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [connection]);

  const handleUriChange = (v: string) => {
    uriDirtyRef.current = true;
    setUri(v);
    onClearError();
    const parsed = parseUri(v);
    if (parsed) onConnectionChange({ ...connection, ...parsed });
  };

  const handleManualChange = (conn: ConnectionInfo) => {
    onConnectionChange(conn);
    setUri(buildUri(conn)); // 手动改 → 连接串框同步反映
    uriDirtyRef.current = false; // 手动字段成为权威，URI 回到镜像态
    onClearError();
  };

  // 当前 URI 是否可解析 —— 决定能否连接（避免用无效串配合旧 connection 误连）。
  const parsed = uri.trim() ? parseUri(uri) : null;
  const canConnect = !connecting && !!parsed;

  const handleConnect = () => {
    if (!parsed) return;
    onConnect({ ...connection, ...parsed });
  };

  return (
    <div>
      <div className="db-card__head">
        <h1 className="db-card__title">连接数据源</h1>
        <p className="db-card__subtitle">把数据库同步到多维表格</p>
      </div>

      <ErrorBanner message={error} onClose={onClearError} />

      <div className="db-field">
        <label className="db-label">数据源类型</label>
        <SourceTypePicker value={sourceType} onChange={onSourceTypeChange} />
      </div>

      <div className="db-field">
        <label className="db-label" htmlFor="db-uri">
          连接串
        </label>
        <div className={`db-box${connecting ? " db-box--disabled" : ""}`}>
          <input
            id="db-uri"
            className="db-input"
            placeholder="postgres://用户:密码@主机:5432/库名"
            value={uri}
            spellCheck={false}
            autoComplete="off"
            disabled={connecting}
            onChange={(e) => handleUriChange(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && canConnect) handleConnect();
            }}
          />
        </div>

        <AiHelper />

        <Button
          className="db-manual__toggle"
          aria-expanded={manualOpen}
          disabled={connecting}
          onClick={() => setManualOpen((o) => !o)}
        >
          手动填写连接信息
          <ChevronDown />
        </Button>
        <div className={`db-manual${manualOpen ? " is-open" : ""}`}>
          <div className="db-manual__rows">
            <ManualFields
              connection={connection}
              onChange={handleManualChange}
            />
          </div>
        </div>
      </div>

      <div className="db-actions">
        <Button
          className="db-btn db-btn--primary"
          disabled={!canConnect}
          onClick={handleConnect}
        >
          {connecting && <span className="db-spinner" />}
          {connecting ? "连接中" : "连接"}
        </Button>
      </div>
    </div>
  );
}
