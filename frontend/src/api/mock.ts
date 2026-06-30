/**
 * DEV-only helper mock —— 仅用于无后端时本地真机验证全部 UI 状态。
 * 由 VITE_MOCK=1 守卫；生产构建不启用，不污染真实接口契约。
 *   VITE_MOCK=1            正常成功 + 112 张表（验证搜索/规模）
 *   VITE_MOCK=fail         连接失败（验证错误态）
 *   VITE_MOCK=empty        连接成功但 0 表（验证空态）
 */
import type { ConnectionInfo, TableInfo } from "../types";
import type { ConnectionTestResult } from "./helper";

const envFlag = import.meta.env.VITE_MOCK as string | undefined;
// 仅在 DEV 下启用 mock：即便生产 CI 误设 VITE_MOCK，也绝不打进生产包（import.meta.env.DEV 为 false → 整段被摇树）。
export const MOCK_ENABLED =
  import.meta.env.DEV &&
  (envFlag === "1" || envFlag === "fail" || envFlag === "empty");

/** 启用 mock 后，可用 URL 参数 ?mock=fail|empty 临时覆盖，便于本地验证各状态。 */
const flag = (() => {
  if (typeof window !== "undefined") {
    const q = new URLSearchParams(window.location.search).get("mock");
    if (q === "1" || q === "fail" || q === "empty") return q;
  }
  return envFlag;
})();

function delay<T>(value: T, ms = 650): Promise<T> {
  return new Promise((resolve) => setTimeout(() => resolve(value), ms));
}

export function mockTestConnection(
  _conn: ConnectionInfo
): Promise<ConnectionTestResult> {
  if (flag === "fail") {
    return delay({
      success: false,
      message: "无法连接到主机 db.example.com:5432 —— 连接超时（请检查主机与端口）",
      server_version: "",
      database_size: "",
      table_count: 0,
    });
  }
  return delay({
    success: true,
    message: "连接成功",
    server_version: "PostgreSQL 17.9",
    database_size: "2.4 GB",
    table_count: flag === "empty" ? 0 : 112,
  });
}

const TABLE_NAMES = [
  "users", "orders", "order_items", "products", "product_variants",
  "categories", "inventory", "warehouses", "shipments", "carriers",
  "payments", "invoices", "refunds", "subscriptions", "plans",
  "customers", "addresses", "contacts", "leads", "campaigns",
  "events", "sessions", "page_views", "clicks", "conversions",
  "audit_logs", "api_keys", "webhooks", "notifications", "messages",
];

export function mockListTables(
  conn: ConnectionInfo & { schema_name: string }
): Promise<TableInfo[]> {
  if (flag === "empty") return delay([], 500);
  const tables: TableInfo[] = [];
  // 拼到 112 张，覆盖搜索/滚动规模
  for (let i = 0; i < 112; i++) {
    const base = TABLE_NAMES[i % TABLE_NAMES.length];
    const name = i < TABLE_NAMES.length ? base : `${base}_${Math.floor(i / TABLE_NAMES.length)}`;
    tables.push({
      name,
      type: i % 11 === 0 ? "view" : "table",
      estimated_rows: Math.round(Math.abs(Math.sin(i + 1)) * 50000),
    });
  }
  void conn;
  return delay(tables, 600);
}
