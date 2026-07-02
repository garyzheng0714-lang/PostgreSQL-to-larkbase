import type { ConnectionInfo, SslMode } from "../types";
import { Button } from "./ui/Button";

interface ManualFieldsProps {
  connection: ConnectionInfo;
  onChange: (conn: ConnectionInfo) => void;
}

const SSL_OPTIONS: { value: SslMode; label: string }[] = [
  { value: "disable", label: "关闭" },
  { value: "require", label: "加密" },
  { value: "verify-full", label: "证书验证" },
];

/** 手动填写连接信息 —— URI-first 的渐进展开兜底，服务没有 URI 的用户。 */
export function ManualFields({ connection, onChange }: ManualFieldsProps) {
  const set = (patch: Partial<ConnectionInfo>) =>
    onChange({ ...connection, ...patch });

  return (
    <div className="db-manual__grid">
      <div className="db-row2">
        <div>
          <span className="db-sublabel">主机地址</span>
          <div className="db-box">
            <input
              className="db-input"
              style={{ fontFamily: "var(--geist)" }}
              placeholder="db.example.com"
              value={connection.host}
              onChange={(e) => set({ host: e.target.value })}
            />
          </div>
        </div>
        <div>
          <span className="db-sublabel">端口</span>
          <div className="db-box">
            <input
              className="db-input"
              style={{ fontFamily: "var(--geist)" }}
              inputMode="numeric"
              placeholder="5432"
              value={connection.port || ""}
              onChange={(e) =>
                set({ port: Number(e.target.value.replace(/\D/g, "")) || 0 })
              }
            />
          </div>
        </div>
      </div>

      <div className="db-row2 db-row2--even">
        <div>
          <span className="db-sublabel">用户名</span>
          <div className="db-box">
            <input
              className="db-input"
              style={{ fontFamily: "var(--geist)" }}
              placeholder="postgres"
              value={connection.username}
              onChange={(e) => set({ username: e.target.value })}
            />
          </div>
        </div>
        <div>
          <span className="db-sublabel">密码</span>
          <div className="db-box">
            <input
              className="db-input"
              style={{ fontFamily: "var(--geist)" }}
              type="password"
              placeholder="••••••"
              value={connection.password}
              onChange={(e) => set({ password: e.target.value })}
            />
          </div>
        </div>
      </div>

      <div>
        <span className="db-sublabel">数据库名</span>
        <div className="db-box">
          <input
            className="db-input"
            style={{ fontFamily: "var(--geist)" }}
            placeholder="analytics"
            value={connection.database}
            onChange={(e) => set({ database: e.target.value })}
          />
        </div>
      </div>

      <div>
        <span className="db-sublabel">SSL</span>
        <div className="db-seg">
          {SSL_OPTIONS.map((opt) => (
            <Button
              key={opt.value}
              className={`db-seg__item${
                (connection.ssl_mode ?? "disable") === opt.value
                  ? " is-active"
                  : ""
              }`}
              onClick={() => set({ ssl_mode: opt.value })}
            >
              {opt.label}
            </Button>
          ))}
        </div>
      </div>

      {connection.ssl_mode === "verify-full" && (
        <div>
          <span className="db-sublabel">CA 证书</span>
          <div className="db-box" style={{ minHeight: 0, padding: "8px 12px" }}>
            <textarea
              className="db-input"
              style={{ resize: "vertical", minHeight: 64, padding: 0 }}
              placeholder="-----BEGIN CERTIFICATE-----"
              value={connection.ssl_root_cert ?? ""}
              onChange={(e) => set({ ssl_root_cert: e.target.value || null })}
            />
          </div>
        </div>
      )}
    </div>
  );
}
