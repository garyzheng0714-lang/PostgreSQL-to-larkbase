import type { ConnectionInfo, SslMode } from "../types";

const SCHEME_RE = /^postgres(?:ql)?:\/\//i;

function encodeHost(host: string): string {
  // IPv6 字面量需用方括号包裹
  return host.includes(":") && !host.startsWith("[") ? `[${host}]` : host;
}

/** 把连接字段拼成 postgres:// URI（用于回填展示，密码存在才带；正确 URL 编码）。 */
export function buildUri(conn: Partial<ConnectionInfo>): string {
  const host = conn.host || "";
  if (!host) return "";
  const port = conn.port || 5432;
  const enc = encodeURIComponent;
  const username = conn.username ? enc(conn.username) : "";
  const auth = username
    ? conn.password
      ? `${username}:${enc(conn.password)}@`
      : `${username}@`
    : "";
  const dbPart = conn.database ? `/${enc(conn.database)}` : "";
  const sslPart =
    conn.ssl_mode && conn.ssl_mode !== "disable"
      ? `?sslmode=${conn.ssl_mode === "verify-full" ? "verify-full" : "require"}`
      : "";
  return `postgres://${auth}${encodeHost(host)}:${port}${dbPart}${sslPart}`;
}

/**
 * 解析 postgres:// 或 postgresql:// URI。无法解析返回 null。
 * 用 `new URL()`（临时换成 http scheme 借特殊 scheme 的健壮解析）：
 * 正确处理 IPv6、百分号编码、端口、query 参数。
 */
export function parseUri(uri: string): Partial<ConnectionInfo> | null {
  const trimmed = uri.trim();
  const m = trimmed.match(SCHEME_RE);
  if (!m) return null;

  let url: URL;
  try {
    url = new URL("http://" + trimmed.slice(m[0].length));
  } catch {
    return null;
  }

  const host = url.hostname.replace(/^\[|\]$/g, ""); // 去 IPv6 方括号
  if (!host) return null;

  const result: Partial<ConnectionInfo> = {
    host,
    port: url.port ? Number(url.port) : 5432,
    username: url.username ? safeDecode(url.username) : "postgres",
    password: url.password ? safeDecode(url.password) : "",
    database: url.pathname ? safeDecode(url.pathname.replace(/^\//, "")) : "",
  };

  const sslmode = url.searchParams.get("sslmode");
  if (sslmode) {
    let mode: SslMode = "require";
    if (sslmode === "disable") mode = "disable";
    else if (sslmode === "verify-full" || sslmode === "verify-ca")
      mode = "verify-full";
    result.ssl_mode = mode;
  }

  const ct = url.searchParams.get("connect_timeout");
  if (ct && /^\d+$/.test(ct)) result.connect_timeout = Number(ct);

  return result;
}

function safeDecode(s: string): string {
  try {
    return decodeURIComponent(s);
  } catch {
    return s;
  }
}
